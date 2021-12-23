use antigen_core::{
    impl_read_write_lock, AddIndirectComponent, Changed, ChangedTrait, GetIndirect,
    IndirectComponent, LazyComponent, ReadWriteLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
    Usage,
};
use legion::{world::SubWorld, Entity, IntoQuery, World};
use wgpu::{
    util::StagingBelt, Buffer, BufferAddress, BufferSize, CommandEncoder, CommandEncoderDescriptor,
    Device,
};

use std::{
    collections::BTreeMap,
    future::Future,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{BufferComponent, CommandBuffersComponent, ToBytes};

// Staging belt
static STAGING_BELT_ID_HEAD: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StagingBeltId(usize);

pub struct StagingBeltManager(BTreeMap<StagingBeltId, StagingBelt>);

impl StagingBeltManager {
    pub fn new() -> Self {
        StagingBeltManager(Default::default())
    }

    pub fn create_staging_belt(&mut self, chunk_size: BufferAddress) -> StagingBeltId {
        let staging_belt = StagingBelt::new(chunk_size);
        let id = STAGING_BELT_ID_HEAD.fetch_add(1, Ordering::Relaxed);
        let id = StagingBeltId(id);
        self.0.insert(id, staging_belt);
        id
    }

    pub fn write_buffer(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder,
        target: &Buffer,
        offset: BufferAddress,
        size: BufferSize,
        belt_id: &StagingBeltId,
        data: &[u8],
    ) {
        self.0
            .get_mut(belt_id)
            .unwrap()
            .write_buffer(encoder, target, offset, size, device)
            .copy_from_slice(data);
    }

    pub fn finish(&mut self, belt_id: &StagingBeltId) {
        self.0.get_mut(belt_id).unwrap().finish()
    }

    pub fn recall(&mut self, belt_id: &StagingBeltId) -> impl Future + Send {
        self.0.get_mut(belt_id).unwrap().recall()
    }
}

// Staging belt handle
pub struct StagingBeltComponent {
    chunk_size: BufferAddress,
    staging_belt: RwLock<LazyComponent<StagingBeltId>>,
    map_closures: RwLock<Vec<Box<dyn Fn(&World, &mut StagingBeltManager) + Send + Sync + 'static>>>,
}

impl_read_write_lock!(
    StagingBeltComponent,
    staging_belt,
    LazyComponent<StagingBeltId>
);

impl StagingBeltComponent {
    pub fn new(chunk_size: BufferAddress) -> Self {
        StagingBeltComponent {
            chunk_size,
            staging_belt: RwLock::new(LazyComponent::Pending),
            map_closures: Default::default(),
        }
    }

    pub fn chunk_size(&self) -> &BufferAddress {
        &self.chunk_size
    }

    pub fn map(&self, f: impl Fn(&World, &mut StagingBeltManager) + Send + Sync + 'static) {
        self.map_closures.write().push(Box::new(f));
    }

    pub fn flush_map_closures(&self, world: &World, staging_belt_manager: &mut StagingBeltManager) {
        for f in self.map_closures.write().drain(..) {
            f(world, staging_belt_manager);
        }
    }
}

// Staging belt buffer write operation
pub struct StagingBeltWriteComponent<T> {
    offset: RwLock<BufferAddress>,
    size: RwLock<BufferSize>,
    _phantom: PhantomData<T>,
}

impl<T> ReadWriteLock<BufferAddress> for StagingBeltWriteComponent<T> {
    fn read(&self) -> RwLockReadGuard<BufferAddress> {
        self.offset.read()
    }

    fn write(&self) -> RwLockWriteGuard<BufferAddress> {
        self.offset.write()
    }
}

impl<T> ReadWriteLock<BufferSize> for StagingBeltWriteComponent<T> {
    fn read(&self) -> RwLockReadGuard<BufferSize> {
        self.size.read()
    }

    fn write(&self) -> RwLockWriteGuard<BufferSize> {
        self.size.write()
    }
}

impl<T> StagingBeltWriteComponent<T> {
    pub fn new(offset: BufferAddress, size: BufferSize) -> Self {
        StagingBeltWriteComponent {
            offset: RwLock::new(offset),
            size: RwLock::new(size),
            _phantom: Default::default(),
        }
    }
}

pub fn assemble_staging_belt(
    cmd: &mut legion::systems::CommandBuffer,
    entity: legion::Entity,
    chunk_size: BufferAddress,
) {
    cmd.add_component(
        entity,
        Changed::new(StagingBeltComponent::new(chunk_size), false),
    )
}

pub fn assemble_staging_belt_data_with_usage<U, T>(
    cmd: &mut legion::systems::CommandBuffer,
    entity: legion::Entity,
    data: T,
    offset: BufferAddress,
    size: BufferSize,
) where
    U: Send + Sync + 'static,
    T: legion::storage::Component,
{
    cmd.add_component(entity, Changed::new(data, true));
    cmd.add_component(entity, StagingBeltWriteComponent::<T>::new(offset, size));
    cmd.add_indirect_component_self::<Changed<StagingBeltComponent>>(entity);
    cmd.add_indirect_component_self::<Usage<U, BufferComponent>>(entity);
    cmd.add_indirect_component_self::<CommandBuffersComponent>(entity);
}

// Initialize staging belts
pub fn create_staging_belt_thread_local(
    world: &World,
    staging_belt_manager: &mut StagingBeltManager,
) {
    <&Changed<StagingBeltComponent>>::query().for_each(world, |staging_belt| {
        if staging_belt.read().is_pending() {
            let staging_belt_id =
                staging_belt_manager.create_staging_belt(*staging_belt.chunk_size());
            staging_belt.write().set_ready(staging_belt_id);
            println!("Created staging belt with ID {:?}", staging_belt_id);
        }
    })
}

// Write data to buffer via staging belt
#[legion::system(par_for_each)]
#[read_component(Changed<StagingBeltComponent>)]
pub fn staging_belt_write<
    T: Send + Sync + 'static,
    L: ReadWriteLock<V> + Send + Sync + 'static,
    V: ToBytes,
>(
    world: &SubWorld,
    entity: &Entity,
    staging_belt_write: &StagingBeltWriteComponent<L>,
    staging_belt: &IndirectComponent<Changed<StagingBeltComponent>>,
) {
    let entity = *entity;

    let staging_belt = world.get_indirect(staging_belt).unwrap();

    let offset = *ReadWriteLock::<BufferAddress>::read(staging_belt_write);
    let size = *ReadWriteLock::<BufferSize>::read(staging_belt_write);

    staging_belt.map(move |world, staging_belt_manager| {
        let device = if let Some(device) = <&Device>::query().iter(world).next() {
            device
        } else {
            return;
        };

        let (
            data_component,
            staging_belt,
            buffer,
            command_buffers,
        ) = if let Ok(components) = <(
            &Changed<L>,
            &IndirectComponent<Changed<StagingBeltComponent>>,
            &IndirectComponent<Usage<T, BufferComponent>>,
            &IndirectComponent<CommandBuffersComponent>,
        )>::query()
            .get(world, entity) {
                components
            } else {
                return;
            };

        let staging_belt_component = world.get_indirect(staging_belt).unwrap();
        let buffer = world.get_indirect(buffer).unwrap();
        let command_buffers = world.get_indirect(command_buffers).unwrap();

        if data_component.get_changed() {
            let staging_belt = staging_belt_component.read();
            let staging_belt = if let LazyComponent::Ready(staging_belt) = &*staging_belt {
                staging_belt
            } else {
                return;
            };

            let buffer = buffer.read();
            let buffer = if let LazyComponent::Ready(buffer) = &*buffer {
                buffer
            } else {
                return;
            };

            let data = data_component.read();
            let bytes = data.to_bytes();

            let mut encoder =
                device.create_command_encoder(&CommandEncoderDescriptor { label: None });

            println!(
                    "Writing {} bytes to {} buffer at offset {} with size {} via staging belt with id {:?}",
                    bytes.len(),
                    std::any::type_name::<T>(),
                    offset,
                    size,
                    staging_belt,
                );

            staging_belt_manager.write_buffer(
                device,
                &mut encoder,
                buffer,
                offset,
                size,
                &*staging_belt,
                bytes,
            );

            command_buffers.write().push(encoder.finish());

            data_component.set_changed(false);
            staging_belt_component.set_changed(true);
        }
    });
}

pub fn staging_belt_flush_thread_local(
    world: &World,
    staging_belt_manager: &mut StagingBeltManager,
) {
    for staging_belt_component in <&Changed<StagingBeltComponent>>::query().iter(world) {
        staging_belt_component.flush_map_closures(world, staging_belt_manager);
    }
}

pub fn staging_belt_finish_thread_local(
    world: &World,
    staging_belt_manager: &mut StagingBeltManager,
) {
    <&Changed<StagingBeltComponent>>::query().for_each(world, |staging_belt| {
        if !staging_belt.get_changed() {
            return;
        }

        let staging_belt = staging_belt.read();
        let staging_belt = if let LazyComponent::Ready(staging_belt) = &*staging_belt {
            staging_belt
        } else {
            return;
        };
        staging_belt_manager.finish(staging_belt);
        println!("Finished staging belt with id {:?}", staging_belt);
    });
}

pub fn staging_belt_recall_thread_local(
    world: &World,
    staging_belt_manager: &mut StagingBeltManager,
) {
    <&Changed<StagingBeltComponent>>::query().for_each(world, |staging_belt_component| {
        if !staging_belt_component.get_changed() {
            return;
        }

        let staging_belt = staging_belt_component.read();
        let staging_belt = if let LazyComponent::Ready(staging_belt) = &*staging_belt {
            staging_belt
        } else {
            return;
        };

        // Ignore resulting future - this assumes the wgpu device is being polled in wait mode
        let _ = staging_belt_manager.recall(staging_belt);
        staging_belt_component.set_changed(false);
        println!("Recalled staging belt with id {:?}", staging_belt);
    });
}

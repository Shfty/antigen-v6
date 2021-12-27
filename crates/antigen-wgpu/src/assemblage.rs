use antigen_core::{
    AsUsage, Changed, ChangedFlag, Construct, Indirect, Usage, With,
};

use hecs::{Component, Entity};
use wgpu::{
    util::BufferInitDescriptor, Adapter, Backends, BufferAddress, BufferDescriptor,
    CommandEncoderDescriptor, Device, DeviceDescriptor, ImageCopyTextureBase, ImageDataLayout,
    Instance, Queue, SamplerDescriptor, ShaderModuleDescriptor, ShaderModuleDescriptorSpirV,
    Surface, SurfaceConfiguration, TextureDescriptor, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};

use std::path::Path;

use crate::{
    AdapterComponent, BufferComponent, BufferDescriptorComponent, BufferInitDescriptorComponent,
    BufferWriteComponent, CommandBuffersComponent, CommandEncoderComponent,
    CommandEncoderDescriptorComponent, DeviceComponent, InstanceComponent, QueueComponent,
    SamplerComponent, SamplerDescriptorComponent, ShaderModuleComponent,
    ShaderModuleDescriptorComponent, ShaderModuleDescriptorSpirVComponent, SurfaceComponent,
    SurfaceConfigurationComponent, SurfaceTextureComponent, TextureComponent,
    TextureDescriptorComponent, TextureViewComponent, TextureViewDescriptorComponent,
    TextureWriteComponent,
};

#[derive(hecs::Bundle)]
pub struct BackendBundle {
    instance: InstanceComponent,
    adapter: AdapterComponent,
    device: DeviceComponent,
    queue: QueueComponent,
}

impl BackendBundle {
    pub fn new(instance: Instance, adapter: Adapter, device: Device, queue: Queue) -> Self {
        let instance = InstanceComponent::construct(instance);
        let adapter = AdapterComponent::construct(adapter);
        let device = DeviceComponent::construct(device);
        let queue = QueueComponent::construct(queue);
        BackendBundle {
            instance,
            adapter,
            device,
            queue,
        }
    }

    pub fn from_env(
        device_desc: &DeviceDescriptor,
        compatible_surface: Option<&Surface>,
        trace_path: Option<&Path>,
    ) -> Self {
        let backend_bits = wgpu::util::backend_bits_from_env().unwrap_or(Backends::PRIMARY);

        let instance = Instance::new(backend_bits);
        println!("Created WGPU instance: {:#?}\n", instance);

        let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
            &instance,
            backend_bits,
            compatible_surface,
        ))
        .expect("Failed to acquire WGPU adapter");

        let adapter_info = adapter.get_info();
        println!("Acquired WGPU adapter: {:#?}\n", adapter_info);

        let (device, queue) =
            pollster::block_on(adapter.request_device(device_desc, trace_path)).unwrap();

        println!("Acquired WGPU device: {:#?}\n", device);
        println!("Acquired WGPU queue: {:#?}\n", queue);

        Self::new(instance, adapter, device, queue)
    }
}

#[derive(hecs::Bundle)]
pub struct WindowSurfaceBundle {
    surface_config: SurfaceConfigurationComponent,
    surface: SurfaceComponent,
    surface_texture: SurfaceTextureComponent,
    render_attachment_texture_view_desc: TextureViewDescriptorComponent<'static>,
    render_attachment_texture_view: TextureViewComponent,
}

impl WindowSurfaceBundle {
    pub fn new() -> Self {
        let surface_config = SurfaceConfigurationComponent::construct(SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: 0,
            height: 0,
            present_mode: wgpu::PresentMode::Immediate,
        })
        .with(ChangedFlag(false));

        let surface_texture = SurfaceTextureComponent::construct(None).with(ChangedFlag(false));

        let render_attachment_texture_view_desc =
            TextureViewDescriptorComponent::construct(TextureViewDescriptor::default())
                .with(ChangedFlag(false));

        let render_attachment_texture_view = TextureViewComponent::default();

        WindowSurfaceBundle {
            surface_config,
            surface: Default::default(),
            surface_texture,
            render_attachment_texture_view_desc,
            render_attachment_texture_view,
        }
    }
}

#[derive(hecs::Bundle)]
pub struct ShaderModuleBundle {
    descriptor: ShaderModuleDescriptorComponent<'static>,
    shader: ShaderModuleComponent,
}

impl ShaderModuleBundle {
    pub fn new(descriptor: ShaderModuleDescriptor<'static>) -> Self {
        let descriptor =
            ShaderModuleDescriptorComponent::construct(descriptor).with(ChangedFlag(true));

        ShaderModuleBundle {
            descriptor,
            shader: Default::default(),
        }
    }
}

#[derive(hecs::Bundle)]
pub struct ShaderModuleSpirVBundle {
    descriptor: ShaderModuleDescriptorSpirVComponent<'static>,
    shader: ShaderModuleComponent,
}

impl ShaderModuleSpirVBundle {
    pub fn new(descriptor: ShaderModuleDescriptorSpirV<'static>) -> Self {
        let descriptor =
            ShaderModuleDescriptorSpirVComponent::construct(descriptor).with(ChangedFlag(false));
        ShaderModuleSpirVBundle {
            descriptor,
            shader: Default::default(),
        }
    }
}

#[derive(hecs::Bundle)]
pub struct BufferBundle {
    descriptor: BufferDescriptorComponent<'static>,
    buffer: BufferComponent,
}

impl BufferBundle {
    pub fn new(descriptor: BufferDescriptor<'static>) -> Self {
        let descriptor = BufferDescriptorComponent::construct(descriptor).with(ChangedFlag(true));
        BufferBundle {
            descriptor,
            buffer: Default::default(),
        }
    }
}

#[derive(hecs::Bundle)]
pub struct BufferInitBundle {
    descriptor: BufferInitDescriptorComponent<'static>,
    buffer: BufferComponent,
}

impl BufferInitBundle {
    pub fn new(descriptor: BufferInitDescriptor<'static>) -> Self {
        let descriptor =
            BufferInitDescriptorComponent::construct(descriptor).with(ChangedFlag(true));

        BufferInitBundle {
            descriptor,
            buffer: Default::default(),
        }
    }
}

#[derive(hecs::Bundle)]
pub struct BufferDataBundle<T> {
    data: Changed<T>,
    buffer_write: BufferWriteComponent<T>,
    buffer_entity: Usage<BufferWriteComponent<T>, Indirect<&'static BufferComponent>>,
}

impl<T> BufferDataBundle<T> {
    pub fn new(data: T, offset: BufferAddress, buffer_entity: Entity) -> Self {
        let data = Changed::<T>::construct(data).with(ChangedFlag(true));
        let buffer_write = BufferWriteComponent::<T>::new(offset);
        let buffer_entity = BufferWriteComponent::<T>::as_usage(
            Indirect::<&BufferComponent>::construct(buffer_entity),
        );
        BufferDataBundle {
            data,
            buffer_write,
            buffer_entity,
        }
    }
}

#[derive(hecs::Bundle)]
pub struct TextureBundle {
    descriptor: TextureDescriptorComponent<'static>,
    texture: TextureComponent,
}

impl TextureBundle {
    pub fn new(descriptor: TextureDescriptor<'static>) -> Self {
        let descriptor = TextureDescriptorComponent::construct(descriptor).with(ChangedFlag(true));
        TextureBundle {
            descriptor,
            texture: Default::default(),
        }
    }
}

#[derive(hecs::Bundle)]
pub struct TextureDataBundle<T> {
    data: Changed<T>,
    texture_write: TextureWriteComponent<T>,
    texture_entity: Usage<TextureWriteComponent<T>, Indirect<&'static TextureComponent>>,
}

impl<T> TextureDataBundle<T>
where
    T: Component,
{
    pub fn new(
        data: T,
        image_copy_texture: ImageCopyTextureBase<()>,
        image_data_layout: ImageDataLayout,
        texture_entity: Entity,
    ) -> Self {
        let data = Changed::<T>::construct(data).with(ChangedFlag(true));
        let texture_write = TextureWriteComponent::<T>::new(image_copy_texture, image_data_layout);
        let texture_entity = TextureWriteComponent::<T>::as_usage(
            Indirect::<&TextureComponent>::construct(texture_entity),
        );
        TextureDataBundle {
            data,
            texture_write,
            texture_entity,
        }
    }
}

#[derive(hecs::Bundle)]
pub struct TextureViewBundle {
    descriptor: TextureViewDescriptorComponent<'static>,
    texture_view: TextureViewComponent,
}

impl TextureViewBundle {
    pub fn new(descriptor: TextureViewDescriptor<'static>) -> Self {
        let descriptor =
            TextureViewDescriptorComponent::construct(descriptor).with(ChangedFlag(true));
        TextureViewBundle {
            descriptor,
            texture_view: Default::default(),
        }
    }
}

#[derive(hecs::Bundle)]
pub struct SamplerBundle {
    descriptor: SamplerDescriptorComponent<'static>,
    sampler: SamplerComponent,
}

impl SamplerBundle {
    pub fn new(descriptor: SamplerDescriptor<'static>) -> Self {
        let descriptor = SamplerDescriptorComponent::construct(descriptor).with(ChangedFlag(false));
        SamplerBundle {
            descriptor,
            sampler: Default::default(),
        }
    }
}

pub enum PushConstant {}
pub type PushConstantComponent = Usage<PushConstant, Vec<u8>>;
pub type PushConstantOffset = Usage<PushConstant, u32>;

#[derive(hecs::Bundle)]
pub struct PushConstantBundle {
    data: PushConstantComponent,
    offset: PushConstantOffset,
}

impl PushConstantBundle {
    pub fn new(data: Vec<u8>, offset: u32) -> Self {
        let data = PushConstant::as_usage(data);
        let offset = PushConstant::as_usage(offset);
        PushConstantBundle { data, offset }
    }
}

#[derive(hecs::Query)]
pub struct PushConstantQuery<'a> {
    pub data: &'a PushConstantComponent,
    pub offset: &'a PushConstantOffset,
}

#[derive(hecs::Bundle)]
pub struct CommandEncoderBundle {
    desc: CommandEncoderDescriptorComponent,
    encoder: CommandEncoderComponent,
    command_buffers_entity:
        Usage<CommandEncoderComponent, Indirect<&'static mut CommandBuffersComponent>>,
}

impl CommandEncoderBundle {
    pub fn new(desc: CommandEncoderDescriptor<'static>, command_encoder_entity: Entity) -> Self {
        let desc = CommandEncoderDescriptorComponent::construct(desc);
        let command_buffers_entity =
            CommandEncoderComponent::as_usage(Indirect::construct(command_encoder_entity));
        CommandEncoderBundle {
            desc,
            encoder: Default::default(),
            command_buffers_entity,
        }
    }
}

use antigen_core::{Changed, LazyComponent, Usage};

use wgpu::{Adapter, BindGroup, BindGroupLayout, Buffer, BufferAddress, BufferDescriptor, CommandBuffer, CommandEncoder, CommandEncoderDescriptor, ComputePipeline, Device, ImageCopyTextureBase, ImageDataLayout, Instance, PipelineLayout, Queue, RenderBundle, RenderPipeline, Sampler, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderModuleDescriptorSpirV, Surface, SurfaceConfiguration, SurfaceTexture, Texture, TextureDescriptor, TextureView, TextureViewDescriptor, util::BufferInitDescriptor};

use std::{marker::PhantomData, sync::Arc};

// Backend primitives
pub type InstanceComponent = Arc<Instance>;
pub type AdapterComponent = Arc<Adapter>;
pub type DeviceComponent = Arc<Device>;
pub type QueueComponent = Arc<Queue>;

// WGPU surface configuration
pub type SurfaceConfigurationComponent = Changed<SurfaceConfiguration>;

// WGPU surface
pub type SurfaceComponent = LazyComponent<Surface>;

// WGPU texture descriptor
pub type TextureDescriptorComponent<'a> = Changed<TextureDescriptor<'a>>;

// WGPU texture
pub type TextureComponent = LazyComponent<Texture>;

// MSAA frambuffer usage flag for TextureComponent
pub enum MsaaFramebuffer {}

pub type MsaaFramebufferTextureDescriptor<'a> =
    Usage<MsaaFramebuffer, TextureDescriptorComponent<'a>>;
pub type MsaaFramebufferTexture = Usage<MsaaFramebuffer, TextureComponent>;

pub type MsaaFramebufferTextureViewDescriptor<'a> =
    Usage<MsaaFramebuffer, TextureViewDescriptorComponent<'a>>;
pub type MsaaFramebufferTextureView = Usage<MsaaFramebuffer, TextureViewComponent>;

// WGPU surface texture
pub type SurfaceTextureComponent = Changed<Option<SurfaceTexture>>;

// WPGU texture view descriptor
pub type TextureViewDescriptorComponent<'a> = Changed<TextureViewDescriptor<'a>>;

// WGPU texture view
pub type TextureViewComponent = LazyComponent<TextureView>;

// WGPU sampler descriptor
pub type SamplerDescriptorComponent<'a> = Changed<SamplerDescriptor<'a>>;

// WGPU sampler
pub type SamplerComponent = LazyComponent<Sampler>;

// WGPU pipeline layout
pub type PipelineLayoutComponent = LazyComponent<PipelineLayout>;

// WGPU render pipeline
pub type RenderPipelineComponent = LazyComponent<RenderPipeline>;

// WGPU compute pipeline
pub type ComputePipelineComponent = LazyComponent<ComputePipeline>;

// WGPU render bundle
pub type RenderBundleComponent = LazyComponent<RenderBundle>;

// WGPU bind group layout
pub type BindGroupLayoutComponent = LazyComponent<BindGroupLayout>;

// WGPU bind group
pub type BindGroupComponent = LazyComponent<BindGroup>;

// WGPU command buffers
pub type CommandBuffersComponent = Vec<CommandBuffer>;

// WGPU buffer descriptor
pub type BufferDescriptorComponent<'a> = Changed<BufferDescriptor<'a>>;

// WGPU buffer init descriptor
pub type BufferInitDescriptorComponent<'a> = Changed<BufferInitDescriptor<'a>>;

// WGPU buffer
pub type BufferComponent = LazyComponent<Buffer>;

// Buffer write operation
pub struct BufferWriteComponent<T> {
    offset: BufferAddress,
    _phantom: PhantomData<T>,
}

impl<T> BufferWriteComponent<T> {
    pub fn new(offset: BufferAddress) -> Self {
        BufferWriteComponent {
            offset,
            _phantom: Default::default(),
        }
    }

    pub fn offset(&self) -> BufferAddress {
        self.offset
    }
}

// Texture write operation
pub struct TextureWriteComponent<T> {
    image_copy_texture: ImageCopyTextureBase<()>,
    image_data_layout: ImageDataLayout,
    _phantom: PhantomData<T>,
}

impl<T> TextureWriteComponent<T> {
    pub fn new(
        image_copy_texture: ImageCopyTextureBase<()>,
        image_data_layout: ImageDataLayout,
    ) -> Self {
        TextureWriteComponent {
            image_copy_texture,
            image_data_layout,
            _phantom: Default::default(),
        }
    }

    pub fn image_copy_texture(&self) -> &ImageCopyTextureBase<()> {
        &self.image_copy_texture
    }

    pub fn image_data_layout(&self) -> &ImageDataLayout {
        &self.image_data_layout
    }
}

// WGPU shader module descriptor
pub type ShaderModuleDescriptorComponent<'a> = Changed<ShaderModuleDescriptor<'a>>;

// WGPU shader module descriptor
pub type ShaderModuleDescriptorSpirVComponent<'a> = Changed<ShaderModuleDescriptorSpirV<'a>>;

// WGPU shader module
pub type ShaderModuleComponent = LazyComponent<ShaderModule>;

// Texture texels usage tag
pub enum Texels {}

// Mesh vertices usage tag
pub enum MeshVertices {}

// Mesh UVs usage tag
pub enum MeshUvs {}

// Mesh indices usage tag
pub enum MeshIndices {}

pub type CommandEncoderDescriptorComponent = Changed<CommandEncoderDescriptor<'static>>;
pub type CommandEncoderComponent = LazyComponent<CommandEncoder>;

use bytemuck::{Pod, Zeroable};
use std::time::Instant;

use antigen_core::{Changed, Usage};
use antigen_wgpu::{BindGroupComponent, BindGroupLayoutComponent, BufferComponent, RenderPipelineComponent, SamplerComponent, ShaderModuleComponent, TextureComponent, TextureDescriptorComponent, TextureViewComponent, TextureViewDescriptorComponent, ToBytes};

// Phosphor renderer tag
pub struct PhosphorRenderer;

// Usage tags
pub enum Position {}

pub enum StartTime {}
pub enum Timestamp {}
pub enum TotalTime {}
pub enum DeltaTime {}

pub struct BeamBuffer;
pub struct BeamMultisample;
pub struct BeamDepthBuffer;

pub enum Linear {}

pub struct MeshVertex;
pub struct MeshIndex;
pub struct LineVertex;
pub struct LineIndex;
pub struct LineInstance;

pub struct Perspective;
pub struct Orthographic;

pub enum FlipFlop {}

pub enum Origin {}

pub struct Uniform;
pub struct ComputeLineInstances;
pub struct PhosphorDecay;
pub struct PhosphorFrontBuffer;
pub struct PhosphorBackBuffer;
pub struct BeamLine;
pub struct BeamMesh;
pub struct Tonemap;

pub enum MapFile {}

#[derive(Debug)]
pub enum VertexCount {}

#[derive(Debug)]
pub enum MeshIndexCount {}

#[derive(Debug)]
pub enum LineIndexCount {}

// Usage-tagged components
pub type StartTimeComponent = Usage<StartTime, Instant>;
pub type TimestampComponent = Usage<Timestamp, Instant>;
pub type TotalTimeComponent = Usage<TotalTime, f32>;
pub type DeltaTimeComponent = Usage<DeltaTime, f32>;
pub type PerspectiveMatrixComponent = Usage<Perspective, [[f32; 4]; 4]>;
pub type OrthographicMatrixComponent = Usage<Orthographic, [[f32; 4]; 4]>;
pub type VertexCountComponent = Usage<VertexCount, u64>;
pub type MeshIndexCountComponent = Usage<MeshIndexCount, u64>;
pub type LineIndexCountComponent = Usage<LineIndexCount, u64>;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct UniformData {
    perspective: [[f32; 4]; 4],
    orthographic: [[f32; 4]; 4],
    total_time: f32,
    delta_time: f32,
    _pad: [f32; 2],
}

pub type OriginComponent = Usage<Origin, (f32, f32, f32)>;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct LineVertexData {
    pub position: [f32; 3],
    pub end: f32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct LineInstanceData {
    pub v0: MeshVertexData,
    pub v1: MeshVertexData,
}

impl ToBytes for LineInstanceData {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct MeshVertexData {
    pub position: [f32; 3],
    pub surface_color: [f32; 3],
    pub line_color: [f32; 3],
    pub intensity: f32,
    pub delta_intensity: f32,
    pub _pad: f32,
}

impl MeshVertexData {
    pub fn new(
        position: (f32, f32, f32),
        surface_color: (f32, f32, f32),
        line_color: (f32, f32, f32),
        intensity: f32,
        delta_intensity: f32,
    ) -> Self {
        MeshVertexData {
            position: [position.0, position.1, position.2],
            surface_color: [surface_color.0, surface_color.1, surface_color.2],
            line_color: [line_color.0, line_color.1, line_color.2],
            intensity,
            delta_intensity,
            ..Default::default()
        }
    }
}

pub type MeshVertexDataComponent = Vec<MeshVertexData>;

pub type MeshIndexDataComponent = Vec<u16>;

pub type BufferFlipFlopComponent = Usage<FlipFlop, bool>;

pub struct Oscilloscope {
    f: Box<dyn Fn(f32) -> (f32, f32, f32) + Send + Sync>,
    speed: f32,
    magnitude: f32,
}

impl Oscilloscope {
    pub fn new<F>(speed: f32, magnitude: f32, f: F) -> Self
    where
        F: Fn(f32) -> (f32, f32, f32) + Send + Sync + 'static,
    {
        Oscilloscope {
            speed,
            magnitude,
            f: Box::new(f),
        }
    }

    pub fn eval(&self, f: f32) -> (f32, f32, f32) {
        let (x, y, z) = (self.f)(f * self.speed);
        (x * self.magnitude, y * self.magnitude, z * self.magnitude)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Timer {
    pub timestamp: std::time::Instant,
    pub duration: std::time::Duration,
}

pub type TimerComponent = Changed<Timer>;

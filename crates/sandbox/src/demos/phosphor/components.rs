use bytemuck::{Pod, Zeroable};
use parking_lot::RwLock;
use std::{collections::BTreeMap, sync::Arc, time::Instant};

use antigen_core::{Changed, Usage, LazyComponent};

// Phosphor renderer tag
pub struct PhosphorRenderer;

// Usage tags
pub enum StartTime {}
pub enum Timestamp {}
pub enum TotalTime {}
pub enum DeltaTime {}

pub struct BeamBuffer;
pub struct BeamMultisample;
pub struct BeamDepthBuffer;

#[derive(Debug, Copy, Clone)]
pub struct Vertices;

#[derive(Debug, Copy, Clone)]
pub struct TriangleIndices;

#[derive(Debug, Copy, Clone)]
pub struct TriangleMeshes;

#[derive(Debug, Copy, Clone)]
pub struct TriangleMeshInstances;

#[derive(Debug, Copy, Clone)]
pub struct LineVertices;

#[derive(Debug, Copy, Clone)]
pub struct LineIndices;

#[derive(Debug, Copy, Clone)]
pub struct LineMeshes;

#[derive(Debug, Copy, Clone)]
pub struct LineMeshInstances;

#[derive(Debug, Copy, Clone)]
pub struct LineInstances;

pub struct Perspective;
pub struct Orthographic;

pub struct Uniform;
pub struct StorageBuffers;
pub struct PhosphorDecay;
pub struct PhosphorFrontBuffer;
pub struct PhosphorBackBuffer;
pub struct Beam;
pub struct BeamLine;
pub struct BeamMesh;
pub struct Tonemap;

pub enum MapFile {}

// Usage-tagged components
pub type StartTimeComponent = Usage<StartTime, Instant>;
pub type TimestampComponent = Usage<Timestamp, Instant>;
pub type TotalTimeComponent = Usage<TotalTime, f32>;
pub type DeltaTimeComponent = Usage<DeltaTime, f32>;
pub type PerspectiveMatrixComponent = Usage<Perspective, [[f32; 4]; 4]>;
pub type OrthographicMatrixComponent = Usage<Orthographic, [[f32; 4]; 4]>;

/// Mesh ID map
#[derive(Copy, Clone)]
pub struct MeshIds;

pub type MeshIdsComponent = Arc<RwLock<BTreeMap<String, (Option<u32>, Option<(u32, u32)>)>>>;

// Line Mesh ID
pub enum LineMeshId {}
pub type LineMeshIdComponent = Usage<LineMeshId, u32>;

/// Singleton shader data
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct UniformData {
    perspective: [[f32; 4]; 4],
    orthographic: [[f32; 4]; 4],
    total_time: f32,
    delta_time: f32,
    _pad_0: [f32; 2],
}

/// Vertex data for 2D line meshes
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct LineVertexData {
    pub position: [f32; 3],
    pub end: f32,
}

pub type LineVertexDataComponent = Vec<LineVertexData>;

/// Vertex data for 3D triangle meshes
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct VertexData {
    pub position: [f32; 3],
    pub surface_color: [f32; 3],
    pub line_color: [f32; 3],
    pub intensity: f32,
    pub delta_intensity: f32,
    pub _pad: f32,
}

impl VertexData {
    pub fn new(
        position: (f32, f32, f32),
        surface_color: (f32, f32, f32),
        line_color: (f32, f32, f32),
        intensity: f32,
        delta_intensity: f32,
    ) -> Self {
        VertexData {
            position: [position.0, position.1, position.2],
            surface_color: [surface_color.0, surface_color.1, surface_color.2],
            line_color: [line_color.0, line_color.1, line_color.2],
            intensity,
            delta_intensity,
            ..Default::default()
        }
    }
}

pub type VertexDataComponent = Vec<VertexData>;

pub type TriangleIndexData = u16;
pub type TriangleIndexDataComponent = Vec<TriangleIndexData>;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct TriangleMeshData {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub index_offset: u32,
    pub vertex_offset: u32,
    pub _pad: u32,
}

pub type TriangleMeshDataComponent = Vec<TriangleMeshData>;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct TriangleMeshInstanceData {
    pub position: [f32; 3],
    pub _pad1: f32,
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub _pad2: f32,
}

pub type TriangleMeshInstanceDataComponent = Vec<TriangleMeshInstanceData>;

pub type LineIndexData = u32;
pub type LineIndexDataComponent = Vec<LineIndexData>;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct LineMeshData {
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub index_offset: u32,
    pub index_count: u32,
}

pub type LineMeshDataComponent = Vec<LineMeshData>;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct LineMeshInstanceData {
    pub position: [f32; 3],
    pub mesh: u32,
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub _pad: f32,
}

pub type LineMeshInstanceDataComponent = Vec<LineMeshInstanceData>;

/// Instance data representing a single line
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct LineInstanceData {
    pub mesh_instance: u32,
    pub line_index: u32,
}

pub type LineInstanceDataComponent = Vec<LineInstanceData>;

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

pub enum MeshInstance {}
pub type MeshInstanceComponent<'a> = Usage<MeshInstance, LazyComponent<(), &'a str>>;


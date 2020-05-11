//!
//! Pipeline that sphere marches the voxel grid of atoms in a compute shader.
//!

use crate::utils::load_glsl;
use wgpu;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RaymarchGlobals {
    pub projection: [f32; 16],
    pub camera_origin: [f32; 3],
    pub padd0: f32,

    pub bb_min: [f32; 3],
    pub padd1: f32,
    pub bb_max: [f32; 3],
    pub padd2: f32,
    pub bb_diff: [f32; 3],
    pub padd3: f32,
    pub bb_size: [f32; 3],
    pub padd4: f32,
    pub window_size: [f32; 2],

    pub voxel_length: f32,

    pub time: f32,
    pub solvent_radius: f32,
    pub max_neighbours: i32,
    pub save: i32,
}

unsafe impl bytemuck::Zeroable for RaymarchGlobals {}
unsafe impl bytemuck::Pod for RaymarchGlobals {}

impl Default for RaymarchGlobals {
    fn default() -> Self {
        Self {
            projection: [0.0; 16],
            camera_origin: [0.0; 3],
            padd0: 0.0,

            bb_min: [0.0; 3],
            padd1: 0.0,
            bb_max: [0.0; 3],
            padd2: 0.0,
            bb_diff: [0.0; 3],
            padd3: 0.0,
            bb_size: [0.0; 3],
            padd4: 0.0,
            window_size: [0.0; 2],

            voxel_length: 0.0,           

            time: 0.0,
            solvent_radius: 0.0,
            max_neighbours: 0,
            save: 0,
        }
    }
}

pub struct RaymarchPipeline {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl RaymarchPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        // Shaders
        let cs_bytes = load_glsl(include_bytes!("raymarch.comp.spv"));
        let cs_module = device.create_shader_module(&cs_bytes);

        // Bind Groups
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raymarch bind group layout"),
            bindings: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                        format: wgpu::TextureFormat::Rgba32Float,
                        readonly: false,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageBuffer {
                        dynamic: false,
                        readonly: true,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageBuffer {
                        dynamic: false,
                        readonly: true,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                        format: wgpu::TextureFormat::R32Float,
                        readonly: false,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                        format: wgpu::TextureFormat::Rgba32Float,
                        readonly: false,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                        format: wgpu::TextureFormat::Rgba32Float,
                        readonly: false,
                    },
                },
            ],
        });

        // Pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: &pipeline_layout,
            compute_stage: wgpu::ProgrammableStageDescriptor {
                module: &cs_module,
                entry_point: "main",
            },
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

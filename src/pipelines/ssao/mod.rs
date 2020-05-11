//!
//! Pipeline implementin Screen-Space Ambient Occlusion.
//!

use crate::utils::load_glsl;
use nalgebra_glm as glm;
use rand::Rng;
use wgpu;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SsaoGlobals {
    // mat4 projection;
    pub projection: [f32; 16],
    // vec4 samples[64];
    pub samples: [f32; 64 * 4],
    // vec4 noise[4][4];
    pub noise: [f32; 4 * 4 * 4],
}

unsafe impl bytemuck::Zeroable for SsaoGlobals {}
unsafe impl bytemuck::Pod for SsaoGlobals {}

fn lerp(a: f32, b: f32, f: f32) -> f32 {
    a + f * (b - a)
}

impl Default for SsaoGlobals {
    fn default() -> Self {
        let mut rng = rand::thread_rng();

        let mut samples = [0.0; 64 * 4];
        for i in 0..64 {
            let mut sample = glm::vec3(
                rng.gen_range(0.0, 1.0) * 2.0 - 1.0,
                rng.gen_range(0.0, 1.0) * 2.0 - 1.0,
                rng.gen_range(0.0, 1.0),
            );
            sample = glm::normalize(&sample);
            sample *= rng.gen_range(0.0, 1.0);

            let mut scale = i as f32 / 64.0;
            scale = lerp(0.1, 1.0, scale * scale);
            sample *= scale;

            samples[i * 4] = sample.x;
            samples[i * 4 + 1] = sample.y;
            samples[i * 4 + 2] = sample.z;
        }

        let mut noise = [0.0; 4 * 4 * 4];
        for i in 0..16 {
            let sample = glm::vec3(rng.gen_range(0.0, 1.0) * 2.0 - 1.0, rng.gen_range(0.0, 1.0) * 2.0 - 1.0, 0.0);

            noise[i * 4] = sample.x;
            noise[i * 4 + 1] = sample.y;
            noise[i * 4 + 2] = sample.z;
        }

        Self {
            projection: [0.0; 16],
            samples,
            noise,
        }
    }
}

pub struct SsaoPipeline {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl SsaoPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        // Shaders
        let cs_bytes = load_glsl(include_bytes!("ssao.comp.spv"));
        let cs_module = device.create_shader_module(&cs_bytes);

        // Bind Groups
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSAO bind group layout"),
            bindings: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        multisampled: false,
                        dimension: wgpu::TextureViewDimension::D2,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::Sampler { comparison: false },
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

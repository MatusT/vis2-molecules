//!
//! Module containing the application itself.
//!

use crate::camera::*;
use crate::grid::*;
use crate::pipelines::{raymarch::*, render::*, ssao::*};
use lib3dmol::structures::GetAtom;
use nalgebra_glm as glm;
use std::convert::TryInto;
use std::time::SystemTime;
use wgpu;

pub struct Application {
    /// Width of the window
    width: u32,
    /// Height of the window
    height: u32,

    /// Device used for rendering
    device: wgpu::Device,
    /// Main queue to which GPU commands are sent
    queue: wgpu::Queue,

    /// Time of initilization of program. Used for animation.
    start_time: SystemTime,

    /// Camera of the application.
    pub camera: RotationCamera,
    /// Holds information whether camera was changed between frames. The information is used for accumulation of result.
    pub camera_changed: bool,

    /// Voxel grid containing atoms of the molecule.
    voxel_grid: VoxelGrid,

    /// Global variables for ray marching passed to GPU.
    raymarch_globals: RaymarchGlobals,
    /// GPU buffer for `raymarch_globals`.
    raymarch_globals_buffer: wgpu::Buffer,

    /// Global variables for SSAO computation passed to GPU.
    ssao_globals_buffer: wgpu::Buffer,

    /// Pipeline for ray marching.
    raymarch_pipeline: RaymarchPipeline,

    /// Pipeline that renders the sphere marched result to the window.
    render_pipeline: RenderPipeline,

    /// Pipeline that adds SSAO to the sphere marched result.
    ssao_pipeline: SsaoPipeline,

    gbuffer_positions: wgpu::TextureView,
    gbuffer_normals: wgpu::TextureView,
    output_texture: wgpu::TextureView,

    sdf_default: wgpu::Buffer,
    /// Texture where signed distance field is stored.
    /// Used to progressively enhance view when camera did not change between frames.
    sdf_texture: wgpu::Texture,
    sdf_texture_view: wgpu::TextureView,

    mouse_pressed: bool,
    mouse_position: winit::dpi::PhysicalPosition<f64>,

    sampler: wgpu::Sampler,
}

impl Application {
    ///
    /// Initialized the application.
    ///
    pub async fn new(width: u32, height: u32, surface: &wgpu::Surface) -> Self {
        // let adapter = &wgpu::Adapter::enumerate(wgpu::BackendBit::PRIMARY)[1];
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        .unwrap();

        println!("{}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: wgpu::Limits::default(),
            })
            .await;

        let raymarch_pipeline = RaymarchPipeline::new(&device);
        let render_pipeline = RenderPipeline::new(&device);
        let ssao_pipeline = SsaoPipeline::new(&device);

        //
        // Globals
        //
        let start_time = SystemTime::now();

        let voxel_grid = {
            let mut atoms = Vec::new();
            atoms.push(glm::vec4(1.5, 0.0, 0.0, 1.0));
            atoms.push(glm::vec4(-1.5, 0.0, 0.0, 1.0));
            atoms.push(glm::vec4(0.0, 2.5, 0.0, 1.0));

            VoxelGrid::new(&device, 2.0, atoms)
        };

        let camera = RotationCamera::new(0.5 * glm::distance(&glm::vec3(0.0, 0.0, 0.0), &voxel_grid.bb_diff));
        let projection = glm::perspective(width as f32 / height as f32, 1.57079633 * 0.5, 0.01, 100.0);

        let raymarch_globals = RaymarchGlobals {
            window_size: [width as f32, height as f32],
            projection: projection.as_slice().try_into().unwrap(),
            camera_origin: camera.eye().as_slice().try_into().expect(""),
            bb_min: voxel_grid.bb_min.into(),
            bb_max: voxel_grid.bb_max.into(),
            bb_diff: voxel_grid.bb_diff.into(),
            bb_size: voxel_grid.bb_size.into(),
            voxel_length: voxel_grid.voxel_length,
            solvent_radius: 0.71590906,
            max_neighbours: 15,
            time: 0.0,
            save: 0,
            max_steps: 8,
            ..Default::default()
        };
        let raymarch_globals_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[raymarch_globals]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let ssao_globals = SsaoGlobals {
            projection: projection.as_slice().try_into().unwrap(),
            ..Default::default()
        };
        let ssao_globals_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[ssao_globals]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let gbuffer_positions = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer positions texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::STORAGE | wgpu::TextureUsage::SAMPLED,
        });
        let gbuffer_positions = gbuffer_positions.create_default_view();

        let gbuffer_normals = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer normals texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::STORAGE | wgpu::TextureUsage::SAMPLED,
        });
        let gbuffer_normals = gbuffer_normals.create_default_view();

        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::STORAGE,
        });
        let output_texture = output_texture.create_default_view();

        let sdf_default_cpu = vec![std::f32::NEG_INFINITY; (width * height) as usize];
        let sdf_default = device.create_buffer_with_data(bytemuck::cast_slice(&sdf_default_cpu), wgpu::BufferUsage::COPY_SRC);
        let sdf_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SDF texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsage::STORAGE | wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::WRITE_ALL,
        });
        let sdf_texture_view = sdf_texture.create_default_view();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Copy sdf encoder"),
        });
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &sdf_default,
                offset: 0,
                bytes_per_row: width * 4,
                rows_per_image: height,
            },
            wgpu::TextureCopyView {
                texture: &sdf_texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::Extent3d { width, height, depth: 1 },
        );
        queue.submit(&[encoder.finish()]);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: wgpu::CompareFunction::Undefined,
        });

        Self {
            width,
            height,

            device,
            queue,

            start_time,

            camera,
            camera_changed: true,

            voxel_grid,

            raymarch_globals,
            raymarch_globals_buffer,
            ssao_globals_buffer,

            raymarch_pipeline,
            render_pipeline,
            ssao_pipeline,

            gbuffer_positions,
            gbuffer_normals,
            output_texture,
            sdf_default,
            sdf_texture,
            sdf_texture_view,

            mouse_pressed: false,
            mouse_position: winit::dpi::PhysicalPosition { x: 0.0, y: 0.0 },

            sampler,
        }
    }

    ///
    /// Called when window is resized. Recreates textures for rendering.
    ///
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        let gbuffer_positions = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer positions texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::STORAGE | wgpu::TextureUsage::SAMPLED,
        });
        self.gbuffer_positions = gbuffer_positions.create_default_view();

        let gbuffer_normals = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer normals texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::STORAGE | wgpu::TextureUsage::SAMPLED,
        });
        self.gbuffer_normals = gbuffer_normals.create_default_view();

        let output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::STORAGE,
        });
        self.output_texture = output_texture.create_default_view();

        let sdf_default_cpu = vec![std::f32::NEG_INFINITY; (width * height) as usize];
        self.sdf_default = self
            .device
            .create_buffer_with_data(bytemuck::cast_slice(&sdf_default_cpu), wgpu::BufferUsage::COPY_SRC);
        self.sdf_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SDF texture"),
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsage::STORAGE | wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::WRITE_ALL,
        });
        self.sdf_texture_view = self.sdf_texture.create_default_view();

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Copy sdf encoder"),
        });
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &self.sdf_default,
                offset: 0,
                bytes_per_row: self.width * 4,
                rows_per_image: self.height,
            },
            wgpu::TextureCopyView {
                texture: &self.sdf_texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::Extent3d { width, height, depth: 1 },
        );

        self.raymarch_globals.window_size = [width as f32, height as f32];

        self.queue.submit(&[encoder.finish()]);
    }

    ///
    /// Called each frame to render.
    ///
    pub fn render(&mut self, frame: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main encoder"),
        });

        // Copy new data to the GPU
        {
            let now = SystemTime::now();
            self.raymarch_globals.time = now.duration_since(self.start_time).expect("Time went backwards").as_secs_f32();

            if self.camera_changed {
                let eye = self.camera.distance * self.camera.direction_vector();
                self.raymarch_globals.camera_origin = eye.as_slice().try_into().expect("");

                encoder.copy_buffer_to_texture(
                    wgpu::BufferCopyView {
                        buffer: &self.sdf_default,
                        offset: 0,
                        bytes_per_row: self.width * 4,
                        rows_per_image: self.height,
                    },
                    wgpu::TextureCopyView {
                        texture: &self.sdf_texture,
                        mip_level: 0,
                        array_layer: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    wgpu::Extent3d {
                        width: self.width,
                        height: self.height,
                        depth: 1,
                    },
                );

                self.camera_changed = false;
            }

            let raymarch_globals_size = std::mem::size_of::<RaymarchGlobals>();
            let raymarch_globals_buffer = self
                .device
                .create_buffer_with_data(bytemuck::cast_slice(&[self.raymarch_globals]), wgpu::BufferUsage::COPY_SRC);

            encoder.copy_buffer_to_buffer(
                &raymarch_globals_buffer,
                0,
                &self.raymarch_globals_buffer,
                0,
                raymarch_globals_size as wgpu::BufferAddress,
            );
        }

        let raymarch_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raymarch bind group"),
            layout: &self.raymarch_pipeline.bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &self.raymarch_globals_buffer,
                        range: 0..std::mem::size_of::<RaymarchGlobals>() as u64,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.output_texture),
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &self.voxel_grid.voxels,
                        range: 0..(self.voxel_grid.voxels_len * std::mem::size_of::<f32>()) as u64,
                    },
                },
                wgpu::Binding {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &self.voxel_grid.voxel_pointers,
                        range: 0..(self.voxel_grid.voxel_pointers_len * std::mem::size_of::<VoxelPointer>()) as u64,
                    },
                },
                wgpu::Binding {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.sdf_texture_view),
                },
                wgpu::Binding {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.gbuffer_positions),
                },
                wgpu::Binding {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.gbuffer_normals),
                },
            ],
        });

        let render_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render bind group"),
            layout: &self.render_pipeline.bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&self.output_texture),
            }],
        });

        let ssao_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO bind group"),
            layout: &self.ssao_pipeline.bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &self.raymarch_globals_buffer,
                        range: 0..std::mem::size_of::<RaymarchGlobals>() as u64,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &self.ssao_globals_buffer,
                        range: 0..std::mem::size_of::<SsaoGlobals>() as u64,
                    },
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.gbuffer_positions),
                },
                wgpu::Binding {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::Binding {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.gbuffer_normals),
                },
                wgpu::Binding {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::Binding {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.output_texture),
                },
            ],
        });

        // Raymarch the scene
        {
            let mut cpass = encoder.begin_compute_pass();
            cpass.set_pipeline(&self.raymarch_pipeline.pipeline);
            cpass.set_bind_group(0, &raymarch_bind_group, &[]);
            cpass.dispatch((self.width + 31) / 32, (self.height + 32) / 32, 1);
        }

        // SSAO
        {
            let mut cpass = encoder.begin_compute_pass();
            cpass.set_pipeline(&self.ssao_pipeline.pipeline);
            cpass.set_bind_group(0, &ssao_bind_group, &[]);
            cpass.dispatch((self.width + 31) / 32, (self.height + 32) / 32, 1);
        }

        // Render the output to the screen
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::GREEN,
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.render_pipeline.pipeline);
            rpass.set_bind_group(0, &render_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(&[encoder.finish()]);
    }

    pub fn window_event(&mut self, event: &winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                if *button == winit::event::MouseButton::Left {
                    if *state == winit::event::ElementState::Pressed && self.mouse_position.x >= 200.0 {
                        self.mouse_pressed = true;
                    } else {
                        self.mouse_pressed = false;
                    }
                }
            }
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                if let winit::event::MouseScrollDelta::LineDelta(_, change) = delta {
                    self.camera.distance -= change * self.camera.speed;
                    self.camera_changed = true;
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = *position;
            }
            winit::event::WindowEvent::DroppedFile(file_path) => {
                let molecule_structure = lib3dmol::parser::read_pdb(file_path.to_str().unwrap(), "");
                let mut atoms = Vec::new();
                for atom in molecule_structure.get_atom() {
                    atoms.push(glm::vec4(atom.coord[0], atom.coord[1], atom.coord[2], 0.0));
                }

                self.voxel_grid = VoxelGrid::new(&self.device, 1.0, atoms);
                self.raymarch_globals.bb_min = self.voxel_grid.bb_min.into();
                self.raymarch_globals.bb_max = self.voxel_grid.bb_max.into();
                self.raymarch_globals.bb_diff = self.voxel_grid.bb_diff.into();
                self.raymarch_globals.bb_size = self.voxel_grid.bb_size.into();
                self.raymarch_globals.voxel_length = self.voxel_grid.voxel_length;

                self.camera_changed = true;
            }
            _ => {}
        };
    }

    pub fn device_event(&mut self, event: &winit::event::DeviceEvent) {
        match event {
            winit::event::DeviceEvent::MouseMotion { delta } => {
                if self.mouse_pressed {
                    self.camera.yaw += delta.0 as f32;
                    self.camera.pitch += delta.1 as f32;
                    self.camera_changed = true;
                }
            }
            _ => {}
        };
    }

    ///
    /// Returns reference to the device used by the application.
    ///
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    ///
    /// Returns reference to the device used by the application.
    ///
    pub fn queue_mut(&self) -> &wgpu::Queue {
        &self.queue
    }

    fn update_raymarch_globals(&mut self) {
        self.raymarch_globals_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&[self.raymarch_globals]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );
    }

    pub fn solvent_radius(&self) -> f32 {
        self.raymarch_globals.solvent_radius
    }

    pub fn set_solvent_radius(&mut self, solvent_radius: f32) {
        self.raymarch_globals.solvent_radius = solvent_radius;
        self.update_raymarch_globals();
        self.camera_changed = true;
    }

    pub fn max_neighbours(&self) -> i32 {
        self.raymarch_globals.max_neighbours
    }

    pub fn set_max_neighbours(&mut self, max_neighbours: i32) {
        self.raymarch_globals.max_neighbours = max_neighbours;
        self.update_raymarch_globals();
        self.camera_changed = true;
    }

    pub fn max_steps(&self) -> i32 {
        self.raymarch_globals.max_steps
    }

    pub fn set_max_steps(&mut self, max_steps: i32) {
        self.raymarch_globals.max_steps = max_steps;
        self.update_raymarch_globals();
        self.camera_changed = true;
    }
}

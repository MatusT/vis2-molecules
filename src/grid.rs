//!
//! Module containing everything related to the voxel grid that is used as acceleration structure
//! for sphere marching.
//!

use crate::utils::*;
use nalgebra_glm as glm;
use wgpu;

///
/// Pointer to block of memory containing one grid cell of voxel grid.
///
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VoxelPointer {
    pub start: u32,
    pub length: u32,
}

unsafe impl bytemuck::Zeroable for VoxelPointer {}
unsafe impl bytemuck::Pod for VoxelPointer {}
///
/// Voxel grid. Contains information about AABB of the scene and GPU buffers containing the voxel grid in flat format for GPU.
///
pub struct VoxelGrid {
    pub bb_min: glm::Vec3,
    pub bb_max: glm::Vec3,
    pub bb_diff: glm::Vec3,
    pub bb_size: glm::Vec3,
    pub voxel_length: f32,

    pub voxels: wgpu::Buffer,
    pub voxels_len: usize,
    pub voxel_pointers: wgpu::Buffer,
    pub voxel_pointers_len: usize,
}

impl VoxelGrid {
    ///
    /// Initializes the voxel grid. Requires
    ///
    pub fn new(device: &wgpu::Device, radius_max: f32, mut atoms: Vec<glm::Vec4>) -> Self {
        // Calculate voxel length
        let solvent_radius_max = 2.0;
        let voxel_length = 2.0 * radius_max + 2.0 * solvent_radius_max;

        // Find bounding box of the entire structure
        let mut bb_max = glm::vec3(std::f32::NEG_INFINITY, std::f32::NEG_INFINITY, std::f32::NEG_INFINITY);
        let mut bb_min = glm::vec3(std::f32::INFINITY, std::f32::INFINITY, std::f32::INFINITY);
        for atom in atoms.iter() {
            bb_max = glm::max2(&bb_max, &glm::vec4_to_vec3(atom));
            bb_min = glm::min2(&bb_min, &glm::vec4_to_vec3(atom));
        }
        bb_min -= glm::vec3(1.0, 1.0, 1.0);
        bb_max += glm::vec3(1.0, 1.0, 1.0);
        let bb_center = (bb_max + bb_min) / 2.0;

        // Center the molecules (+their bounding box)
        bb_max = bb_max - bb_center;
        bb_min = bb_min - bb_center;
        for atom in atoms.iter_mut() {
            atom.x -= bb_center.x;
            atom.y -= bb_center.y;
            atom.z -= bb_center.z;
        }

        // Pad the bounding box to size divisible by voxel length
        bb_max.apply(|e| e.round_to_multiple(voxel_length as i32));
        bb_min.apply(|e| e.round_to_multiple(voxel_length as i32));

        let bb_diff = bb_max - bb_min;
        let bb_size = bb_diff.apply_into(|e| e.abs() / voxel_length as f32);

        let mut voxels_nested: Vec<Vec<glm::Vec4>> = vec![Vec::new(); (bb_size.x * bb_size.y * bb_size.z) as usize];

        for atom in atoms.iter() {
            let grid_position_vec3 = (atom.xyz() - bb_min) / voxel_length;
            let grid_position_ivec3 = glm::vec3(
                grid_position_vec3.x as i32,
                grid_position_vec3.y as i32,
                grid_position_vec3.z as i32,
            );

            let bb_size = glm::vec3(bb_size.x as i32, bb_size.y as i32, bb_size.z as i32);
            let width = bb_size.x;
            let height = bb_size.y;
            let x = grid_position_ivec3.x;
            let y = grid_position_ivec3.y;
            let z = grid_position_ivec3.z;
            let index = (width * height * z) + (width * y) + x;

            voxels_nested[index as usize].push(glm::vec4(atom.x, atom.y, atom.z, 1.0));
        }

        let mut voxels: Vec<f32> = Vec::new();
        let mut voxel_pointers = Vec::new();
        let mut count = 0;
        for voxel in voxels_nested.iter_mut() {
            voxel_pointers.push(VoxelPointer {
                start: count,
                length: voxel.len() as u32,
            });
            count += voxel.len() as u32;

            for v in voxel {
                voxels.push(v[0]);
                voxels.push(v[1]);
                voxels.push(v[2]);
                voxels.push(v[3]);
            }
        }

        let voxels_len = voxels.len() as usize;
        let voxels = device.create_buffer_with_data(bytemuck::cast_slice(&voxels), wgpu::BufferUsage::STORAGE_READ);

        let voxel_pointers_len = voxel_pointers.len() as usize;
        let voxel_pointers = device.create_buffer_with_data(bytemuck::cast_slice(&voxel_pointers), wgpu::BufferUsage::STORAGE_READ);

        Self {
            bb_min,
            bb_max,
            bb_diff,
            bb_size,
            voxel_length,

            voxels,
            voxels_len,
            voxel_pointers,
            voxel_pointers_len,
        }
    }
}

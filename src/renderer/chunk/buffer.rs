use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ash::vk;
use azalea_core::position::ChunkPos;
use gpu_allocator::vulkan::{Allocation, Allocator};

use super::mesher::{ChunkMeshData, ChunkVertex};
use crate::renderer::shader;
use crate::renderer::util;
use crate::renderer::MAX_FRAMES_IN_FLIGHT;

pub const MAX_CHUNKS: usize = 8192;
const INITIAL_VERTICES: u32 = 4_000_000;
const INITIAL_INDICES: u32 = 6_000_000;
const VERTEX_SIZE: usize = std::mem::size_of::<ChunkVertex>();
const INDEX_SIZE: usize = std::mem::size_of::<u32>();

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkAABB {
    pub min: [f32; 4],
    pub max: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkMeta {
    aabb_min: [f32; 4],
    aabb_max: [f32; 4],
    index_count: u32,
    first_index: u32,
    vertex_offset: i32,
    _pad: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawCommand {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    vertex_offset: i32,
    first_instance: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FrustumData {
    planes: [[f32; 4]; 6],
    chunk_count: u32,
    _pad: [u32; 3],
}

struct ChunkSlot {
    vertex_offset: u32,
    vertex_count: u32,
    index_offset: u32,
    index_count: u32,
    aabb: ChunkAABB,
}

struct PendingFree {
    vertex_offset: u32,
    vertex_count: u32,
    index_offset: u32,
    index_count: u32,
    frame_retired: u64,
}

struct FreeList {
    free: Vec<(u32, u32)>,
}

impl FreeList {
    fn new(capacity: u32) -> Self {
        Self {
            free: vec![(0, capacity)],
        }
    }

    fn alloc(&mut self, count: u32) -> Option<u32> {
        for i in 0..self.free.len() {
            if self.free[i].1 >= count {
                let offset = self.free[i].0;
                if self.free[i].1 == count {
                    self.free.remove(i);
                } else {
                    self.free[i].0 += count;
                    self.free[i].1 -= count;
                }
                return Some(offset);
            }
        }
        None
    }

    fn free(&mut self, offset: u32, count: u32) {
        let pos = self.free.partition_point(|&(o, _)| o < offset);
        self.free.insert(pos, (offset, count));
        if pos + 1 < self.free.len() && self.free[pos].0 + self.free[pos].1 == self.free[pos + 1].0
        {
            self.free[pos].1 += self.free[pos + 1].1;
            self.free.remove(pos + 1);
        }
        if pos > 0 && self.free[pos - 1].0 + self.free[pos - 1].1 == self.free[pos].0 {
            self.free[pos - 1].1 += self.free[pos].1;
            self.free.remove(pos);
        }
    }
}

pub struct ChunkBufferStore {
    vertex_buffer: vk::Buffer,
    vertex_allocation: Allocation,
    index_buffer: vk::Buffer,
    index_allocation: Allocation,
    vertex_free: FreeList,
    index_free: FreeList,

    chunks: HashMap<ChunkPos, ChunkSlot>,
    pending_frees: Vec<PendingFree>,
    global_frame: u64,

    compute_pipeline: vk::Pipeline,
    compute_layout: vk::PipelineLayout,
    compute_descriptor_layout: vk::DescriptorSetLayout,
    compute_pool: vk::DescriptorPool,
    compute_sets: Vec<vk::DescriptorSet>,

    metadata_buffers: Vec<vk::Buffer>,
    metadata_allocations: Vec<Allocation>,
    indirect_buffers: Vec<vk::Buffer>,
    indirect_allocations: Vec<Allocation>,
    draw_count_buffers: Vec<vk::Buffer>,
    draw_count_allocations: Vec<Allocation>,
    frustum_buffers: Vec<vk::Buffer>,
    frustum_allocations: Vec<Allocation>,
}

impl ChunkBufferStore {
    pub fn new(device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) -> Self {
        let (vertex_buffer, vertex_allocation) = util::create_host_buffer(
            device,
            allocator,
            INITIAL_VERTICES as u64 * VERTEX_SIZE as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            "vertex_mega",
        );
        let (index_buffer, index_allocation) = util::create_host_buffer(
            device,
            allocator,
            INITIAL_INDICES as u64 * INDEX_SIZE as u64,
            vk::BufferUsageFlags::INDEX_BUFFER,
            "index_mega",
        );

        let meta_size = (MAX_CHUNKS * std::mem::size_of::<ChunkMeta>()) as u64;
        let indirect_size = (MAX_CHUNKS * std::mem::size_of::<DrawCommand>()) as u64;
        let count_size = std::mem::size_of::<u32>() as u64;
        let frustum_size = std::mem::size_of::<FrustumData>() as u64;

        let mut metadata_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut metadata_allocations = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut indirect_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut indirect_allocations = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut draw_count_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut draw_count_allocations = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut frustum_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut frustum_allocations = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let (b, a) = util::create_host_buffer(
                device,
                allocator,
                meta_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                "chunk_meta",
            );
            metadata_buffers.push(b);
            metadata_allocations.push(a);

            let (b, a) = util::create_host_buffer(
                device,
                allocator,
                indirect_size,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::INDIRECT_BUFFER,
                "indirect_cmds",
            );
            indirect_buffers.push(b);
            indirect_allocations.push(a);

            let (b, a) = util::create_host_buffer(
                device,
                allocator,
                count_size,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::INDIRECT_BUFFER,
                "draw_count",
            );
            draw_count_buffers.push(b);
            draw_count_allocations.push(a);

            let (b, a) = util::create_host_buffer(
                device,
                allocator,
                frustum_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                "frustum_ubo",
            );
            frustum_buffers.push(b);
            frustum_allocations.push(a);
        }

        let compute_descriptor_layout = create_cull_descriptor_layout(device);
        let set_layouts = [compute_descriptor_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
        let compute_layout = unsafe { device.create_pipeline_layout(&layout_info, None) }
            .expect("failed to create compute pipeline layout");

        let comp_spv = shader::include_spirv!("cull.comp.spv");
        let comp_module = shader::create_shader_module(device, comp_spv);
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(comp_module)
            .name(c"main");
        let pipeline_info = [vk::ComputePipelineCreateInfo::default()
            .stage(stage)
            .layout(compute_layout)];
        let compute_pipeline = unsafe {
            device.create_compute_pipelines(vk::PipelineCache::null(), &pipeline_info, None)
        }
        .expect("failed to create cull compute pipeline")[0];
        unsafe { device.destroy_shader_module(comp_module, None) };

        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 3 * MAX_FRAMES_IN_FLIGHT as u32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
            },
        ];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(MAX_FRAMES_IN_FLIGHT as u32)
            .pool_sizes(&pool_sizes);
        let compute_pool = unsafe { device.create_descriptor_pool(&pool_info, None) }
            .expect("failed to create cull descriptor pool");

        let layouts: Vec<_> = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| compute_descriptor_layout)
            .collect();
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(compute_pool)
            .set_layouts(&layouts);
        let compute_sets = unsafe { device.allocate_descriptor_sets(&alloc_info) }
            .expect("failed to allocate cull descriptor sets");

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let meta_info = [vk::DescriptorBufferInfo {
                buffer: metadata_buffers[i],
                offset: 0,
                range: meta_size,
            }];
            let frustum_info = [vk::DescriptorBufferInfo {
                buffer: frustum_buffers[i],
                offset: 0,
                range: frustum_size,
            }];
            let indirect_info = [vk::DescriptorBufferInfo {
                buffer: indirect_buffers[i],
                offset: 0,
                range: indirect_size,
            }];
            let count_info = [vk::DescriptorBufferInfo {
                buffer: draw_count_buffers[i],
                offset: 0,
                range: count_size,
            }];
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(compute_sets[i])
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&meta_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(compute_sets[i])
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&frustum_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(compute_sets[i])
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&indirect_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(compute_sets[i])
                    .dst_binding(3)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&count_info),
            ];
            unsafe { device.update_descriptor_sets(&writes, &[]) };
        }

        Self {
            vertex_buffer,
            vertex_allocation,
            index_buffer,
            index_allocation,
            vertex_free: FreeList::new(INITIAL_VERTICES),
            index_free: FreeList::new(INITIAL_INDICES),
            chunks: HashMap::new(),
            pending_frees: Vec::new(),
            global_frame: 0,
            compute_pipeline,
            compute_layout,
            compute_descriptor_layout,
            compute_pool,
            compute_sets,
            metadata_buffers,
            metadata_allocations,
            indirect_buffers,
            indirect_allocations,
            draw_count_buffers,
            draw_count_allocations,
            frustum_buffers,
            frustum_allocations,
        }
    }

    pub fn begin_frame(&mut self) {
        self.global_frame += 1;
        let safe_frame = self
            .global_frame
            .saturating_sub(MAX_FRAMES_IN_FLIGHT as u64 + 1);
        let mut i = 0;
        while i < self.pending_frees.len() {
            if self.pending_frees[i].frame_retired <= safe_frame {
                let pf = self.pending_frees.swap_remove(i);
                self.vertex_free.free(pf.vertex_offset, pf.vertex_count);
                self.index_free.free(pf.index_offset, pf.index_count);
            } else {
                i += 1;
            }
        }
    }

    pub fn upload(&mut self, mesh: &ChunkMeshData) {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            self.remove(&mesh.pos);
            return;
        }

        self.remove(&mesh.pos);

        let v_count = mesh.vertices.len() as u32;
        let i_count = mesh.indices.len() as u32;

        let vertex_offset = match self.vertex_free.alloc(v_count) {
            Some(o) => o,
            None => {
                log::warn!("Vertex mega-buffer full, skipping chunk {:?}", mesh.pos);
                return;
            }
        };
        let index_offset = match self.index_free.alloc(i_count) {
            Some(o) => o,
            None => {
                self.vertex_free.free(vertex_offset, v_count);
                log::warn!("Index mega-buffer full, skipping chunk {:?}", mesh.pos);
                return;
            }
        };

        let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
        let vb_byte_offset = vertex_offset as usize * VERTEX_SIZE;
        self.vertex_allocation.mapped_slice_mut().unwrap()
            [vb_byte_offset..vb_byte_offset + vertex_bytes.len()]
            .copy_from_slice(vertex_bytes);

        let index_bytes = bytemuck::cast_slice(&mesh.indices);
        let ib_byte_offset = index_offset as usize * INDEX_SIZE;
        self.index_allocation.mapped_slice_mut().unwrap()
            [ib_byte_offset..ib_byte_offset + index_bytes.len()]
            .copy_from_slice(index_bytes);

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for v in &mesh.vertices {
            min_y = min_y.min(v.position[1]);
            max_y = max_y.max(v.position[1]);
        }
        let cx = mesh.pos.x as f32 * 16.0;
        let cz = mesh.pos.z as f32 * 16.0;

        self.chunks.insert(
            mesh.pos,
            ChunkSlot {
                vertex_offset,
                vertex_count: v_count,
                index_offset,
                index_count: i_count,
                aabb: ChunkAABB {
                    min: [cx, min_y, cz, 0.0],
                    max: [cx + 16.0, max_y, cz + 16.0, 0.0],
                },
            },
        );
    }

    pub fn remove(&mut self, pos: &ChunkPos) {
        if let Some(slot) = self.chunks.remove(pos) {
            self.pending_frees.push(PendingFree {
                vertex_offset: slot.vertex_offset,
                vertex_count: slot.vertex_count,
                index_offset: slot.index_offset,
                index_count: slot.index_count,
                frame_retired: self.global_frame,
            });
        }
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.pending_frees.clear();
        self.vertex_free = FreeList::new(INITIAL_VERTICES);
        self.index_free = FreeList::new(INITIAL_INDICES);
    }

    pub fn chunk_count(&self) -> u32 {
        self.chunks.len().min(MAX_CHUNKS) as u32
    }

    pub fn dispatch_cull(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: usize,
        frustum: &[[f32; 4]; 6],
        camera_pos: [f32; 3],
    ) {
        let count = self.chunks.len().min(MAX_CHUNKS) as u32;
        if count == 0 {
            return;
        }

        let mut entries: Vec<ChunkMeta> = self
            .chunks
            .values()
            .map(|s| ChunkMeta {
                aabb_min: s.aabb.min,
                aabb_max: s.aabb.max,
                index_count: s.index_count,
                first_index: s.index_offset,
                vertex_offset: s.vertex_offset as i32,
                _pad: 0,
            })
            .collect();

        entries.sort_by(|a, b| {
            let da = chunk_dist_sq(a, camera_pos);
            let db = chunk_dist_sq(b, camera_pos);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

        let meta_bytes = bytemuck::cast_slice(&entries);
        self.metadata_allocations[frame].mapped_slice_mut().unwrap()[..meta_bytes.len()]
            .copy_from_slice(meta_bytes);

        let frustum_data = FrustumData {
            planes: *frustum,
            chunk_count: count,
            _pad: [0; 3],
        };
        let frustum_bytes = bytemuck::bytes_of(&frustum_data);
        self.frustum_allocations[frame].mapped_slice_mut().unwrap()[..frustum_bytes.len()]
            .copy_from_slice(frustum_bytes);

        self.draw_count_allocations[frame]
            .mapped_slice_mut()
            .unwrap()[..4]
            .copy_from_slice(&0u32.to_ne_bytes());

        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, self.compute_pipeline);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_layout,
                0,
                &[self.compute_sets[frame]],
                &[],
            );

            let groups = count.div_ceil(64);
            device.cmd_dispatch(cmd, groups, 1, 1);

            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::INDIRECT_COMMAND_READ);
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::DRAW_INDIRECT,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );
        }
    }

    pub fn draw_indirect(&self, device: &ash::Device, cmd: vk::CommandBuffer, frame: usize) {
        if self.chunks.is_empty() {
            return;
        }

        let max_draws = self.chunks.len().min(MAX_CHUNKS) as u32;

        unsafe {
            device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer], &[0]);
            device.cmd_bind_index_buffer(cmd, self.index_buffer, 0, vk::IndexType::UINT32);

            device.cmd_draw_indexed_indirect_count(
                cmd,
                self.indirect_buffers[frame],
                0,
                self.draw_count_buffers[frame],
                0,
                max_draws,
                std::mem::size_of::<DrawCommand>() as u32,
            );
        }
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        let mut alloc = allocator.lock().unwrap();

        unsafe {
            device.destroy_buffer(self.vertex_buffer, None);
            device.destroy_buffer(self.index_buffer, None);
        }
        alloc
            .free(std::mem::replace(&mut self.vertex_allocation, unsafe {
                std::mem::zeroed()
            }))
            .ok();
        alloc
            .free(std::mem::replace(&mut self.index_allocation, unsafe {
                std::mem::zeroed()
            }))
            .ok();

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            unsafe {
                device.destroy_buffer(self.metadata_buffers[i], None);
                device.destroy_buffer(self.indirect_buffers[i], None);
                device.destroy_buffer(self.draw_count_buffers[i], None);
                device.destroy_buffer(self.frustum_buffers[i], None);
            }
            alloc
                .free(std::mem::replace(
                    &mut self.metadata_allocations[i],
                    unsafe { std::mem::zeroed() },
                ))
                .ok();
            alloc
                .free(std::mem::replace(
                    &mut self.indirect_allocations[i],
                    unsafe { std::mem::zeroed() },
                ))
                .ok();
            alloc
                .free(std::mem::replace(
                    &mut self.draw_count_allocations[i],
                    unsafe { std::mem::zeroed() },
                ))
                .ok();
            alloc
                .free(std::mem::replace(
                    &mut self.frustum_allocations[i],
                    unsafe { std::mem::zeroed() },
                ))
                .ok();
        }

        drop(alloc);

        unsafe {
            device.destroy_pipeline(self.compute_pipeline, None);
            device.destroy_pipeline_layout(self.compute_layout, None);
            device.destroy_descriptor_pool(self.compute_pool, None);
            device.destroy_descriptor_set_layout(self.compute_descriptor_layout, None);
        }
    }
}

fn chunk_dist_sq(meta: &ChunkMeta, cam: [f32; 3]) -> f32 {
    let cx = (meta.aabb_min[0] + meta.aabb_max[0]) * 0.5;
    let cy = (meta.aabb_min[1] + meta.aabb_max[1]) * 0.5;
    let cz = (meta.aabb_min[2] + meta.aabb_max[2]) * 0.5;
    let dx = cx - cam[0];
    let dy = cy - cam[1];
    let dz = cz - cam[2];
    dx * dx + dy * dy + dz * dz
}

fn create_cull_descriptor_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
    let bindings = [
        vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            ..Default::default()
        },
        vk::DescriptorSetLayoutBinding {
            binding: 1,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            ..Default::default()
        },
        vk::DescriptorSetLayoutBinding {
            binding: 2,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            ..Default::default()
        },
        vk::DescriptorSetLayoutBinding {
            binding: 3,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            ..Default::default()
        },
    ];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    unsafe { device.create_descriptor_set_layout(&info, None) }
        .expect("failed to create cull descriptor set layout")
}

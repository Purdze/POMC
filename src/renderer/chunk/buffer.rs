use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ash::vk;
use azalea_core::position::ChunkPos;
use gpu_allocator::vulkan::{Allocation, Allocator};

use super::mesher::ChunkMeshData;
use crate::renderer::util;

const INITIAL_VERTEX_CAPACITY: u64 = 128 * 1024 * 1024;
const INITIAL_INDEX_CAPACITY: u64 = 32 * 1024 * 1024;
const VERTEX_STRIDE: u64 = std::mem::size_of::<super::mesher::ChunkVertex>() as u64;
const INDEX_STRIDE: u64 = 4;
pub const MAX_CHUNKS: usize = 8192;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawIndexedIndirectCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkAABB {
    pub min: [f32; 4],
    pub max: [f32; 4],
}

struct ChunkSlot {
    vertex_offset: u64,
    vertex_size: u64,
    index_offset: u64,
    index_size: u64,
    index_count: u32,
    aabb: ChunkAABB,
}

struct FreeBlock {
    offset: u64,
    size: u64,
}

struct MegaBuffer {
    buffer: vk::Buffer,
    allocation: Allocation,
    capacity: u64,
    free_list: Vec<FreeBlock>,
}

impl MegaBuffer {
    fn new(
        device: &ash::Device,
        allocator: &Arc<Mutex<Allocator>>,
        capacity: u64,
        usage: vk::BufferUsageFlags,
        name: &str,
    ) -> Self {
        let (buffer, allocation) =
            util::create_host_buffer(device, allocator, capacity, usage, name);
        Self {
            buffer,
            allocation,
            capacity,
            free_list: vec![FreeBlock {
                offset: 0,
                size: capacity,
            }],
        }
    }

    fn alloc(&mut self, size: u64, align: u64) -> Option<u64> {
        for i in 0..self.free_list.len() {
            let block_offset = self.free_list[i].offset;
            let block_size = self.free_list[i].size;
            let aligned = (block_offset + align - 1) & !(align - 1);
            let padding = aligned - block_offset;
            if block_size < size + padding {
                continue;
            }

            let result = aligned;
            let remaining = block_size - size - padding;

            if remaining > 0 {
                self.free_list[i] = FreeBlock {
                    offset: result + size,
                    size: remaining,
                };
            } else {
                self.free_list.remove(i);
            }

            if padding > 0 {
                let pos = self.free_list.partition_point(|b| b.offset < block_offset);
                self.free_list.insert(
                    pos,
                    FreeBlock {
                        offset: block_offset,
                        size: padding,
                    },
                );
            }

            return Some(result);
        }
        None
    }

    fn free(&mut self, offset: u64, size: u64) {
        let pos = self.free_list.partition_point(|b| b.offset < offset);
        self.free_list.insert(pos, FreeBlock { offset, size });

        if pos + 1 < self.free_list.len()
            && self.free_list[pos].offset + self.free_list[pos].size
                == self.free_list[pos + 1].offset
        {
            self.free_list[pos].size += self.free_list[pos + 1].size;
            self.free_list.remove(pos + 1);
        }

        if pos > 0
            && self.free_list[pos - 1].offset + self.free_list[pos - 1].size
                == self.free_list[pos].offset
        {
            self.free_list[pos - 1].size += self.free_list[pos].size;
            self.free_list.remove(pos);
        }
    }

    fn write(&mut self, offset: u64, data: &[u8]) {
        let slice = self.allocation.mapped_slice_mut().unwrap();
        slice[offset as usize..offset as usize + data.len()].copy_from_slice(data);
    }

    fn reset(&mut self) {
        self.free_list.clear();
        self.free_list.push(FreeBlock {
            offset: 0,
            size: self.capacity,
        });
    }

    fn destroy(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        unsafe { device.destroy_buffer(self.buffer, None) };
        let alloc = std::mem::replace(&mut self.allocation, unsafe { std::mem::zeroed() });
        allocator.lock().unwrap().free(alloc).ok();
    }
}

pub struct ChunkBufferStore {
    vertex_mega: MegaBuffer,
    index_mega: MegaBuffer,
    slots: HashMap<ChunkPos, ChunkSlot>,
}

impl ChunkBufferStore {
    pub fn new(device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) -> Self {
        Self {
            vertex_mega: MegaBuffer::new(
                device,
                allocator,
                INITIAL_VERTEX_CAPACITY,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                "vertex_mega",
            ),
            index_mega: MegaBuffer::new(
                device,
                allocator,
                INITIAL_INDEX_CAPACITY,
                vk::BufferUsageFlags::INDEX_BUFFER,
                "index_mega",
            ),
            slots: HashMap::new(),
        }
    }

    pub fn upload(&mut self, mesh: &ChunkMeshData) {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            self.remove(&mesh.pos);
            return;
        }

        self.remove(&mesh.pos);

        let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
        let index_bytes = bytemuck::cast_slice(&mesh.indices);

        let vertex_offset = self
            .vertex_mega
            .alloc(vertex_bytes.len() as u64, VERTEX_STRIDE)
            .expect("vertex mega-buffer full");
        let index_offset = self
            .index_mega
            .alloc(index_bytes.len() as u64, INDEX_STRIDE)
            .expect("index mega-buffer full");

        self.vertex_mega.write(vertex_offset, vertex_bytes);
        self.index_mega.write(index_offset, index_bytes);

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for v in &mesh.vertices {
            min_y = min_y.min(v.position[1]);
            max_y = max_y.max(v.position[1]);
        }

        let cx = mesh.pos.x as f32 * 16.0;
        let cz = mesh.pos.z as f32 * 16.0;

        self.slots.insert(
            mesh.pos,
            ChunkSlot {
                vertex_offset,
                vertex_size: vertex_bytes.len() as u64,
                index_offset,
                index_size: index_bytes.len() as u64,
                index_count: mesh.indices.len() as u32,
                aabb: ChunkAABB {
                    min: [cx, min_y, cz, 0.0],
                    max: [cx + 16.0, max_y, cz + 16.0, 0.0],
                },
            },
        );
    }

    pub fn remove(&mut self, pos: &ChunkPos) {
        if let Some(slot) = self.slots.remove(pos) {
            self.vertex_mega.free(slot.vertex_offset, slot.vertex_size);
            self.index_mega.free(slot.index_offset, slot.index_size);
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.vertex_mega.reset();
        self.index_mega.reset();
    }

    pub fn vertex_buffer(&self) -> vk::Buffer {
        self.vertex_mega.buffer
    }

    pub fn index_buffer(&self) -> vk::Buffer {
        self.index_mega.buffer
    }

    pub fn chunk_count(&self) -> u32 {
        self.slots.len().min(MAX_CHUNKS) as u32
    }

    pub fn write_draw_data(
        &self,
        commands: &mut [DrawIndexedIndirectCommand],
        aabbs: &mut [ChunkAABB],
    ) -> u32 {
        let count = self.slots.len().min(MAX_CHUNKS);
        for (i, slot) in self.slots.values().take(count).enumerate() {
            commands[i] = DrawIndexedIndirectCommand {
                index_count: slot.index_count,
                instance_count: 1,
                first_index: (slot.index_offset / INDEX_STRIDE) as u32,
                vertex_offset: (slot.vertex_offset / VERTEX_STRIDE) as i32,
                first_instance: 0,
            };
            aabbs[i] = slot.aabb;
        }
        count as u32
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        self.slots.clear();
        self.vertex_mega.destroy(device, allocator);
        self.index_mega.destroy(device, allocator);
    }
}

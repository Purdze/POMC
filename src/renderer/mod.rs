pub mod camera;
pub mod chunk;
mod context;
mod pipelines;

use std::path::Path;
use std::sync::Arc;

use azalea_core::position::ChunkPos;
use thiserror::Error;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use camera::{Camera, CameraUniform};
use chunk::atlas::TextureAtlas;
use chunk::mesher::ChunkMeshData;
use context::GpuContext;
use pipelines::chunk::ChunkPipeline;

use crate::window::input::InputState;
use crate::world::block::registry::BlockRegistry;
use crate::world::chunk::ChunkStore;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("failed to initialize GPU context: {0}")]
    Context(#[from] context::ContextError),

    #[error("surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),

    #[error("atlas error: {0}")]
    Atlas(#[from] chunk::atlas::AtlasError),
}

pub struct Renderer {
    ctx: GpuContext,
    camera: Camera,
    chunk_pipeline: ChunkPipeline,
    depth_view: wgpu::TextureView,
    atlas: TextureAtlas,
    registry: BlockRegistry,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
}

impl Renderer {
    pub fn new(window: Arc<Window>, assets_dir: &Path) -> Result<Self, RendererError> {
        let ctx = pollster::block_on(GpuContext::new(Arc::clone(&window)))?;
        let aspect = ctx.config.width as f32 / ctx.config.height as f32;
        let camera = Camera::new(aspect);

        let registry = BlockRegistry::new();
        let texture_names: std::collections::HashSet<&str> = registry.texture_names().collect();
        let atlas = TextureAtlas::build(&ctx.device, &ctx.queue, assets_dir, &texture_names)?;

        let chunk_pipeline = ChunkPipeline::new(&ctx.device, ctx.config.format, &atlas);
        let depth_view = create_depth_view(&ctx.device, ctx.config.width, ctx.config.height);

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            None,
            None,
            None,
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(&ctx.device, ctx.config.format, None, 1, false);

        Ok(Self {
            ctx,
            camera,
            chunk_pipeline,
            depth_view,
            atlas,
            registry,
            egui_renderer,
            egui_state,
            egui_ctx,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.ctx.resize(new_size);
        if new_size.width > 0 && new_size.height > 0 {
            self.depth_view = create_depth_view(&self.ctx.device, new_size.width, new_size.height);
            self.camera
                .set_aspect_ratio(new_size.width as f32 / new_size.height as f32);
        }
    }

    pub fn handle_window_event(
        &mut self,
        window: &Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.egui_state.on_window_event(window, event)
    }

    pub fn update_camera(&mut self, input: &mut InputState) {
        self.camera.update_look(input);
        let uniform = CameraUniform::from_camera(&self.camera);
        self.chunk_pipeline.update_camera(&self.ctx.queue, &uniform);
    }

    pub fn sync_camera_to_player(&mut self, eye_pos: glam::Vec3, yaw: f32, pitch: f32) {
        self.camera.position = eye_pos;
        self.camera.yaw = yaw;
        self.camera.pitch = pitch;
        let uniform = CameraUniform::from_camera(&self.camera);
        self.chunk_pipeline.update_camera(&self.ctx.queue, &uniform);
    }

    pub fn camera_yaw(&self) -> f32 {
        self.camera.yaw
    }

    pub fn camera_pitch(&self) -> f32 {
        self.camera.pitch
    }

    pub fn set_camera_position(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        self.camera
            .set_position(glam::Vec3::new(x as f32, y as f32, z as f32), yaw, pitch);
    }

    pub fn upload_chunk_mesh(&mut self, mesh: &ChunkMeshData) {
        self.chunk_pipeline.upload_mesh(&self.ctx.device, mesh);
    }

    pub fn remove_chunk_mesh(&mut self, pos: &ChunkPos) {
        self.chunk_pipeline.remove_mesh(pos);
    }

    pub fn mesh_chunk(&self, chunk_store: &ChunkStore, pos: ChunkPos) -> ChunkMeshData {
        chunk::mesher::mesh_chunk(chunk_store, pos, &self.registry, &self.atlas)
    }

    pub fn render_world(&mut self) -> Result<(), RendererError> {
        let output = self.ctx.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.529,
                            g: 0.808,
                            b: 0.922,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.chunk_pipeline.draw(&mut render_pass);
        }

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn render_ui(
        &mut self,
        window: &Window,
        ui_fn: impl FnMut(&egui::Context),
    ) -> Result<(), RendererError> {
        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, ui_fn);

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.ctx.device, &self.ctx.queue, *id, delta);
        }

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.ctx.config.width, self.ctx.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        let output = self.ctx.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("egui_encoder"),
            });

        let commands = self.egui_renderer.update_buffers(
            &self.ctx.device,
            &self.ctx.queue,
            &mut encoder,
            &tris,
            &screen,
        );

        {
            let mut render_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();

            self.egui_renderer.render(&mut render_pass, &tris, &screen);
        }

        let mut submit: Vec<wgpu::CommandBuffer> = commands;
        submit.push(encoder.finish());
        self.ctx.queue.submit(submit);
        output.present();

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        Ok(())
    }
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

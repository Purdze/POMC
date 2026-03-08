pub mod input;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::net::NetworkEvent;
use crate::physics::movement;
use crate::player::LocalPlayer;
use crate::renderer::Renderer;
use crate::ui::menu::{MainMenu, MenuAction};
use crate::world::chunk::ChunkStore;
use input::InputState;

#[derive(Error, Debug)]
pub enum WindowError {
    #[error("failed to create event loop: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    #[error("failed to create window: {0}")]
    CreateWindow(#[from] winit::error::OsError),

    #[error("renderer error: {0}")]
    Renderer(#[from] crate::renderer::RendererError),
}

enum GameState {
    Menu,
    InGame,
}

const TICK_RATE: f32 = 1.0 / 20.0;

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    input: InputState,
    last_frame: Option<Instant>,
    net_events: Option<crossbeam_channel::Receiver<NetworkEvent>>,
    chunk_store: ChunkStore,
    assets_dir: PathBuf,
    position_set: bool,
    state: GameState,
    menu: MainMenu,
    tokio_rt: Arc<tokio::runtime::Runtime>,
    player: LocalPlayer,
    tick_accumulator: f32,
    prev_player_pos: glam::Vec3,
}

impl App {
    fn new(
        net_events: Option<crossbeam_channel::Receiver<NetworkEvent>>,
        assets_dir: PathBuf,
        tokio_rt: Arc<tokio::runtime::Runtime>,
    ) -> Self {
        let state = if net_events.is_some() {
            GameState::InGame
        } else {
            GameState::Menu
        };

        Self {
            window: None,
            renderer: None,
            input: InputState::new(),
            last_frame: None,
            net_events,
            chunk_store: ChunkStore::new(8),
            assets_dir,
            position_set: false,
            state,
            menu: MainMenu::new(),
            tokio_rt,
            player: LocalPlayer::new(),
            tick_accumulator: 0.0,
            prev_player_pos: glam::Vec3::ZERO,
        }
    }

    fn apply_cursor_grab(&self) {
        let Some(window) = &self.window else { return };
        let captured = matches!(self.state, GameState::InGame) && self.input.is_cursor_captured();
        if captured {
            let _ = window.set_cursor_grab(CursorGrabMode::Confined);
            window.set_cursor_visible(false);
        } else {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
        }
    }

    fn connect_to_server(&mut self, server: String, username: String) {
        let connect_args = crate::net::connection::ConnectArgs {
            server,
            username,
            uuid: uuid::Uuid::nil(),
            access_token: None,
        };

        self.net_events = Some(crate::net::connection::spawn_connection(
            &self.tokio_rt,
            connect_args,
        ));
        self.state = GameState::InGame;
        self.apply_cursor_grab();
    }

    fn drain_network_events(&mut self) {
        let Some(rx) = &self.net_events else { return };
        let mut chunks_to_mesh = Vec::new();

        while let Ok(event) = rx.try_recv() {
            match event {
                NetworkEvent::Connected => {
                    log::info!("Connected to server");
                }
                NetworkEvent::ChunkLoaded {
                    pos,
                    data,
                    heightmaps,
                } => {
                    if let Err(e) = self.chunk_store.load_chunk(pos, &data, &heightmaps) {
                        log::error!("Failed to load chunk [{}, {}]: {e}", pos.x, pos.z);
                        continue;
                    }
                    chunks_to_mesh.push(pos);
                }
                NetworkEvent::ChunkUnloaded { pos } => {
                    self.chunk_store.unload_chunk(&pos);
                    if let Some(renderer) = &mut self.renderer {
                        renderer.remove_chunk_mesh(&pos);
                    }
                }
                NetworkEvent::ChunkCacheCenter { x, z } => {
                    self.chunk_store
                        .set_center(azalea_core::position::ChunkPos::new(x, z));
                }
                NetworkEvent::PlayerPosition {
                    x,
                    y,
                    z,
                    yaw,
                    pitch,
                    ..
                } => {
                    if !self.position_set {
                        self.player.position = glam::Vec3::new(x as f32, y as f32, z as f32);
                        self.player.yaw = yaw.to_radians();
                        self.player.pitch = pitch.to_radians();
                        self.prev_player_pos = self.player.position;
                        if let Some(renderer) = &mut self.renderer {
                            renderer.set_camera_position(x, y, z, yaw, pitch);
                        }
                        self.position_set = true;
                        log::info!("Player position set to ({x:.1}, {y:.1}, {z:.1})");
                    }
                }
                NetworkEvent::Disconnected { reason } => {
                    log::warn!("Disconnected: {reason}");
                }
            }
        }

        if let Some(renderer) = &mut self.renderer {
            for pos in chunks_to_mesh {
                let mesh = renderer.mesh_chunk(&self.chunk_store, pos);
                renderer.upload_chunk_mesh(&mesh);
            }
        }
    }

    fn tick_physics(&mut self) {
        if let Some(renderer) = &self.renderer {
            self.player.yaw = renderer.camera_yaw();
            self.player.pitch = renderer.camera_pitch();
        }

        self.prev_player_pos = self.player.position;
        movement::tick(&mut self.player, &self.input, &self.chunk_store);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Ferrite")
            .with_inner_size(winit::dpi::LogicalSize::new(854, 480));

        let window = match event_loop.create_window(window_attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("Failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        let renderer = match Renderer::new(Arc::clone(&window), &self.assets_dir) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to create renderer: {e}");
                event_loop.exit();
                return;
            }
        };

        self.renderer = Some(renderer);
        self.window = Some(window);
        self.apply_cursor_grab();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(self.state, GameState::Menu) {
            if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                let response = renderer.handle_window_event(window, &event);
                if response.consumed && !matches!(event, WindowEvent::RedrawRequested) {
                    return;
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if matches!(self.state, GameState::InGame) {
                    if event.state.is_pressed() {
                        if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                            self.input.toggle_cursor_capture();
                            self.apply_cursor_grab();
                        }
                    }
                    self.input.on_key_event(&event);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = self
                    .last_frame
                    .map(|last| now.duration_since(last).as_secs_f32())
                    .unwrap_or(0.0)
                    .min(0.1);
                self.last_frame = Some(now);

                match self.state {
                    GameState::Menu => {
                        if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                            let menu = &mut self.menu;
                            let mut action = MenuAction::None;
                            if let Err(e) = renderer.render_ui(window, |ctx| {
                                action = menu.draw(ctx);
                            }) {
                                log::error!("Render error: {e}");
                            }

                            match action {
                                MenuAction::Connect { server, username } => {
                                    self.connect_to_server(server, username);
                                }
                                MenuAction::Quit => {
                                    event_loop.exit();
                                }
                                MenuAction::None => {}
                            }
                        }
                    }
                    GameState::InGame => {
                        self.drain_network_events();

                        if let Some(renderer) = &mut self.renderer {
                            renderer.update_camera(&mut self.input);
                        }

                        self.tick_accumulator += dt;
                        while self.tick_accumulator >= TICK_RATE {
                            self.tick_physics();
                            self.tick_accumulator -= TICK_RATE;
                        }

                        let alpha = self.tick_accumulator / TICK_RATE;
                        let interp_pos = self.prev_player_pos.lerp(self.player.position, alpha);
                        let eye_pos = interp_pos + glam::Vec3::new(0.0, 1.62, 0.0);

                        if let Some(renderer) = &mut self.renderer {
                            renderer.sync_camera_to_player(
                                eye_pos,
                                renderer.camera_yaw(),
                                renderer.camera_pitch(),
                            );

                            if let Err(e) = renderer.render_world() {
                                log::error!("Render error: {e}");
                            }
                        }
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.input.is_cursor_captured() {
                self.input.on_mouse_motion(delta);
            }
        }
    }
}

pub fn run(
    net_events: Option<crossbeam_channel::Receiver<NetworkEvent>>,
    assets_dir: PathBuf,
    tokio_rt: Arc<tokio::runtime::Runtime>,
) -> Result<(), WindowError> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new(net_events, assets_dir, tokio_rt);
    event_loop.run_app(&mut app)?;
    Ok(())
}

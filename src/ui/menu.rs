use std::path::Path;
use std::time::Instant;

use egui::{Color32, Pos2, Rect, Vec2};

use super::font::{mc_text, mc_text_width};
use super::hud::{gui_scale, mc_button, mc_button_w, mc_icon_button, HudTextures, BUTTON_GAP, UV_FULL};
use crate::assets::AssetIndex;

pub enum MenuAction {
    None,
    Connect { server: String, username: String },
    Quit,
}

const LABEL_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
const LOGO_WIDTH: f32 = 256.0;
const EDITION_WIDTH: f32 = 128.0;
const EDITION_OVERLAP: f32 = 7.0;
const LOGO_Y_OFFSET: f32 = 30.0;

enum Screen {
    Main,
    Multiplayer,
}

pub struct MainMenu {
    server_address: String,
    username: String,
    screen: Screen,
    splash: Option<String>,
    start_time: Instant,
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            server_address: "localhost:25565".into(),
            username: "Steve".into(),
            screen: Screen::Main,
            splash: None,
            start_time: Instant::now(),
        }
    }

    pub fn load_splash(&mut self, assets_dir: &Path, asset_index: &Option<AssetIndex>) {
        let path = asset_index
            .as_ref()
            .and_then(|idx| idx.resolve("minecraft/texts/splashes.txt"))
            .unwrap_or_else(|| assets_dir.join("assets/minecraft/texts/splashes.txt"));

        let Ok(contents) = std::fs::read_to_string(&path) else {
            log::warn!("Failed to load splashes.txt");
            return;
        };

        let lines: Vec<&str> = contents.lines().filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            return;
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.start_time.hash(&mut hasher);
        let index = hasher.finish() as usize % lines.len();
        self.splash = Some(lines[index].to_string());
    }

    pub fn draw(&mut self, ctx: &egui::Context, textures: &HudTextures) -> MenuAction {
        let mut action = MenuAction::None;

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let screen = ui.max_rect();
                let painter = ui.painter();

                painter.image(textures.panorama_overlay.id(), screen, UV_FULL, Color32::WHITE);

                let gs = gui_scale(ctx);

                ui.vertical_centered(|ui| {
                    ui.add_space(LOGO_Y_OFFSET * gs);

                    draw_title_logo(ui, textures, &self.splash, self.start_time, gs);

                    ui.add_space(40.0 * gs);

                    match self.screen {
                        Screen::Main => self.draw_main_buttons(ui, textures, &mut action),
                        Screen::Multiplayer => self.draw_connect_form(ui, textures, &mut action),
                    }
                });

                draw_bottom_text(ui, screen, gs);
            });

        action
    }

    fn draw_main_buttons(
        &mut self,
        ui: &mut egui::Ui,
        textures: &HudTextures,
        action: &mut MenuAction,
    ) {
        if mc_button(ui, textures, "Singleplayer") {
            // TODO: singleplayer world list
        }

        ui.add_space(BUTTON_GAP);

        if mc_button(ui, textures, "Multiplayer") {
            self.screen = Screen::Multiplayer;
        }

        let gs = gui_scale(ui.ctx());
        ui.add_space(16.0 * gs);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = BUTTON_GAP;
            let btn_w = textures.button.size()[0] as f32 * gs;
            let half_w = (btn_w - BUTTON_GAP) / 2.0;
            let icon_size = 20.0 * gs;
            let total_w = icon_size + BUTTON_GAP + btn_w + BUTTON_GAP + icon_size;
            let offset = (ui.available_width() - total_w) / 2.0;
            ui.add_space(offset.max(0.0));

            if mc_icon_button(ui, textures, &textures.icon_language) {
                // TODO: language screen
            }

            if mc_button_w(ui, textures, "Options...", half_w) {
                // TODO: options screen
            }

            if mc_button_w(ui, textures, "Quit Game", half_w) {
                *action = MenuAction::Quit;
            }

            if mc_icon_button(ui, textures, &textures.icon_accessibility) {
                // TODO: accessibility screen
            }
        });
    }

    fn draw_connect_form(
        &mut self,
        ui: &mut egui::Ui,
        textures: &HudTextures,
        action: &mut MenuAction,
    ) {
        ui.set_max_width(300.0);

        ui.label(
            egui::RichText::new("Username")
                .size(14.0)
                .color(LABEL_COLOR),
        );
        ui.add_sized(
            [300.0, 30.0],
            egui::TextEdit::singleline(&mut self.username),
        );

        ui.add_space(8.0);

        ui.label(
            egui::RichText::new("Server Address")
                .size(14.0)
                .color(LABEL_COLOR),
        );
        let response = ui.add_sized(
            [300.0, 30.0],
            egui::TextEdit::singleline(&mut self.server_address),
        );

        ui.add_space(16.0);

        let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        ui.horizontal(|ui| {
            ui.add_space(40.0);

            if mc_button(ui, textures, "Back") {
                self.screen = Screen::Main;
            }

            ui.add_space(16.0);

            if mc_button(ui, textures, "Connect") || enter_pressed {
                *action = MenuAction::Connect {
                    server: self.server_address.clone(),
                    username: self.username.clone(),
                };
            }
        });
    }
}

fn draw_title_logo(
    ui: &mut egui::Ui,
    textures: &HudTextures,
    splash: &Option<String>,
    start_time: Instant,
    gs: f32,
) {
    let logo_w = LOGO_WIDTH * gs;
    let logo_rect = draw_scaled_image(ui, &textures.title_logo, logo_w);
    ui.add_space(-EDITION_OVERLAP * gs - ui.spacing().item_spacing.y);
    draw_scaled_image(ui, &textures.edition_badge, EDITION_WIDTH * gs);

    if let Some(splash_text) = splash {
        draw_splash(ui, splash_text, logo_rect, start_time, gs);
    }
}

fn draw_scaled_image(ui: &mut egui::Ui, texture: &egui::TextureHandle, width: f32) -> Rect {
    let aspect = texture.size()[0] as f32 / texture.size()[1] as f32;
    let size = Vec2::new(width, width / aspect);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .image(texture.id(), rect, UV_FULL, Color32::WHITE);
    rect
}

fn draw_splash(ui: &mut egui::Ui, text: &str, logo_rect: Rect, start_time: Instant, gs: f32) {
    let elapsed_ms = start_time.elapsed().as_millis() as f32;
    let cycle = (elapsed_ms % 1000.0) / 1000.0 * std::f32::consts::TAU;
    let pulse = 1.8 - cycle.sin().abs() * 0.1;

    let base_scale = 16.0 * gs;
    let text_w = mc_text_width(ui.ctx(), text, base_scale);
    let splash_scale = pulse * 100.0 / (text_w + 32.0 * gs);
    let font_scale = base_scale * splash_scale;

    let center = Pos2::new(
        logo_rect.center().x + 123.0 * gs,
        logo_rect.min.y + 69.0 * gs,
    );

    let yellow = Color32::from_rgb(255, 255, 0);
    let rotation = -std::f32::consts::PI / 9.0;

    if let Some(font) = super::font::McFont::get(ui.ctx()) {
        let w = font.text_width(text, font_scale);
        let pos = Pos2::new(center.x - w / 2.0, center.y - font_scale / 2.0);
        font.draw_text_rotated(ui.painter(), pos, text, font_scale, yellow, true, center, rotation);
    }

    ui.ctx().request_repaint();
}

fn draw_bottom_text(ui: &mut egui::Ui, screen: Rect, gs: f32) {
    let painter = ui.painter();
    let ctx = ui.ctx();
    let color = Color32::from_rgb(150, 150, 150);
    let font_size = 8.0 * gs;
    let pad = 2.0 * gs;
    let y = screen.max.y - pad - font_size;

    let left = "Minecraft 1.21.11";
    let right = "Ferrite - Not affiliated with Mojang";

    mc_text(painter, ctx, Pos2::new(pad, y), left, font_size, color, true);
    let rw = mc_text_width(ctx, right, font_size);
    mc_text(painter, ctx, Pos2::new(screen.max.x - pad - rw, y), right, font_size, color, true);
}

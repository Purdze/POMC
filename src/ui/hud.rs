use std::path::Path;

use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Vec2};

use crate::assets::{load_image, AssetIndex};

pub fn gui_scale(ctx: &egui::Context) -> f32 {
    let screen = ctx.screen_rect();
    let w = screen.width();
    let h = screen.height();
    let mut scale = 1;
    while (w / (scale + 1) as f32) >= 320.0 && (h / (scale + 1) as f32) >= 240.0 {
        scale += 1;
    }
    scale as f32
}

const CROSSHAIR_SIZE: f32 = 10.0;
const CROSSHAIR_THICKNESS: f32 = 2.0;
pub const BUTTON_GAP: f32 = 8.0;
pub const UV_FULL: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
pub const NEAREST_FILTER: TextureOptions = TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Nearest,
    wrap_mode: egui::TextureWrapMode::ClampToEdge,
    mipmap_mode: None,
};

pub struct HudTextures {
    hotbar: TextureHandle,
    hotbar_selection: TextureHandle,
    pub button: TextureHandle,
    pub button_highlighted: TextureHandle,
    heart_container: TextureHandle,
    heart_full: TextureHandle,
    heart_half: TextureHandle,
    food_empty: TextureHandle,
    food_full: TextureHandle,
    food_half: TextureHandle,
    pub title_logo: TextureHandle,
    pub edition_badge: TextureHandle,
    pub icon_language: TextureHandle,
    pub icon_accessibility: TextureHandle,
    pub panorama_overlay: TextureHandle,
}

impl HudTextures {
    pub fn load(ctx: &egui::Context, assets_dir: &Path, asset_index: &Option<AssetIndex>) -> Self {
        let gui_dir = assets_dir.join("assets/minecraft/textures/gui");
        let sprites = gui_dir.join("sprites");
        let hud = sprites.join("hud");
        let heart = hud.join("heart");

        let overlay_path = asset_index
            .as_ref()
            .and_then(|idx| idx.resolve("minecraft/textures/gui/title/background/panorama_overlay.png"))
            .unwrap_or_else(|| gui_dir.join("title/background/panorama_overlay.png"));

        Self {
            hotbar: load_texture(ctx, &hud.join("hotbar.png"), "hotbar", NEAREST_FILTER),
            hotbar_selection: load_texture(ctx, &hud.join("hotbar_selection.png"), "hotbar_sel", NEAREST_FILTER),
            button: load_texture(ctx, &sprites.join("widget/button.png"), "button", NEAREST_FILTER),
            button_highlighted: load_texture(ctx, &sprites.join("widget/button_highlighted.png"), "button_hl", NEAREST_FILTER),
            heart_container: load_texture(ctx, &heart.join("container.png"), "heart_bg", NEAREST_FILTER),
            heart_full: load_texture(ctx, &heart.join("full.png"), "heart_full", NEAREST_FILTER),
            heart_half: load_texture(ctx, &heart.join("half.png"), "heart_half", NEAREST_FILTER),
            food_empty: load_texture(ctx, &hud.join("food_empty.png"), "food_bg", NEAREST_FILTER),
            food_full: load_texture(ctx, &hud.join("food_full.png"), "food_full", NEAREST_FILTER),
            food_half: load_texture(ctx, &hud.join("food_half.png"), "food_half", NEAREST_FILTER),
            title_logo: load_texture(ctx, &gui_dir.join("title/minecraft.png"), "title_logo", NEAREST_FILTER),
            edition_badge: load_texture(ctx, &gui_dir.join("title/edition.png"), "edition", NEAREST_FILTER),
            icon_language: load_texture(ctx, &sprites.join("icon/language.png"), "icon_lang", NEAREST_FILTER),
            icon_accessibility: load_texture(ctx, &sprites.join("icon/accessibility.png"), "icon_a11y", NEAREST_FILTER),
            panorama_overlay: load_texture(ctx, &overlay_path, "panorama_overlay", TextureOptions::LINEAR),
        }
    }
}

pub fn load_texture(
    ctx: &egui::Context,
    path: &Path,
    name: &str,
    opts: TextureOptions,
) -> TextureHandle {
    let img = load_image(path)
        .unwrap_or_else(|e| {
            log::warn!("Failed to load HUD texture {name}: {e}");
            image::DynamicImage::new_rgba8(1, 1)
        })
        .to_rgba8();

    let size = [img.width() as usize, img.height() as usize];
    let pixels = img.into_raw();

    ctx.load_texture(
        name,
        ColorImage::from_rgba_unmultiplied(size, &pixels),
        opts,
    )
}

pub fn draw_hud(ctx: &egui::Context, textures: &HudTextures, selected_slot: u8, health: f32, food: u32) {
    let screen = ctx.screen_rect();
    let scale = gui_scale(ctx);

    egui::Area::new(egui::Id::new("hud"))
        .fixed_pos(Pos2::ZERO)
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_clip_rect(screen);
            let painter = ui.painter();

            draw_crosshair(painter, screen.center());
            let (hotbar_x, hotbar_top, hotbar_w) = draw_hotbar(painter, screen, textures, selected_slot, scale);

            draw_hearts(painter, textures, hotbar_x, hotbar_top, health, scale);
            draw_food(painter, textures, hotbar_x + hotbar_w, hotbar_top, food, scale);

            ui.allocate_rect(
                Rect::from_min_size(Pos2::ZERO, screen.size()),
                egui::Sense::hover(),
            );
        });
}

fn draw_crosshair(painter: &egui::Painter, center: Pos2) {
    let stroke = Stroke::new(CROSSHAIR_THICKNESS, Color32::WHITE);

    painter.line_segment(
        [
            Pos2::new(center.x - CROSSHAIR_SIZE, center.y),
            Pos2::new(center.x + CROSSHAIR_SIZE, center.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(center.x, center.y - CROSSHAIR_SIZE),
            Pos2::new(center.x, center.y + CROSSHAIR_SIZE),
        ],
        stroke,
    );
}

fn draw_hotbar(painter: &egui::Painter, screen: Rect, textures: &HudTextures, selected_slot: u8, gs: f32) -> (f32, f32, f32) {
    let hotbar_w = textures.hotbar.size()[0] as f32 * gs;
    let hotbar_h = textures.hotbar.size()[1] as f32 * gs;
    let hotbar_x = screen.center().x - hotbar_w / 2.0;
    let hotbar_y = screen.max.y - hotbar_h;
    let hotbar_rect = Rect::from_min_size(
        Pos2::new(hotbar_x, hotbar_y),
        egui::Vec2::new(hotbar_w, hotbar_h),
    );

    painter.image(textures.hotbar.id(), hotbar_rect, UV_FULL, Color32::WHITE);

    let sel_w = textures.hotbar_selection.size()[0] as f32 * gs;
    let sel_h = textures.hotbar_selection.size()[1] as f32 * gs;
    let slot_stride = 20.0 * gs;
    let sel_x = hotbar_x - 1.0 * gs + selected_slot as f32 * slot_stride;
    let sel_y = hotbar_y - 1.0 * gs;
    let sel_rect = Rect::from_min_size(Pos2::new(sel_x, sel_y), egui::Vec2::new(sel_w, sel_h));

    painter.image(textures.hotbar_selection.id(), sel_rect, UV_FULL, Color32::WHITE);

    (hotbar_x, hotbar_y, hotbar_w)
}

struct StatusBarArgs<'a> {
    bg: &'a TextureHandle,
    full: &'a TextureHandle,
    half: &'a TextureHandle,
    x_start: f32,
    y: f32,
    value: f32,
    right_to_left: bool,
}

fn draw_status_bar(painter: &egui::Painter, args: &StatusBarArgs, gs: f32) {
    let icon_size = Vec2::splat(9.0 * gs);
    let stride = 8.0 * gs;
    let full_icons = (args.value / 2.0).floor() as u8;
    let has_half = (args.value % 2.0) >= 1.0;

    for i in 0..10u8 {
        let x = if args.right_to_left {
            args.x_start - (i as f32 + 1.0) * stride
        } else {
            args.x_start + i as f32 * stride
        };
        let rect = Rect::from_min_size(Pos2::new(x, args.y - icon_size.y), icon_size);

        painter.image(args.bg.id(), rect, UV_FULL, Color32::WHITE);

        let tex = if i < full_icons {
            Some(args.full)
        } else if i == full_icons && has_half {
            Some(args.half)
        } else {
            None
        };
        if let Some(t) = tex {
            painter.image(t.id(), rect, UV_FULL, Color32::WHITE);
        }
    }
}

fn draw_hearts(painter: &egui::Painter, textures: &HudTextures, hotbar_x: f32, hotbar_top: f32, health: f32, gs: f32) {
    let y = hotbar_top - 2.0 * gs;
    draw_status_bar(painter, &StatusBarArgs {
        bg: &textures.heart_container, full: &textures.heart_full, half: &textures.heart_half,
        x_start: hotbar_x, y, value: health, right_to_left: false,
    }, gs);
}

fn draw_food(painter: &egui::Painter, textures: &HudTextures, hotbar_right: f32, hotbar_top: f32, food: u32, gs: f32) {
    let y = hotbar_top - 2.0 * gs;
    draw_status_bar(painter, &StatusBarArgs {
        bg: &textures.food_empty, full: &textures.food_full, half: &textures.food_half,
        x_start: hotbar_right, y, value: food as f32, right_to_left: true,
    }, gs);
}

pub fn mc_button(ui: &mut egui::Ui, textures: &HudTextures, label: &str) -> bool {
    let gs = gui_scale(ui.ctx());
    let btn_w = textures.button.size()[0] as f32 * gs;
    mc_button_sized(ui, textures, label, btn_w, gs)
}

pub fn mc_icon_button(ui: &mut egui::Ui, textures: &HudTextures, icon: &TextureHandle) -> bool {
    let gs = gui_scale(ui.ctx());
    let btn_h = textures.button.size()[1] as f32 * gs;
    let (rect, response) = allocate_button(ui, textures, Vec2::splat(btn_h));

    let icon_size = Vec2::splat(btn_h * 0.5);
    let icon_rect = Rect::from_center_size(rect.center(), icon_size);
    ui.painter().image(icon.id(), icon_rect, UV_FULL, Color32::WHITE);

    response.clicked()
}

pub fn mc_button_w(ui: &mut egui::Ui, textures: &HudTextures, label: &str, width: f32) -> bool {
    let gs = gui_scale(ui.ctx());
    mc_button_sized(ui, textures, label, width, gs)
}

fn mc_button_sized(ui: &mut egui::Ui, textures: &HudTextures, label: &str, width: f32, gs: f32) -> bool {
    let btn_h = textures.button.size()[1] as f32 * gs;
    let (rect, response) = allocate_button(ui, textures, Vec2::new(width, btn_h));

    super::font::mc_text_centered(ui.painter(), ui.ctx(), rect.center(), label, 16.0, Color32::WHITE, true);

    response.clicked()
}

fn allocate_button(ui: &mut egui::Ui, textures: &HudTextures, size: Vec2) -> (Rect, egui::Response) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let tex = if response.hovered() {
        &textures.button_highlighted
    } else {
        &textures.button
    };
    ui.painter().image(tex.id(), rect, UV_FULL, Color32::WHITE);
    (rect, response)
}

use std::path::Path;

use azalea_inventory::ItemStack;
use egui::{Color32, Pos2, Rect, TextureHandle, Vec2};

use crate::player::inventory::{item_display_name, Inventory};
use crate::ui::hud::{gui_scale, load_texture, NEAREST_FILTER, UV_FULL};

const SLOT_SIZE: f32 = 18.0;
const INV_TEX_W: f32 = 176.0;
const INV_TEX_H: f32 = 166.0;

const HOTBAR_OFFSET: (f32, f32) = (7.0, 141.0);
const MAIN_OFFSET: (f32, f32) = (7.0, 83.0);
const ARMOR_OFFSET: (f32, f32) = (7.0, 7.0);
const OFFHAND_OFFSET: (f32, f32) = (76.0, 61.0);
const CRAFT_INPUT_OFFSET: (f32, f32) = (97.0, 17.0);
const CRAFT_OUTPUT_OFFSET: (f32, f32) = (153.0, 27.0);

pub struct InventoryTextures {
    background: TextureHandle,
}

impl InventoryTextures {
    pub fn load(ctx: &egui::Context, assets_dir: &Path) -> Self {
        let container_dir = assets_dir.join("assets/minecraft/textures/gui/container");
        Self {
            background: load_texture(
                ctx,
                &container_dir.join("inventory.png"),
                "inventory_bg",
                NEAREST_FILTER,
            ),
        }
    }
}

pub fn draw_inventory(
    ctx: &egui::Context,
    textures: &InventoryTextures,
    inventory: &Inventory,
) -> bool {
    let mut close = false;
    let screen = ctx.screen_rect();
    let scale = gui_scale(ctx)
        .min(screen.width() / INV_TEX_W)
        .min(screen.height() / INV_TEX_H);
    let inv_w = INV_TEX_W * scale;
    let inv_h = INV_TEX_H * scale;
    let origin = Pos2::new(
        (screen.width() - inv_w) / 2.0,
        (screen.height() - inv_h) / 2.0,
    );

    egui::Area::new(egui::Id::new("inventory_overlay"))
        .fixed_pos(Pos2::ZERO)
        .interactable(true)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.set_clip_rect(screen);
            let painter = ui.painter();
            painter.rect_filled(screen, 0.0, Color32::from_black_alpha(128));

            let bg_rect = Rect::from_min_size(origin, Vec2::new(inv_w, inv_h));
            painter.image(textures.background.id(), bg_rect, UV_FULL, Color32::WHITE);

            draw_slot_grid(ui, origin, scale, HOTBAR_OFFSET, 9, 1, inventory.hotbar_slots());
            draw_slot_grid(ui, origin, scale, MAIN_OFFSET, 9, 3, inventory.main_slots());
            draw_slot_grid(ui, origin, scale, ARMOR_OFFSET, 1, 4, inventory.armor_slots());
            draw_slot_grid(ui, origin, scale, CRAFT_INPUT_OFFSET, 2, 2, inventory.craft_input_slots());
            draw_single_slot(ui, origin, scale, OFFHAND_OFFSET, inventory.offhand());
            draw_single_slot(ui, origin, scale, CRAFT_OUTPUT_OFFSET, inventory.craft_output());

            let bg_response = ui.allocate_rect(screen, egui::Sense::click());
            if bg_response.clicked_elsewhere() {
                close = true;
            }
        });

    close
}

fn draw_slot_grid(
    ui: &egui::Ui,
    origin: Pos2,
    scale: f32,
    offset: (f32, f32),
    cols: usize,
    rows: usize,
    items: &[ItemStack],
) {
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            let item = items.get(idx).unwrap_or(&ItemStack::Empty);
            let x = origin.x + (offset.0 + col as f32 * SLOT_SIZE + 1.0) * scale;
            let y = origin.y + (offset.1 + row as f32 * SLOT_SIZE + 1.0) * scale;
            draw_item(ui, Pos2::new(x, y), scale, item);
        }
    }
}

fn draw_single_slot(ui: &egui::Ui, origin: Pos2, scale: f32, offset: (f32, f32), item: &ItemStack) {
    let x = origin.x + (offset.0 + 1.0) * scale;
    let y = origin.y + (offset.1 + 1.0) * scale;
    draw_item(ui, Pos2::new(x, y), scale, item);
}

fn draw_item(ui: &egui::Ui, pos: Pos2, scale: f32, item: &ItemStack) {
    let ItemStack::Present(data) = item else {
        return;
    };

    let size = 16.0 * scale;
    let rect = Rect::from_min_size(pos, Vec2::splat(size));

    let name = item_display_name(data.kind);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        &name,
        egui::FontId::proportional(9.0),
        Color32::WHITE,
    );

    if data.count > 1 {
        ui.painter().text(
            Pos2::new(rect.max.x - 1.0, rect.max.y - 1.0),
            egui::Align2::RIGHT_BOTTOM,
            data.count.to_string(),
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );
    }
}

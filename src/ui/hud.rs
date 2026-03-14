use super::common::WHITE;
use crate::renderer::pipelines::menu_overlay::{MenuElement, SpriteId};

const CROSSHAIR_SIZE: f32 = 10.0;
const CROSSHAIR_THICKNESS: f32 = 2.0;

const HOTBAR_W: f32 = 182.0;
const HOTBAR_H: f32 = 22.0;
const SELECTION_W: f32 = 24.0;
const SELECTION_H: f32 = 24.0;
const SLOT_STRIDE: f32 = 20.0;
const ICON_SIZE: f32 = 9.0;
const ICON_STRIDE: f32 = 8.0;

pub fn gui_scale(screen_w: f32, screen_h: f32) -> f32 {
    let mut scale = 1;
    while (screen_w / (scale + 1) as f32) >= 320.0 && (screen_h / (scale + 1) as f32) >= 240.0 {
        scale += 1;
    }
    scale as f32
}

pub fn build_hud(
    elements: &mut Vec<MenuElement>,
    screen_w: f32,
    screen_h: f32,
    selected_slot: u8,
    health: f32,
    food: u32,
    fps: Option<u32>,
) {
    let gs = gui_scale(screen_w, screen_h);
    let cx = screen_w / 2.0;
    let cy = screen_h / 2.0;

    build_crosshair(elements, cx, cy);

    if let Some(fps) = fps {
        let fs = super::common::FONT_SIZE * gs;
        elements.push(MenuElement::Text {
            x: 4.0 * gs,
            y: 4.0 * gs,
            text: format!("{fps} fps"),
            scale: fs,
            color: WHITE,
            centered: false,
        });
    }

    let hotbar_w = HOTBAR_W * gs;
    let hotbar_h = HOTBAR_H * gs;
    let hotbar_x = cx - hotbar_w / 2.0;
    let hotbar_y = screen_h - hotbar_h;

    elements.push(MenuElement::Image {
        x: hotbar_x,
        y: hotbar_y,
        w: hotbar_w,
        h: hotbar_h,
        sprite: SpriteId::Hotbar,
        tint: WHITE,
    });

    let sel_w = SELECTION_W * gs;
    let sel_h = SELECTION_H * gs;
    let sel_x = hotbar_x - 1.0 * gs + selected_slot as f32 * SLOT_STRIDE * gs;
    let sel_y = hotbar_y - 1.0 * gs;
    elements.push(MenuElement::Image {
        x: sel_x,
        y: sel_y,
        w: sel_w,
        h: sel_h,
        sprite: SpriteId::HotbarSelection,
        tint: WHITE,
    });

    build_status_bar(
        elements,
        hotbar_x,
        hotbar_y - 2.0 * gs,
        health,
        false,
        SpriteId::HeartContainer,
        SpriteId::HeartFull,
        SpriteId::HeartHalf,
        gs,
    );
    build_status_bar(
        elements,
        hotbar_x + hotbar_w,
        hotbar_y - 2.0 * gs,
        food as f32,
        true,
        SpriteId::FoodEmpty,
        SpriteId::FoodFull,
        SpriteId::FoodHalf,
        gs,
    );
}

fn build_crosshair(elements: &mut Vec<MenuElement>, cx: f32, cy: f32) {
    elements.push(MenuElement::Rect {
        x: cx - CROSSHAIR_SIZE,
        y: cy - CROSSHAIR_THICKNESS / 2.0,
        w: CROSSHAIR_SIZE * 2.0,
        h: CROSSHAIR_THICKNESS,
        corner_radius: 0.0,
        color: WHITE,
    });
    elements.push(MenuElement::Rect {
        x: cx - CROSSHAIR_THICKNESS / 2.0,
        y: cy - CROSSHAIR_SIZE,
        w: CROSSHAIR_THICKNESS,
        h: CROSSHAIR_SIZE * 2.0,
        corner_radius: 0.0,
        color: WHITE,
    });
}

#[allow(clippy::too_many_arguments)]
fn build_status_bar(
    elements: &mut Vec<MenuElement>,
    x_start: f32,
    y: f32,
    value: f32,
    right_to_left: bool,
    bg: SpriteId,
    full: SpriteId,
    half: SpriteId,
    gs: f32,
) {
    let icon_size = ICON_SIZE * gs;
    let stride = ICON_STRIDE * gs;
    let full_icons = (value / 2.0).floor() as u8;
    let has_half = (value % 2.0) >= 1.0;

    for i in 0..10u8 {
        let x = if right_to_left {
            x_start - (i as f32 + 1.0) * stride
        } else {
            x_start + i as f32 * stride
        };
        let iy = y - icon_size;

        elements.push(MenuElement::Image {
            x,
            y: iy,
            w: icon_size,
            h: icon_size,
            sprite: bg,
            tint: WHITE,
        });

        let icon = if i < full_icons {
            Some(full)
        } else if i == full_icons && has_half {
            Some(half)
        } else {
            None
        };
        if let Some(sprite) = icon {
            elements.push(MenuElement::Image {
                x,
                y: iy,
                w: icon_size,
                h: icon_size,
                sprite,
                tint: WHITE,
            });
        }
    }
}

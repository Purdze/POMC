use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use egui::{Color32, ColorImage, Pos2, Rect, TextureHandle, Vec2};

use super::hud::NEAREST_FILTER;

use crate::assets::{load_image, AssetIndex};

const GRID_COLS: u32 = 16;
const GRID_ROWS: u32 = 16;

const ASCII_CHARS: [&str; 16] = [
    "\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}",
    "\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}",
    "\u{0020}\u{0021}\u{0022}\u{0023}\u{0024}\u{0025}\u{0026}\u{0027}\u{0028}\u{0029}\u{002a}\u{002b}\u{002c}\u{002d}\u{002e}\u{002f}",
    "\u{0030}\u{0031}\u{0032}\u{0033}\u{0034}\u{0035}\u{0036}\u{0037}\u{0038}\u{0039}\u{003a}\u{003b}\u{003c}\u{003d}\u{003e}\u{003f}",
    "\u{0040}\u{0041}\u{0042}\u{0043}\u{0044}\u{0045}\u{0046}\u{0047}\u{0048}\u{0049}\u{004a}\u{004b}\u{004c}\u{004d}\u{004e}\u{004f}",
    "\u{0050}\u{0051}\u{0052}\u{0053}\u{0054}\u{0055}\u{0056}\u{0057}\u{0058}\u{0059}\u{005a}\u{005b}\u{005c}\u{005d}\u{005e}\u{005f}",
    "\u{0060}\u{0061}\u{0062}\u{0063}\u{0064}\u{0065}\u{0066}\u{0067}\u{0068}\u{0069}\u{006a}\u{006b}\u{006c}\u{006d}\u{006e}\u{006f}",
    "\u{0070}\u{0071}\u{0072}\u{0073}\u{0074}\u{0075}\u{0076}\u{0077}\u{0078}\u{0079}\u{007a}\u{007b}\u{007c}\u{007d}\u{007e}\u{0000}",
    "\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}",
    "\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{00a3}\u{0000}\u{0000}\u{0192}",
    "\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{00aa}\u{00ba}\u{0000}\u{0000}\u{00ac}\u{0000}\u{0000}\u{0000}\u{00ab}\u{00bb}",
    "\u{2591}\u{2592}\u{2593}\u{2502}\u{2524}\u{2561}\u{2562}\u{2556}\u{2555}\u{2563}\u{2551}\u{2557}\u{255d}\u{255c}\u{255b}\u{2510}",
    "\u{2514}\u{2534}\u{252c}\u{251c}\u{2500}\u{253c}\u{255e}\u{255f}\u{255a}\u{2554}\u{2569}\u{2566}\u{2560}\u{2550}\u{256c}\u{2567}",
    "\u{2568}\u{2564}\u{2565}\u{2559}\u{2558}\u{2552}\u{2553}\u{256b}\u{256a}\u{2518}\u{250c}\u{2588}\u{2584}\u{258c}\u{2590}\u{2580}",
    "\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{2205}\u{2208}\u{0000}",
    "\u{2261}\u{00b1}\u{2265}\u{2264}\u{2320}\u{2321}\u{00f7}\u{2248}\u{00b0}\u{2219}\u{0000}\u{221a}\u{207f}\u{00b2}\u{25a0}\u{0000}",
];

#[derive(Clone)]
struct GlyphInfo {
    col: u32,
    row: u32,
    width: u32,
}

#[derive(Clone)]
struct McFontInner {
    texture: TextureHandle,
    glyphs: HashMap<char, GlyphInfo>,
    cell_w: u32,
    cell_h: u32,
}

#[derive(Clone)]
pub struct McFont(Arc<McFontInner>);

impl McFont {
    pub fn load(
        ctx: &egui::Context,
        assets_dir: &Path,
        asset_index: &Option<AssetIndex>,
    ) -> Option<Self> {
        let path = asset_index
            .as_ref()
            .and_then(|idx| idx.resolve("minecraft/textures/font/ascii.png"))
            .unwrap_or_else(|| assets_dir.join("assets/minecraft/textures/font/ascii.png"));

        let img = load_image(&path)
            .map_err(|e| log::warn!("Failed to load MC font: {e}"))
            .ok()?
            .to_rgba8();

        let tex_w = img.width();
        let tex_h = img.height();
        let cell_w = tex_w / GRID_COLS;
        let cell_h = tex_h / GRID_ROWS;

        let mut glyphs = HashMap::new();
        for (row, line) in ASCII_CHARS.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                if ch == '\0' {
                    continue;
                }
                let width = detect_glyph_width(&img, col as u32 * cell_w, row as u32 * cell_h, cell_w, cell_h);
                if width > 0 {
                    glyphs.insert(ch, GlyphInfo {
                        col: col as u32,
                        row: row as u32,
                        width,
                    });
                }
            }
        }

        glyphs.insert(' ', GlyphInfo { col: 0, row: 2, width: cell_w / 2 });

        let size = [tex_w as usize, tex_h as usize];
        let pixels = img.into_raw();
        let texture = ctx.load_texture(
            "mc_font",
            ColorImage::from_rgba_unmultiplied(size, &pixels),
            NEAREST_FILTER,
        );

        let font = Self(Arc::new(McFontInner {
            texture,
            glyphs,
            cell_w,
            cell_h,
        }));

        ctx.data_mut(|d| d.insert_temp(egui::Id::new("mc_font"), font.clone()));

        Some(font)
    }

    pub fn get(ctx: &egui::Context) -> Option<Self> {
        ctx.data(|d| d.get_temp(egui::Id::new("mc_font")))
    }

    pub fn draw_text(
        &self,
        painter: &egui::Painter,
        pos: Pos2,
        text: &str,
        scale: f32,
        color: Color32,
        shadow: bool,
    ) {
        if shadow {
            let shadow_pos = Pos2::new(pos.x + 1.0, pos.y + 1.0);
            self.draw_glyphs(painter, shadow_pos, text, scale, shadow_color(color));
        }
        self.draw_glyphs(painter, pos, text, scale, color);
    }

    pub fn text_width(&self, text: &str, scale: f32) -> f32 {
        let mut w = 0.0;
        for ch in text.chars() {
            if let Some(glyph) = self.0.glyphs.get(&ch) {
                w += (glyph.width as f32 + 1.0) * scale / self.0.cell_h as f32;
            }
        }
        w
    }

    pub fn draw_text_rotated(
        &self,
        painter: &egui::Painter,
        pos: Pos2,
        text: &str,
        scale: f32,
        color: Color32,
        shadow: bool,
        pivot: Pos2,
        angle: f32,
    ) {
        if shadow {
            let shadow_pos = Pos2::new(pos.x + 1.0, pos.y + 1.0);
            self.draw_glyphs_rotated(painter, shadow_pos, text, scale, shadow_color(color), pivot, angle);
        }
        self.draw_glyphs_rotated(painter, pos, text, scale, color, pivot, angle);
    }

    fn draw_glyphs_rotated(
        &self,
        painter: &egui::Painter,
        mut pos: Pos2,
        text: &str,
        scale: f32,
        color: Color32,
        pivot: Pos2,
        angle: f32,
    ) {
        let inner = &self.0;
        let tex_w = (inner.cell_w * GRID_COLS) as f32;
        let tex_h = (inner.cell_h * GRID_ROWS) as f32;
        let cos = angle.cos();
        let sin = angle.sin();

        let rotate = |p: Pos2| -> Pos2 {
            let dx = p.x - pivot.x;
            let dy = p.y - pivot.y;
            Pos2::new(pivot.x + dx * cos - dy * sin, pivot.y + dx * sin + dy * cos)
        };

        for ch in text.chars() {
            let Some(glyph) = inner.glyphs.get(&ch) else {
                continue;
            };

            let glyph_w = glyph.width as f32 * scale / inner.cell_h as f32;

            let u0 = (glyph.col * inner.cell_w) as f32 / tex_w;
            let v0 = (glyph.row * inner.cell_h) as f32 / tex_h;
            let u1 = (glyph.col * inner.cell_w + glyph.width) as f32 / tex_w;
            let v1 = ((glyph.row + 1) * inner.cell_h) as f32 / tex_h;

            let tl = rotate(pos);
            let tr = rotate(Pos2::new(pos.x + glyph_w, pos.y));
            let bl = rotate(Pos2::new(pos.x, pos.y + scale));
            let br = rotate(Pos2::new(pos.x + glyph_w, pos.y + scale));

            let mesh = egui::Mesh {
                texture_id: inner.texture.id(),
                indices: vec![0, 1, 2, 2, 1, 3],
                vertices: vec![
                    egui::epaint::Vertex { pos: tl, uv: Pos2::new(u0, v0), color },
                    egui::epaint::Vertex { pos: tr, uv: Pos2::new(u1, v0), color },
                    egui::epaint::Vertex { pos: bl, uv: Pos2::new(u0, v1), color },
                    egui::epaint::Vertex { pos: br, uv: Pos2::new(u1, v1), color },
                ],
            };
            painter.add(egui::Shape::mesh(mesh));

            pos.x += glyph_w + scale / inner.cell_h as f32;
        }
    }

    fn draw_glyphs(
        &self,
        painter: &egui::Painter,
        mut pos: Pos2,
        text: &str,
        scale: f32,
        color: Color32,
    ) {
        let inner = &self.0;
        let tex_w = (inner.cell_w * GRID_COLS) as f32;
        let tex_h = (inner.cell_h * GRID_ROWS) as f32;

        for ch in text.chars() {
            let Some(glyph) = inner.glyphs.get(&ch) else {
                continue;
            };

            let glyph_w = glyph.width as f32 * scale / inner.cell_h as f32;

            let u0 = (glyph.col * inner.cell_w) as f32 / tex_w;
            let v0 = (glyph.row * inner.cell_h) as f32 / tex_h;
            let u1 = (glyph.col * inner.cell_w + glyph.width) as f32 / tex_w;
            let v1 = ((glyph.row + 1) * inner.cell_h) as f32 / tex_h;

            let rect = Rect::from_min_size(pos, Vec2::new(glyph_w, scale));
            let uv = Rect::from_min_max(Pos2::new(u0, v0), Pos2::new(u1, v1));

            painter.image(inner.texture.id(), rect, uv, color);

            pos.x += glyph_w + scale / inner.cell_h as f32;
        }
    }
}

pub fn mc_text(painter: &egui::Painter, ctx: &egui::Context, pos: Pos2, text: &str, scale: f32, color: Color32, shadow: bool) {
    if let Some(font) = McFont::get(ctx) {
        font.draw_text(painter, pos, text, scale, color, shadow);
    } else {
        painter.text(pos, egui::Align2::LEFT_TOP, text, egui::FontId::proportional(scale * 0.7), color);
    }
}

pub fn mc_text_width(ctx: &egui::Context, text: &str, scale: f32) -> f32 {
    McFont::get(ctx)
        .map(|f| f.text_width(text, scale))
        .unwrap_or_else(|| text.len() as f32 * scale * 0.5)
}

pub fn mc_text_centered(painter: &egui::Painter, ctx: &egui::Context, center: Pos2, text: &str, scale: f32, color: Color32, shadow: bool) {
    let w = mc_text_width(ctx, text, scale);
    mc_text(painter, ctx, Pos2::new(center.x - w / 2.0, center.y - scale / 2.0), text, scale, color, shadow);
}

fn shadow_color(color: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color.r() as f32 * 0.25) as u8,
        (color.g() as f32 * 0.25) as u8,
        (color.b() as f32 * 0.25) as u8,
        color.a(),
    )
}

fn detect_glyph_width(img: &image::RgbaImage, x0: u32, y0: u32, cell_w: u32, cell_h: u32) -> u32 {
    let mut max_x = 0;
    for y in y0..y0 + cell_h {
        for x in (x0..x0 + cell_w).rev() {
            if img.get_pixel(x, y)[3] > 0 {
                let local_x = x - x0 + 1;
                if local_x > max_x {
                    max_x = local_x;
                }
                break;
            }
        }
    }
    max_x
}

use azalea_registry::builtin::ItemKind;

use super::common::{self, WHITE};
use crate::player::inventory::item_resource_name;
use crate::renderer::pipelines::menu_overlay::MenuElement;

const COLS: usize = 9;
const SLOT_SIZE: f32 = 18.0;
const SLOT_PAD: f32 = 2.0;
const GRID_PAD: f32 = 8.0;
const TAB_H: f32 = 20.0;
const SEARCH_H: f32 = 16.0;
const BG_COLOR: [f32; 4] = [0.12, 0.12, 0.12, 0.94];
const SLOT_COLOR: [f32; 4] = [0.15, 0.15, 0.15, 0.8];
const SLOT_HOVER: [f32; 4] = [0.3, 0.3, 0.3, 0.9];
const TAB_COLOR: [f32; 4] = [0.18, 0.18, 0.18, 0.9];
const TAB_ACTIVE: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
const LABEL_COLOR: [f32; 4] = [0.9, 0.9, 0.9, 1.0];
const DIM_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 0.7];

pub struct CreativeState {
    pub scroll: f32,
    pub tab: usize,
    items_cache: Vec<Vec<ItemKind>>,
    #[allow(dead_code)]
    search: String,
}

#[derive(Clone, Copy)]
struct Tab {
    name: &'static str,
    filter: fn(&str) -> bool,
}

const TABS: &[Tab] = &[
    Tab {
        name: "All",
        filter: |_| true,
    },
    Tab {
        name: "Building",
        filter: is_building,
    },
    Tab {
        name: "Nature",
        filter: is_nature,
    },
    Tab {
        name: "Redstone",
        filter: is_redstone,
    },
    Tab {
        name: "Tools",
        filter: is_tool,
    },
];

impl CreativeState {
    pub fn new() -> Self {
        let mut state = Self {
            scroll: 0.0,
            tab: 0,
            items_cache: Vec::new(),
            search: String::new(),
        };
        state.rebuild_cache();
        state
    }

    fn rebuild_cache(&mut self) {
        self.items_cache.clear();
        let all_items = all_item_kinds();
        for tab in TABS {
            let filtered: Vec<ItemKind> = all_items
                .iter()
                .copied()
                .filter(|k| {
                    let name = k
                        .to_string()
                        .strip_prefix("minecraft:")
                        .unwrap_or("air")
                        .to_string();
                    name != "air" && (tab.filter)(&name)
                })
                .collect();
            self.items_cache.push(filtered);
        }
    }

    fn filtered_items(&self) -> &[ItemKind] {
        &self.items_cache[self.tab.min(self.items_cache.len() - 1)]
    }
}

pub struct CreativeResult {
    pub picked: Option<(u8, ItemKind)>,
    pub close: bool,
    #[allow(dead_code)]
    pub any_hovered: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn build_creative(
    elements: &mut Vec<MenuElement>,
    state: &mut CreativeState,
    screen_w: f32,
    screen_h: f32,
    cursor: (f32, f32),
    clicked: bool,
    scroll_delta: f32,
    selected_slot: u8,
    gs: f32,
) -> CreativeResult {
    let s = gs.max(1.0);
    let grid_w = COLS as f32 * (SLOT_SIZE + SLOT_PAD) * s;
    let panel_w = grid_w + GRID_PAD * 2.0 * s;
    let visible_rows = 6;
    let grid_h = visible_rows as f32 * (SLOT_SIZE + SLOT_PAD) * s;
    let panel_h =
        TAB_H * s + SEARCH_H * s + GRID_PAD * s + grid_h + GRID_PAD * s + (SLOT_SIZE + 4.0) * s;

    let ox = (screen_w - panel_w) / 2.0;
    let oy = (screen_h - panel_h) / 2.0;

    let mut any_hovered = false;
    let mut picked = None;
    let mut close = false;

    common::push_overlay(elements, screen_w, screen_h, 0.5);

    elements.push(MenuElement::Rect {
        x: ox,
        y: oy,
        w: panel_w,
        h: panel_h,
        corner_radius: 4.0 * s,
        color: BG_COLOR,
    });

    let mut tx = ox + GRID_PAD * s;
    let ty = oy + 4.0 * s;
    let fs = 7.0 * s;
    for (i, tab) in TABS.iter().enumerate() {
        let tw = (tab.name.len() as f32 * 5.0 + 12.0) * s;
        let tab_rect = [tx, ty, tw, TAB_H * s - 4.0 * s];
        let hovered = common::hit_test(cursor, tab_rect);
        any_hovered |= hovered;

        elements.push(MenuElement::Rect {
            x: tab_rect[0],
            y: tab_rect[1],
            w: tab_rect[2],
            h: tab_rect[3],
            corner_radius: 3.0 * s,
            color: if i == state.tab {
                TAB_ACTIVE
            } else if hovered {
                SLOT_HOVER
            } else {
                TAB_COLOR
            },
        });
        elements.push(MenuElement::Text {
            x: tx + tw / 2.0,
            y: ty + (TAB_H * s - 4.0 * s - fs) / 2.0,
            text: tab.name.into(),
            scale: fs,
            color: if i == state.tab {
                LABEL_COLOR
            } else {
                DIM_COLOR
            },
            centered: true,
        });

        if clicked && hovered {
            state.tab = i;
            state.scroll = 0.0;
        }

        tx += tw + 3.0 * s;
    }

    let grid_x = ox + GRID_PAD * s;
    let grid_y = oy + TAB_H * s + SEARCH_H * s + GRID_PAD * s;

    let items: Vec<ItemKind> = state.filtered_items().to_vec();
    let total_rows = items.len().div_ceil(COLS);
    let max_scroll =
        ((total_rows as f32 - visible_rows as f32) * (SLOT_SIZE + SLOT_PAD) * s).max(0.0);

    let grid_rect = [grid_x, grid_y, grid_w, grid_h];
    let grid_hovered = common::hit_test(cursor, grid_rect);
    if grid_hovered {
        state.scroll =
            (state.scroll - scroll_delta * (SLOT_SIZE + SLOT_PAD) * s * 3.0).clamp(0.0, max_scroll);
    }

    let slot_stride = (SLOT_SIZE + SLOT_PAD) * s;
    let slot_sz = SLOT_SIZE * s;

    for (idx, &kind) in items.iter().enumerate() {
        let col = idx % COLS;
        let row = idx / COLS;
        let sx = grid_x + col as f32 * slot_stride;
        let sy = grid_y + row as f32 * slot_stride - state.scroll;

        if sy + slot_sz < grid_y || sy > grid_y + grid_h {
            continue;
        }

        let slot_rect = [sx, sy, slot_sz, slot_sz];
        let hovered = common::hit_test(cursor, slot_rect) && grid_hovered;
        any_hovered |= hovered;

        elements.push(MenuElement::Rect {
            x: sx,
            y: sy,
            w: slot_sz,
            h: slot_sz,
            corner_radius: 2.0 * s,
            color: if hovered { SLOT_HOVER } else { SLOT_COLOR },
        });

        let name = item_resource_name(kind);
        elements.push(MenuElement::ItemIcon {
            x: sx,
            y: sy,
            w: slot_sz,
            h: slot_sz,
            item_name: name.clone(),
            tint: WHITE,
        });

        if clicked && hovered {
            picked = Some((selected_slot, kind));
        }

        if hovered {
            let tooltip = name.replace('_', " ");
            let tip_fs = 6.0 * s;
            let tip_w = tooltip.len() as f32 * 4.0 * s + 8.0 * s;
            let tip_x = (cursor.0 + 8.0 * s).min(screen_w - tip_w);
            let tip_y = cursor.1 - tip_fs - 6.0 * s;
            elements.push(MenuElement::Rect {
                x: tip_x,
                y: tip_y,
                w: tip_w,
                h: tip_fs + 4.0 * s,
                corner_radius: 2.0 * s,
                color: [0.1, 0.1, 0.1, 0.95],
            });
            elements.push(MenuElement::Text {
                x: tip_x + 4.0 * s,
                y: tip_y + 2.0 * s,
                text: tooltip,
                scale: tip_fs,
                color: LABEL_COLOR,
                centered: false,
            });
        }
    }

    let hotbar_y = grid_y + grid_h + GRID_PAD * s;
    let hotbar_fs = 6.0 * s;
    elements.push(MenuElement::Text {
        x: grid_x,
        y: hotbar_y - hotbar_fs - 2.0 * s,
        text: "Hotbar".into(),
        scale: hotbar_fs,
        color: DIM_COLOR,
        centered: false,
    });

    let outside =
        cursor.0 < ox || cursor.0 > ox + panel_w || cursor.1 < oy || cursor.1 > oy + panel_h;
    if clicked && outside {
        close = true;
    }

    CreativeResult {
        picked,
        close,
        any_hovered,
    }
}

fn all_item_kinds() -> Vec<ItemKind> {
    let mut items = Vec::new();
    let mut id = 0u32;
    while let Ok(kind) = ItemKind::try_from(id) {
        items.push(kind);
        id += 1;
    }
    items
}

fn is_building(name: &str) -> bool {
    name.contains("planks")
        || name.contains("log")
        || name.contains("wood")
        || name.contains("stairs")
        || name.contains("slab")
        || name.contains("fence")
        || name.contains("wall")
        || name.contains("door")
        || name.contains("trapdoor")
        || name.contains("bricks")
        || name.contains("stone")
        || name.contains("cobblestone")
        || name.contains("sandstone")
        || name.contains("deepslate")
        || name.contains("copper")
        || name.contains("iron_block")
        || name.contains("gold_block")
        || name.contains("diamond_block")
        || name.contains("glass")
        || name.contains("concrete")
        || name.contains("terracotta")
        || name.contains("wool")
        || name.contains("quartz")
        || name.contains("prismarine")
        || name.contains("purpur")
}

fn is_nature(name: &str) -> bool {
    name.contains("dirt")
        || name.contains("grass")
        || name.contains("sand")
        || name.contains("gravel")
        || name.contains("clay")
        || name.contains("mud")
        || name.contains("moss")
        || name.contains("leaves")
        || name.contains("sapling")
        || name.contains("flower")
        || name.contains("fern")
        || name.contains("vine")
        || name.contains("mushroom")
        || name.contains("coral")
        || name.contains("kelp")
        || name.contains("seagrass")
        || name.contains("cactus")
        || name.contains("melon")
        || name.contains("pumpkin")
        || name.contains("ice")
        || name.contains("snow")
        || name.contains("ore")
        || name.contains("seed")
}

fn is_redstone(name: &str) -> bool {
    name.contains("redstone")
        || name.contains("piston")
        || name.contains("lever")
        || name.contains("button")
        || name.contains("pressure_plate")
        || name.contains("repeater")
        || name.contains("comparator")
        || name.contains("observer")
        || name.contains("hopper")
        || name.contains("dropper")
        || name.contains("dispenser")
        || name.contains("lamp")
        || name.contains("target")
        || name.contains("tripwire")
}

fn is_tool(name: &str) -> bool {
    name.contains("sword")
        || name.contains("pickaxe")
        || name.contains("axe")
        || name.contains("shovel")
        || name.contains("hoe")
        || name.contains("bow")
        || name.contains("arrow")
        || name.contains("shield")
        || name.contains("helmet")
        || name.contains("chestplate")
        || name.contains("leggings")
        || name.contains("boots")
        || name.contains("bucket")
        || name.contains("compass")
        || name.contains("clock")
        || name.contains("map")
        || name.contains("shears")
        || name.contains("flint_and_steel")
        || name.contains("fishing_rod")
        || name.contains("lead")
        || name.contains("name_tag")
}

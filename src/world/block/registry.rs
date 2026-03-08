use std::collections::HashMap;

use azalea_block::BlockState;

pub struct FaceTextures {
    pub top: &'static str,
    pub bottom: &'static str,
    pub north: &'static str,
    pub south: &'static str,
    pub east: &'static str,
    pub west: &'static str,
}

impl FaceTextures {
    fn all(name: &'static str) -> Self {
        Self {
            top: name,
            bottom: name,
            north: name,
            south: name,
            east: name,
            west: name,
        }
    }

    fn top_bottom_side(top: &'static str, bottom: &'static str, side: &'static str) -> Self {
        Self {
            top,
            bottom,
            north: side,
            south: side,
            east: side,
            west: side,
        }
    }
}

pub struct BlockRegistry {
    textures: HashMap<&'static str, FaceTextures>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        let mut textures = HashMap::new();

        let all = |name: &'static str| FaceTextures::all(name);
        let tbs = |t: &'static str, b: &'static str, s: &'static str| {
            FaceTextures::top_bottom_side(t, b, s)
        };

        textures.insert("stone", all("stone"));
        textures.insert("granite", all("granite"));
        textures.insert("polished_granite", all("polished_granite"));
        textures.insert("diorite", all("diorite"));
        textures.insert("polished_diorite", all("polished_diorite"));
        textures.insert("andesite", all("andesite"));
        textures.insert("polished_andesite", all("polished_andesite"));
        textures.insert(
            "grass_block",
            tbs("grass_block_top", "dirt", "grass_block_side"),
        );
        textures.insert("dirt", all("dirt"));
        textures.insert("coarse_dirt", all("coarse_dirt"));
        textures.insert("cobblestone", all("cobblestone"));
        textures.insert("bedrock", all("bedrock"));
        textures.insert("sand", all("sand"));
        textures.insert("red_sand", all("red_sand"));
        textures.insert("gravel", all("gravel"));
        textures.insert("oak_log", tbs("oak_log_top", "oak_log_top", "oak_log"));
        textures.insert("oak_planks", all("oak_planks"));
        textures.insert("oak_leaves", all("oak_leaves"));
        textures.insert("glass", all("glass"));
        textures.insert("coal_ore", all("coal_ore"));
        textures.insert("iron_ore", all("iron_ore"));
        textures.insert("gold_ore", all("gold_ore"));
        textures.insert("diamond_ore", all("diamond_ore"));
        textures.insert(
            "deepslate",
            tbs("deepslate_top", "deepslate_top", "deepslate"),
        );
        textures.insert("cobbled_deepslate", all("cobbled_deepslate"));
        textures.insert("tuff", all("tuff"));
        textures.insert("water", all("water_still"));
        textures.insert("lava", all("lava_still"));
        textures.insert("clay", all("clay"));
        textures.insert("snow_block", all("snow"));
        textures.insert("short_grass", all("short_grass"));

        Self { textures }
    }

    pub fn get_textures(&self, state: BlockState) -> Option<&FaceTextures> {
        let block: Box<dyn azalea_block::BlockTrait> = state.into();
        let name = block.id();
        self.textures.get(name)
    }

    pub fn texture_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.textures
            .values()
            .flat_map(|ft| [ft.top, ft.bottom, ft.north, ft.south, ft.east, ft.west])
    }
}

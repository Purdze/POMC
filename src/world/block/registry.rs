use std::collections::HashMap;
use std::path::Path;

use azalea_block::BlockState;
use serde::{Deserialize, Serialize};

use crate::assets::AssetIndex;

use super::model::{self, BakedModel};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tint {
    None,
    Grass,
    Foliage,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FaceTextures {
    pub top: String,
    pub bottom: String,
    pub north: String,
    pub south: String,
    pub east: String,
    pub west: String,
    pub side_overlay: Option<String>,
    pub tint: Tint,
}

impl FaceTextures {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        top: &str,
        bottom: &str,
        north: &str,
        south: &str,
        east: &str,
        west: &str,
        side_overlay: Option<&str>,
        tint: Tint,
    ) -> Self {
        Self {
            top: top.into(),
            bottom: bottom.into(),
            north: north.into(),
            south: south.into(),
            east: east.into(),
            west: west.into(),
            side_overlay: side_overlay.map(Into::into),
            tint,
        }
    }

    pub fn uniform(name: &str, tint: Tint) -> Self {
        Self::new(name, name, name, name, name, name, None, tint)
    }
}

#[derive(Clone)]
pub struct BlockRegistry {
    textures: HashMap<String, FaceTextures>,
    /// block_name → (variant_key → BakedModel)
    baked: HashMap<String, HashMap<String, BakedModel>>,
}

impl BlockRegistry {
    pub fn load(assets_dir: &Path, asset_index: &Option<AssetIndex>, game_dir: &Path) -> Self {
        let cache_path = game_dir.join("pomc_block_cache.json");

        let textures = if let Some(cached) = load_cache(&cache_path) {
            log::info!("Block registry: {} blocks (cached textures)", cached.len());
            cached
        } else {
            let mut textures = model::load_all_block_textures(assets_dir, asset_index);

            textures
                .entry("water".into())
                .or_insert_with(|| FaceTextures::uniform("water_still", Tint::None));
            textures
                .entry("lava".into())
                .or_insert_with(|| FaceTextures::uniform("lava_still", Tint::None));

            save_cache(&cache_path, &textures);
            log::info!(
                "Block registry: {} blocks (built and cached)",
                textures.len()
            );
            textures
        };

        let baked = model::bake_all_models(assets_dir, asset_index);

        Self { textures, baked }
    }

    pub fn get_textures(&self, state: BlockState) -> Option<&FaceTextures> {
        let block: Box<dyn azalea_block::BlockTrait> = state.into();
        self.textures.get(block.id())
    }

    pub fn get_baked_model(&self, state: BlockState) -> Option<&BakedModel> {
        let block: Box<dyn azalea_block::BlockTrait> = state.into();
        let variants = self.baked.get(block.id())?;

        if variants.len() == 1 {
            return variants.values().next();
        }

        let variant_key = build_variant_key(&*block);
        variants
            .get(&variant_key)
            .or_else(|| variants.get(""))
            .or_else(|| variants.values().next())
    }

    pub fn is_opaque_full_cube(&self, state: BlockState) -> bool {
        if state.is_air() {
            return false;
        }
        self.get_baked_model(state)
            .map(|m| m.is_full_cube)
            .unwrap_or(false)
    }

    pub fn texture_names(&self) -> impl Iterator<Item = &str> + '_ {
        let face_textures = self.textures.values().flat_map(|ft| {
            let base = [
                &ft.top, &ft.bottom, &ft.north, &ft.south, &ft.east, &ft.west,
            ];
            base.into_iter()
                .map(|s| s.as_str())
                .chain(ft.side_overlay.as_deref())
        });

        let baked_textures = self.baked.values().flat_map(|variants| {
            variants
                .values()
                .flat_map(|model| model.quads.iter().map(|q| q.texture.as_str()))
        });

        face_textures.chain(baked_textures)
    }
}

fn build_variant_key(block: &dyn azalea_block::BlockTrait) -> String {
    let props = block.property_map();
    if props.is_empty() {
        return String::new();
    }
    let mut entries: Vec<_> = props.iter().collect();
    entries.sort_by_key(|(k, _)| *k);
    entries
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn load_cache(path: &Path) -> Option<HashMap<String, FaceTextures>> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_cache(path: &Path, textures: &HashMap<String, FaceTextures>) {
    if let Ok(json) = serde_json::to_string(textures) {
        if let Err(e) = std::fs::write(path, json) {
            log::warn!("Failed to write block cache: {e}");
        }
    }
}

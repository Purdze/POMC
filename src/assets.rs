use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn load_image(path: &Path) -> Result<image::DynamicImage, image::ImageError> {
    image::open(path).or_else(|_| {
        let data = std::fs::read(path).map_err(image::ImageError::IoError)?;
        image::load_from_memory(&data)
    })
}

pub struct AssetIndex {
    objects_dir: PathBuf,
    hashes: HashMap<String, String>,
}

impl AssetIndex {
    pub fn load(game_dir: &Path) -> Option<Self> {
        let assets_dir = game_dir.join("assets");
        let index_path = find_latest_asset_index(&assets_dir)?;

        let content = std::fs::read_to_string(&index_path)
            .map_err(|e| log::warn!("Failed to read asset index: {e}"))
            .ok()?;
        let parsed: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| log::warn!("Failed to parse asset index: {e}"))
            .ok()?;

        let objects = parsed.get("objects")?.as_object()?;
        let hashes = objects
            .iter()
            .filter_map(|(k, v)| {
                let hash = v.get("hash")?.as_str()?;
                Some((k.clone(), hash.to_owned()))
            })
            .collect();

        Some(Self {
            objects_dir: assets_dir.join("objects"),
            hashes,
        })
    }

    pub fn resolve(&self, asset_key: &str) -> Option<PathBuf> {
        let hash = self.hashes.get(asset_key)?;
        let path = self.objects_dir.join(&hash[..2]).join(hash);
        path.exists().then_some(path)
    }
}

fn find_latest_asset_index(assets_dir: &Path) -> Option<PathBuf> {
    let indexes_dir = assets_dir.join("indexes");
    let mut best: Option<(u32, PathBuf)> = None;

    let entries = std::fs::read_dir(&indexes_dir)
        .map_err(|e| log::warn!("Failed to read asset indexes dir: {e}"))
        .ok()?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Some(num_str) = name_str.strip_suffix(".json") {
            if let Ok(num) = num_str.parse::<u32>() {
                if best.as_ref().is_none_or(|(b, _)| num > *b) {
                    best = Some((num, entry.path()));
                }
            }
        }
    }

    best.map(|(_, path)| path)
}

use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const CURRENT_PACK_FORMAT: u32 = 84;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PackCompat {
    Compatible,
    TooOld,
    TooNew,
}

#[derive(Clone)]
pub struct PackInfo {
    pub name: String,
    pub description: String,
    pub compat: PackCompat,
    pub source: PackSource,
    pub enabled: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub enum PackSource {
    Server,
    Local,
}

enum PackStorage {
    Directory(PathBuf),
    Zip(Mutex<zip::ZipArchive<std::io::Cursor<Vec<u8>>>>),
}

struct ActivePack {
    id: String,
    source: PackSource,
    storage: PackStorage,
    info: PackInfo,
}

pub struct ResourcePackManager {
    packs_dir: PathBuf,
    server_cache_dir: PathBuf,
    active_packs: Vec<ActivePack>,
    available_local: Vec<PackInfo>,
}

impl ResourcePackManager {
    pub fn new(instance_dir: &Path) -> Self {
        let packs_dir = instance_dir.join("resourcepacks");
        let server_cache_dir = packs_dir.join(".server_cache");
        let _ = std::fs::create_dir_all(&packs_dir);
        let _ = std::fs::create_dir_all(&server_cache_dir);
        let mut mgr = Self {
            packs_dir,
            server_cache_dir,
            active_packs: Vec::new(),
            available_local: Vec::new(),
        };
        mgr.scan_local_packs();
        mgr
    }

    pub fn resolve_asset(&self, asset_key: &str) -> Option<PathBuf> {
        for pack in self.active_packs.iter().rev() {
            match &pack.storage {
                PackStorage::Directory(dir) => {
                    let path = dir.join("assets").join(asset_key);
                    if path.exists() {
                        return Some(path);
                    }
                }
                PackStorage::Zip(archive) => {
                    let zip_path = format!("assets/{asset_key}");
                    let mut archive = archive.lock().unwrap();
                    if let Ok(mut entry) = archive.by_name(&zip_path) {
                        let cache_path = self.server_cache_dir.join("_zip_cache").join(asset_key);
                        if let Some(parent) = cache_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Ok(mut out) = std::fs::File::create(&cache_path) {
                            let _ = std::io::copy(&mut entry, &mut out);
                            return Some(cache_path);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn has_active_packs(&self) -> bool {
        !self.active_packs.is_empty()
    }

    fn server_pack_dir(&self, hash: &str) -> PathBuf {
        self.server_cache_dir.join(hash)
    }

    fn is_server_cached(&self, hash: &str) -> bool {
        self.server_pack_dir(hash).is_dir()
    }

    pub async fn download_and_apply(
        &mut self,
        id: uuid::Uuid,
        url: &str,
        hash: &str,
    ) -> Result<(), PackError> {
        if !self.is_server_cached(hash) {
            let data = download_pack(url).await?;
            validate_hash(&data, hash)?;
            extract_zip(&data, &self.server_pack_dir(hash))?;
        }

        let pack_id = id.to_string();
        self.active_packs.retain(|p| p.id != pack_id);
        let dir = self.server_pack_dir(hash);
        let info = parse_pack_meta_dir(&dir, hash);
        self.active_packs.push(ActivePack {
            id: pack_id,
            source: PackSource::Server,
            storage: PackStorage::Directory(dir),
            info: PackInfo {
                enabled: true,
                source: PackSource::Server,
                ..info
            },
        });

        log::info!("Applied server resource pack {id} (hash: {hash})");
        Ok(())
    }

    pub fn remove_server_pack(&mut self, id: &uuid::Uuid) -> bool {
        let id_str = id.to_string();
        let before = self.active_packs.len();
        self.active_packs
            .retain(|p| !(p.id == id_str && p.source == PackSource::Server));
        let removed = self.active_packs.len() < before;
        if removed {
            log::info!("Removed server resource pack {id}");
        }
        removed
    }

    pub fn clear_server_packs(&mut self) {
        self.active_packs.retain(|p| p.source != PackSource::Server);
        log::info!("Cleared all server resource packs");
    }

    pub fn scan_local_packs(&mut self) {
        self.available_local.clear();
        let entries = match std::fs::read_dir(&self.packs_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path == self.server_cache_dir {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                if path.join("pack.mcmeta").exists() {
                    let already_active = self.active_packs.iter().any(|p| {
                        p.source == PackSource::Local
                            && matches!(&p.storage, PackStorage::Directory(d) if d == &path)
                    });
                    let mut info = parse_pack_meta_dir(&path, &name);
                    info.enabled = already_active;
                    info.source = PackSource::Local;
                    self.available_local.push(info);
                }
            } else if path.extension().is_some_and(|e| e == "zip")
                && let Some(mut info) = parse_pack_meta_zip(&path, &name)
            {
                let already_active = self
                    .active_packs
                    .iter()
                    .any(|p| p.source == PackSource::Local && p.id == name);
                info.enabled = already_active;
                info.source = PackSource::Local;
                self.available_local.push(info);
            }
        }
    }

    pub fn enable_local_pack(&mut self, name: &str) {
        let path = self.packs_dir.join(name);
        if path.is_dir() && path.join("pack.mcmeta").exists() {
            self.active_packs.retain(|p| p.id != name);
            let info = parse_pack_meta_dir(&path, name);
            self.active_packs.push(ActivePack {
                id: name.to_owned(),
                source: PackSource::Local,
                storage: PackStorage::Directory(path),
                info: PackInfo {
                    enabled: true,
                    source: PackSource::Local,
                    ..info
                },
            });
            log::info!("Enabled local resource pack: {name}");
        } else if path.extension().is_some_and(|e| e == "zip")
            && let Ok(data) = std::fs::read(&path)
        {
            let cursor = std::io::Cursor::new(data);
            if let Ok(archive) = zip::ZipArchive::new(cursor) {
                let info = parse_pack_meta_dir(&path, name);
                self.active_packs.retain(|p| p.id != name);
                self.active_packs.push(ActivePack {
                    id: name.to_owned(),
                    source: PackSource::Local,
                    storage: PackStorage::Zip(Mutex::new(archive)),
                    info: PackInfo {
                        name: name.to_owned(),
                        enabled: true,
                        source: PackSource::Local,
                        ..info
                    },
                });
                log::info!("Enabled local resource pack: {name}");
            }
        }
        self.scan_local_packs();
    }

    pub fn disable_local_pack(&mut self, name: &str) {
        self.active_packs
            .retain(|p| !(p.id == name && p.source == PackSource::Local));
        log::info!("Disabled local resource pack: {name}");
        self.scan_local_packs();
    }

    pub fn active_pack_info(&self) -> Vec<PackInfo> {
        self.active_packs.iter().map(|p| p.info.clone()).collect()
    }

    pub fn available_local_packs(&self) -> &[PackInfo] {
        &self.available_local
    }

    pub fn packs_dir(&self) -> &Path {
        &self.packs_dir
    }
}

#[derive(Debug)]
pub enum PackError {
    Download(String),
    HashMismatch,
    Extract(String),
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Download(e) => write!(f, "download failed: {e}"),
            Self::HashMismatch => write!(f, "SHA-1 hash mismatch"),
            Self::Extract(e) => write!(f, "extraction failed: {e}"),
        }
    }
}

async fn download_pack(url: &str) -> Result<Vec<u8>, PackError> {
    log::info!("Downloading resource pack from {url}");
    let resp = reqwest::get(url)
        .await
        .map_err(|e| PackError::Download(e.to_string()))?;
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| PackError::Download(e.to_string()))?;
    log::info!("Downloaded {} bytes", bytes.len());
    Ok(bytes.to_vec())
}

fn validate_hash(data: &[u8], expected: &str) -> Result<(), PackError> {
    if expected.is_empty() {
        return Ok(());
    }
    let actual = sha1_smol::Sha1::from(data).digest().to_string();
    if actual != expected {
        log::error!("Hash mismatch: expected {expected}, got {actual}");
        return Err(PackError::HashMismatch);
    }
    Ok(())
}

fn parse_meta_value(v: &serde_json::Value, fallback: &str) -> (String, String, PackCompat) {
    let pack = v.get("pack");

    let description = pack
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or(fallback)
        .to_owned();

    let name = pack
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or(fallback)
        .to_owned();

    let compat = pack
        .map(|p| {
            let (min, max) = parse_format_range(p);
            if CURRENT_PACK_FORMAT < min {
                PackCompat::TooNew
            } else if CURRENT_PACK_FORMAT > max {
                PackCompat::TooOld
            } else {
                PackCompat::Compatible
            }
        })
        .unwrap_or(PackCompat::Compatible);

    (name, description, compat)
}

fn parse_pack_meta_dir(dir: &Path, fallback_name: &str) -> PackInfo {
    let meta_path = dir.join("pack.mcmeta");
    let Some(v) = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
    else {
        return PackInfo {
            name: fallback_name.to_owned(),
            description: fallback_name.to_owned(),
            compat: PackCompat::Compatible,
            source: PackSource::Local,
            enabled: false,
        };
    };

    let (name, description, compat) = parse_meta_value(&v, fallback_name);
    PackInfo {
        name,
        description,
        compat,
        source: PackSource::Local,
        enabled: false,
    }
}

fn parse_pack_meta_zip(path: &Path, fallback_name: &str) -> Option<PackInfo> {
    let data = std::fs::read(path).ok()?;
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).ok()?;
    let mut mcmeta = archive.by_name("pack.mcmeta").ok()?;
    let mut contents = String::new();
    std::io::Read::read_to_string(&mut mcmeta, &mut contents).ok()?;
    let v: serde_json::Value = serde_json::from_str(&contents).ok()?;

    let (name, description, compat) = parse_meta_value(&v, fallback_name);
    Some(PackInfo {
        name,
        description,
        compat,
        source: PackSource::Local,
        enabled: false,
    })
}

fn parse_format_range(pack: &serde_json::Value) -> (u32, u32) {
    if let (Some(min), Some(max)) = (pack.get("min_format"), pack.get("max_format")) {
        let min_v = format_value(min);
        let max_v = format_value(max);
        if min_v > 0 && max_v > 0 {
            return (min_v, max_v);
        }
    }

    if let Some(supported) = pack.get("supported_formats") {
        if let Some(arr) = supported.as_array()
            && arr.len() == 2
        {
            return (
                arr[0].as_u64().unwrap_or(0) as u32,
                arr[1].as_u64().unwrap_or(0) as u32,
            );
        }
        if let Some(obj) = supported.as_object() {
            let min = obj
                .get("min_inclusive")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let max = obj
                .get("max_inclusive")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            return (min, max);
        }
        if let Some(n) = supported.as_u64() {
            return (n as u32, n as u32);
        }
    }

    if let Some(fmt) = pack.get("pack_format").and_then(|v| v.as_u64()) {
        return (fmt as u32, fmt as u32);
    }

    (0, u32::MAX)
}

fn format_value(v: &serde_json::Value) -> u32 {
    if let Some(n) = v.as_u64() {
        return n as u32;
    }
    if let Some(arr) = v.as_array()
        && let Some(major) = arr.first().and_then(|v| v.as_u64())
    {
        return major as u32;
    }
    0
}

fn extract_zip(data: &[u8], dest: &Path) -> Result<(), PackError> {
    let _ = std::fs::create_dir_all(dest);
    let cursor = std::io::Cursor::new(data);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| PackError::Extract(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| PackError::Extract(e.to_string()))?;

        let Some(enclosed) = file.enclosed_name() else {
            continue;
        };
        let out_path = dest.join(enclosed);

        if file.is_dir() {
            let _ = std::fs::create_dir_all(&out_path);
        } else {
            if let Some(parent) = out_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut out =
                std::fs::File::create(&out_path).map_err(|e| PackError::Extract(e.to_string()))?;
            std::io::copy(&mut file, &mut out).map_err(|e| PackError::Extract(e.to_string()))?;
        }
    }

    log::info!("Extracted resource pack to {}", dest.display());
    Ok(())
}

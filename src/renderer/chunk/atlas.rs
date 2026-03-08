use std::collections::{HashMap, HashSet};
use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AtlasError {
    #[error("failed to load texture {path}: {source}")]
    Load {
        path: String,
        source: image::ImageError,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

pub struct TextureAtlas {
    pub texture_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    regions: HashMap<String, AtlasRegion>,
    pub missing: AtlasRegion,
}

impl TextureAtlas {
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        assets_dir: &Path,
        texture_names: &HashSet<&str>,
    ) -> Result<Self, AtlasError> {
        let tile_size = 16u32;
        let grid_size = (texture_names.len() as f32 + 1.0).sqrt().ceil() as u32 + 1;
        let atlas_size = (grid_size * tile_size).next_power_of_two();

        let mut atlas_image = image::RgbaImage::new(atlas_size, atlas_size);
        let mut regions = HashMap::new();

        let missing_region =
            tile_region(tile_origin(0, grid_size, tile_size), tile_size, atlas_size);

        // Slot 0: magenta/black checkerboard for missing textures
        for py in 0..tile_size {
            for px in 0..tile_size {
                let is_check = ((px / 8) + (py / 8)) % 2 == 0;
                let color = if is_check {
                    image::Rgba([255, 0, 255, 255])
                } else {
                    image::Rgba([0, 0, 0, 255])
                };
                atlas_image.put_pixel(px, py, color);
            }
        }

        let textures_path = assets_dir.join("assets/minecraft/textures/block");
        let mut slot = 1u32;

        for &name in texture_names {
            let file_path = textures_path.join(format!("{name}.png"));
            if !file_path.exists() {
                log::warn!("Missing texture: {name}");
                regions.insert(name.to_string(), missing_region);
                continue;
            }

            let img = image::open(&file_path)
                .map_err(|e| AtlasError::Load {
                    path: file_path.display().to_string(),
                    source: e,
                })?
                .to_rgba8();

            let origin = tile_origin(slot, grid_size, tile_size);
            let region = tile_region(origin, tile_size, atlas_size);

            for py in 0..tile_size.min(img.height()) {
                for px in 0..tile_size.min(img.width()) {
                    atlas_image.put_pixel(origin.0 + px, origin.1 + py, *img.get_pixel(px, py));
                }
            }

            regions.insert(name.to_string(), region);
            slot += 1;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("block_atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * atlas_size),
                rows_per_image: Some(atlas_size),
            },
            wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        log::info!(
            "Built {atlas_size}x{atlas_size} texture atlas with {} textures",
            regions.len()
        );

        Ok(Self {
            texture_view,
            sampler,
            regions,
            missing: missing_region,
        })
    }

    pub fn get_region(&self, name: &str) -> AtlasRegion {
        self.regions.get(name).copied().unwrap_or(self.missing)
    }
}

fn tile_origin(slot: u32, grid_size: u32, tile_size: u32) -> (u32, u32) {
    (
        (slot % grid_size) * tile_size,
        (slot / grid_size) * tile_size,
    )
}

fn tile_region(origin: (u32, u32), tile_size: u32, atlas_size: u32) -> AtlasRegion {
    let s = atlas_size as f32;
    AtlasRegion {
        u_min: origin.0 as f32 / s,
        v_min: origin.1 as f32 / s,
        u_max: (origin.0 + tile_size) as f32 / s,
        v_max: (origin.1 + tile_size) as f32 / s,
    }
}

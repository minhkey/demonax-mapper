use anyhow::{Context, Result};
use dashmap::DashMap;
use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

pub struct SpriteCache {
    sprites: Arc<DashMap<u32, Arc<RgbaImage>>>,
    sprite_path: PathBuf,
    missing_sprite: Arc<RgbaImage>,
}

impl SpriteCache {
    pub fn new<P: AsRef<Path>>(sprite_path: P) -> Result<Self> {
        let sprite_path = sprite_path.as_ref().to_path_buf();

        if !sprite_path.exists() {
            anyhow::bail!("Sprite directory does not exist: {:?}", sprite_path);
        }

        let missing_sprite = Arc::new(Self::create_missing_sprite());

        Ok(Self {
            sprites: Arc::new(DashMap::new()),
            sprite_path,
            missing_sprite,
        })
    }

    pub fn get_sprite(&self, object_id: u32) -> Result<Arc<RgbaImage>> {
        if let Some(sprite) = self.sprites.get(&object_id) {
            return Ok(Arc::clone(&sprite));
        }

        match self.load_sprite_from_disk(object_id) {
            Ok(sprite) => {
                let sprite_arc = Arc::new(sprite);
                self.sprites.insert(object_id, Arc::clone(&sprite_arc));
                Ok(sprite_arc)
            }
            Err(e) => {
                warn!("Failed to load sprite {}: {}. Using placeholder", object_id, e);
                Ok(Arc::clone(&self.missing_sprite))
            }
        }
    }

    pub fn preload_sprites(&self, object_ids: &[u32]) -> Result<()> {
        use rayon::prelude::*;

        object_ids.par_iter().try_for_each(|&id| {
            self.get_sprite(id)?;
            Ok::<_, anyhow::Error>(())
        })?;

        Ok(())
    }

    pub fn cache_size(&self) -> usize {
        self.sprites.len()
    }

    fn load_sprite_from_disk(&self, object_id: u32) -> Result<RgbaImage> {
        let filename = format!("{}.png", object_id);
        let path = self.sprite_path.join(&filename);

        let img = image::open(&path)
            .with_context(|| format!("Failed to load sprite from {:?}", path))?;

        let rgba = img.to_rgba8();

        if rgba.width() != 32 || rgba.height() != 32 {
            warn!(
                "Sprite {} has unexpected dimensions: {}x{} (expected 32x32)",
                object_id,
                rgba.width(),
                rgba.height()
            );
        }

        Ok(rgba)
    }

    fn create_missing_sprite() -> RgbaImage {
        use image::Rgba;

        let mut img = RgbaImage::new(32, 32);

        for y in 0..32 {
            for x in 0..32 {
                let color = if (x / 8 + y / 8) % 2 == 0 {
                    Rgba([255, 0, 255, 255])
                } else {
                    Rgba([255, 105, 180, 255])
                };
                img.put_pixel(x, y, color);
            }
        }

        img
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_missing_sprite() {
        let sprite = SpriteCache::create_missing_sprite();
        assert_eq!(sprite.dimensions(), (32, 32));

        let top_left = sprite.get_pixel(0, 0);
        assert_eq!(top_left[0], 255);
    }
}

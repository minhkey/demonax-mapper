use crate::{sprites::SpriteCache, ObjectDatabase};
use anyhow::{Context, Result};
use image::{imageops, Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileStack {
    pub x: u32,
    pub y: u32,
    pub object_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteMapData {
    pub floor: u8,
    pub tiles: Vec<TileStack>,
    pub min_sector_x: u32,
    pub max_sector_x: u32,
    pub min_sector_y: u32,
    pub max_sector_y: u32,
    #[serde(default)]
    pub version: u32,
}

pub fn parse_sprite_map<P: AsRef<Path>>(
    game_path: P,
    floor: u8,
    global_min_sector_x: u32,
    global_min_sector_y: u32,
    global_max_sector_x: u32,
    global_max_sector_y: u32,
) -> Result<SpriteMapData> {
    let map_dir = game_path.as_ref().join("map");

    let sec_files: Vec<PathBuf> = fs::read_dir(&map_dir)
        .with_context(|| format!("Failed to read map directory: {:?}", map_dir))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| matches_pattern(n, floor))
                .unwrap_or(false)
        })
        .collect();


    let all_tiles: Vec<Vec<TileStack>> = sec_files
        .par_iter()
        .filter_map(|path| {
            match parse_sector_file_stacks(path, global_min_sector_x, global_min_sector_y) {
                Ok(tiles) => Some(tiles),
                Err(e) => {
                    tracing::warn!("Failed to parse sector {:?}: {}", path.file_name(), e);
                    None
                }
            }
        })
        .collect();

    let mut tiles: Vec<TileStack> = all_tiles.into_iter().flatten().collect();

    // Sort tiles for correct Z-ordering when sprites overlap across tiles
    // Y ascending (back to front), X ascending (left to right)
    // This ensures sprites farther away (lower Y, lower X) draw first
    tiles.sort_by_key(|t| (t.y, t.x));

    Ok(SpriteMapData {
        floor,
        tiles,
        min_sector_x: global_min_sector_x,
        max_sector_x: global_max_sector_x,
        min_sector_y: global_min_sector_y,
        max_sector_y: global_max_sector_y,
        version: 2,
    })
}

fn matches_pattern(filename: &str, floor: u8) -> bool {
    filename.ends_with(&format!("-{:02}.sec", floor))
}

fn parse_sector_coords(filename: &str) -> Option<(u32, u32, u8)> {
    let name = filename.strip_suffix(".sec")?;
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let x = parts[0].parse().ok()?;
    let y = parts[1].parse().ok()?;
    let z = parts[2].parse().ok()?;

    Some((x, y, z))
}


fn parse_sector_file_stacks(
    path: &Path,
    min_sector_x: u32,
    min_sector_y: u32,
) -> Result<Vec<TileStack>> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    let (sector_x, sector_y, _) = parse_sector_coords(filename)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse sector coordinates"))?;

    let content = String::from_utf8_lossy(&fs::read(path)?).into_owned();
    let mut tiles = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains("Content=") {
            continue;
        }

        if let Some((local_x, local_y, obj_ids)) = parse_content_line(line) {
            if !obj_ids.is_empty() {
                let world_x = (sector_x - min_sector_x) * 32 + local_x;
                let world_y = (sector_y - min_sector_y) * 32 + local_y;

                tiles.push(TileStack {
                    x: world_x,
                    y: world_y,
                    object_ids: obj_ids,
                });
            }
        }
    }

    Ok(tiles)
}

fn parse_content_line(line: &str) -> Option<(u32, u32, Vec<u32>)> {
    // Split only on the FIRST colon to avoid issues with String attributes containing colons
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() < 2 {
        return None;
    }

    let coords: Vec<&str> = parts[0].split('-').collect();
    if coords.len() != 2 {
        return None;
    }

    let local_x = coords[0].parse().ok()?;
    let local_y = coords[1].parse().ok()?;

    let content = parts[1];
    let start = content.find('{')?;
    let end = content.find('}')?;
    let ids_str = &content[start + 1..end];

    let obj_ids: Vec<u32> = ids_str
        .split(',')
        .filter_map(|s| {
            // Extract just the first token (object ID), ignoring attributes like String="..."
            let trimmed = s.trim();
            let id_part = trimmed.split_whitespace().next()?;
            id_part.parse().ok()
        })
        .collect();

    Some((local_x, local_y, obj_ids))
}

fn is_ground_flower(obj: &crate::objects::GameObject) -> bool {
    // Check if object is a planted flower/blossom (ground decoration)
    let name_lower = obj.name.to_lowercase();
    let is_flower = name_lower.contains("flower") || name_lower.contains("blossom");

    if !is_flower {
        return false;
    }

    // Must have only Unmove flag (or Unmove + Avoid)
    // This excludes flowery walls (have Hang), potted flowers (have other flags),
    // and flowers already in Bottom layer (have Bottom flag)
    let flags_set: HashSet<&str> = obj.flags.iter().map(|s: &String| s.as_str()).collect();

    (flags_set.len() == 1 && flags_set.contains("Unmove")) ||
    (flags_set.len() == 2 && flags_set.contains("Unmove") && flags_set.contains("Avoid"))
}

pub fn select_sprite_layers(obj_ids: &[u32], objects: &ObjectDatabase) -> Vec<u32> {
    let mut ground_layers = Vec::new();
    let mut clip_layers = Vec::new();
    let mut bottom_layers = Vec::new();
    let mut normal_layers = Vec::new();
    let mut top_layers = Vec::new();

    // Chest/container object IDs that should always be rendered (for quest chests)
    const CHEST_IDS: &[u32] = &[2543, 2546, 2550, 2551, 2552, 2555, 2560, 4445, 4830];

    for &id in obj_ids {
        let Some(obj) = objects.get(&id) else { continue };

        // Skip takeable items, except for chests/containers which should always be visible
        let is_chest = CHEST_IDS.contains(&id);
        let is_container = obj.flags.iter().any(|f| f == "Chest" || f == "Container");
        if obj.flags.iter().any(|f| f == "Take") && !is_chest && !is_container {
            continue;
        }

        // Classify by layer type
        if obj.is_ground || obj.flags.iter().any(|f| f == "Bank") {
            // Ground layer: is_ground=true OR has Bank flag (water/swamp)
            ground_layers.push(id);
        } else if obj.flags.iter().any(|f| f == "Clip") {
            // Clip layer: ground decorations (grass overlays, small details)
            clip_layers.push(id);
        } else if is_ground_flower(obj) {
            // Clip layer: planted flowers/blossoms (ground decorations)
            clip_layers.push(id);
        } else if obj.flags.iter().any(|f| f == "Top") {
            // Top layer: explicit Top flag (open doors, hangings)
            top_layers.push(id);
        } else if obj.flags.iter().any(|f| f == "Bottom" || f == "Text") {
            // Bottom layer: walls, closed doors, plant bases, signs/text
            bottom_layers.push(id);
        } else {
            // Normal layer: everything else
            normal_layers.push(id);
        }
    }

    // Combine in render order: Ground → Clip → Bottom → Normal → Top
    let mut layers = Vec::new();
    layers.extend(ground_layers);
    layers.extend(clip_layers);
    layers.extend(bottom_layers);
    layers.extend(normal_layers);
    layers.extend(top_layers);

    layers
}

pub fn generate_sprite_tiles<P: AsRef<Path>>(
    map_data: &SpriteMapData,
    sprite_cache: &SpriteCache,
    objects: &ObjectDatabase,
    output_path: P,
    floor: u8,
    min_zoom: u8,
    max_zoom: u8,
) -> Result<usize> {
    let output_path = output_path.as_ref();
    let map_width = (map_data.max_sector_x - map_data.min_sector_x + 1) * 32;
    let map_height = (map_data.max_sector_y - map_data.min_sector_y + 1) * 32;

    let mut total_tiles = 0;

    for zoom in min_zoom..=max_zoom {
        let n_tiles = render_sprite_zoom_level(
            map_data,
            sprite_cache,
            objects,
            output_path,
            floor,
            zoom,
            map_width,
            map_height,
        )?;
        total_tiles += n_tiles;
        debug!("Generated {} tiles for zoom level {}", n_tiles, zoom);
    }

    Ok(total_tiles)
}

fn render_sprite_zoom_level(
    map_data: &SpriteMapData,
    sprite_cache: &SpriteCache,
    objects: &ObjectDatabase,
    output_path: &Path,
    floor: u8,
    zoom: u8,
    map_width: u32,
    map_height: u32,
) -> Result<usize> {
    let scale = 2u32.pow(zoom as u32);
    let tile_size = 256u32;

    let num_tiles_x = (map_width * scale + tile_size - 1) / tile_size;
    let num_tiles_y = (map_height * scale + tile_size - 1) / tile_size;

    let zoom_dir = output_path.join(floor.to_string()).join(zoom.to_string());
    fs::create_dir_all(&zoom_dir)?;

    let tile_coords: Vec<(u32, u32)> = (0..num_tiles_x)
        .flat_map(|x| (0..num_tiles_y).map(move |y| (x, y)))
        .collect();

    tile_coords
        .par_iter()
        .try_for_each(|(x, y)| {
            render_single_sprite_tile(
                map_data,
                sprite_cache,
                objects,
                &zoom_dir,
                *x,
                *y,
                scale,
                map_width,
                map_height,
            )
        })?;

    Ok((num_tiles_x * num_tiles_y) as usize)
}

fn render_single_sprite_tile(
    map_data: &SpriteMapData,
    sprite_cache: &SpriteCache,
    objects: &ObjectDatabase,
    output_dir: &Path,
    tile_x: u32,
    tile_y: u32,
    scale: u32,
    map_width: u32,
    map_height: u32,
) -> Result<()> {
    const TILE_SIZE: u32 = 256;

    let mut output = RgbaImage::from_pixel(
        TILE_SIZE,
        TILE_SIZE,
        Rgba([0, 0, 0, 0]),
    );

    let tile_start_x = tile_x * TILE_SIZE / scale;
    let tile_start_y = tile_y * TILE_SIZE / scale;
    let tile_end_x = ((tile_x + 1) * TILE_SIZE / scale).min(map_width);
    let tile_end_y = ((tile_y + 1) * TILE_SIZE / scale).min(map_height);

    // Maximum sprite size is 64px, which translates to 64/scale game tiles when scaled
    let max_sprite_tiles = (64 + scale - 1) / scale;

    // Only process tiles that could possibly overlap with this output tile
    // A sprite at position (x,y) can extend up to max_sprite_tiles in each direction
    let search_start_x = tile_start_x.saturating_sub(max_sprite_tiles);
    let search_end_x = tile_end_x + max_sprite_tiles;
    let search_start_y = tile_start_y.saturating_sub(max_sprite_tiles);
    let search_end_y = tile_end_y + max_sprite_tiles;

    for tile_stack in &map_data.tiles {
        // Early filter: skip tiles that are definitely out of range
        if tile_stack.x < search_start_x || tile_stack.x >= search_end_x ||
           tile_stack.y < search_start_y || tile_stack.y >= search_end_y {
            continue;
        }

        // Debug logging for problematic coordinates (scale==4 is zoom 2)
        if scale == 4 && tile_x == 22 && tile_y == 15 && tile_stack.x >= 1408 && tile_stack.x <= 1415 && tile_stack.y >= 960 && tile_stack.y <= 965 {
            tracing::debug!("Tile ({}, {}) scale {}: processing tile_stack at ({}, {}) with objects {:?}",
                tile_x, tile_y, scale, tile_stack.x, tile_stack.y, tile_stack.object_ids);
        }

        let layers = select_sprite_layers(&tile_stack.object_ids, objects);

        // Debug logging for layer selection
        if scale == 4 && tile_x == 22 && tile_y == 15 && tile_stack.x >= 1408 && tile_stack.x <= 1415 && tile_stack.y >= 960 && tile_stack.y <= 965 {
            tracing::debug!("  -> Selected layers: {:?}", layers);
        }

        for &obj_id in &layers {
            // Use DisguiseTarget sprite if object has one
            let sprite_id = objects.get(&obj_id)
                .and_then(|obj| obj.disguise_target)
                .unwrap_or(obj_id);
            let sprite = sprite_cache.get_sprite(sprite_id)?;
            let scaled = scale_sprite(&*sprite, scale);
            let (sprite_width, sprite_height) = scaled.dimensions();

            let sprite_tiles_wide = (sprite_width + scale - 1) / scale;
            let sprite_tiles_high = (sprite_height + scale - 1) / scale;

            // The tile position is the ANCHOR POINT (bottom-right corner) of the sprite
            // For a 64x64 sprite (2x2 tiles), we need to offset by -1,-1 to get the top-left
            let sprite_top_left_x = tile_stack.x as i32 - (sprite_tiles_wide as i32 - 1);
            let sprite_top_left_y = tile_stack.y as i32 - (sprite_tiles_high as i32 - 1);

            // Calculate sprite bounds (keep as i32 to handle negative coordinates at boundaries)
            let sprite_end_x = sprite_top_left_x + sprite_tiles_wide as i32;
            let sprite_end_y = sprite_top_left_y + sprite_tiles_high as i32;

            // Overlap check using i32 to properly handle sprites at sector boundaries
            // Use <= for top-left checks to include sprites that start exactly at tile boundary
            if sprite_top_left_x <= tile_end_x as i32 && sprite_end_x > tile_start_x as i32 &&
               sprite_top_left_y <= tile_end_y as i32 && sprite_end_y > tile_start_y as i32 {

                let px = (sprite_top_left_x - tile_start_x as i32) * scale as i32;
                let py = (sprite_top_left_y - tile_start_y as i32) * scale as i32;

                overlay_with_alpha(&mut output, &scaled, px, py);
            }
        }
    }

    let x_dir = output_dir.join(tile_x.to_string());
    fs::create_dir_all(&x_dir)?;
    let tile_path = x_dir.join(format!("{}.png", tile_y));
    output.save(&tile_path)?;

    trace!("Rendered tile {}/{}", tile_x, tile_y);

    Ok(())
}

fn scale_sprite(sprite: &RgbaImage, target_size: u32) -> RgbaImage {
    let (width, height) = sprite.dimensions();

    let scale_factor = target_size as f32 / 32.0;

    let new_width = (width as f32 * scale_factor).round() as u32;
    let new_height = (height as f32 * scale_factor).round() as u32;

    if new_width == width && new_height == height {
        return (*sprite).clone();
    }

    imageops::resize(
        sprite,
        new_width,
        new_height,
        imageops::FilterType::Lanczos3,
    )
}

fn overlay_with_alpha(
    base: &mut RgbaImage,
    overlay: &RgbaImage,
    x_offset: i32,
    y_offset: i32,
) {
    let (overlay_width, overlay_height) = overlay.dimensions();
    let (base_width, base_height) = base.dimensions();

    for y in 0..overlay_height {
        for x in 0..overlay_width {
            let base_x = x_offset + x as i32;
            let base_y = y_offset + y as i32;

            if base_x >= 0 && base_x < base_width as i32 &&
               base_y >= 0 && base_y < base_height as i32 {
                let base_pixel = *base.get_pixel(base_x as u32, base_y as u32);
                let overlay_pixel = *overlay.get_pixel(x, y);

                let blended = alpha_blend(base_pixel, overlay_pixel);
                base.put_pixel(base_x as u32, base_y as u32, blended);
            }
        }
    }
}

fn alpha_blend(bottom: Rgba<u8>, top: Rgba<u8>) -> Rgba<u8> {
    let alpha_top = top[3] as f32 / 255.0;

    if alpha_top == 0.0 {
        return bottom;
    }

    if alpha_top == 1.0 {
        return top;
    }

    let alpha_bottom = bottom[3] as f32 / 255.0;
    let alpha_out = alpha_top + alpha_bottom * (1.0 - alpha_top);

    if alpha_out == 0.0 {
        return Rgba([0, 0, 0, 0]);
    }

    let blend_channel = |bottom_c: u8, top_c: u8| -> u8 {
        let bottom_f = bottom_c as f32 / 255.0;
        let top_f = top_c as f32 / 255.0;
        let out_f = (top_f * alpha_top + bottom_f * alpha_bottom * (1.0 - alpha_top)) / alpha_out;
        (out_f * 255.0) as u8
    };

    Rgba([
        blend_channel(bottom[0], top[0]),
        blend_channel(bottom[1], top[1]),
        blend_channel(bottom[2], top[2]),
        (alpha_out * 255.0) as u8,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_blend_transparent() {
        let bottom = Rgba([100, 100, 100, 255]);
        let top = Rgba([200, 0, 0, 0]);
        let result = alpha_blend(bottom, top);
        assert_eq!(result, bottom);
    }

    #[test]
    fn test_alpha_blend_opaque() {
        let bottom = Rgba([100, 100, 100, 255]);
        let top = Rgba([200, 0, 0, 255]);
        let result = alpha_blend(bottom, top);
        assert_eq!(result, top);
    }

    #[test]
    fn test_alpha_blend_partial() {
        let bottom = Rgba([100, 100, 100, 255]);
        let top = Rgba([200, 0, 0, 128]);
        let result = alpha_blend(bottom, top);

        assert!(result[0] > 100 && result[0] < 200);
        assert_eq!(result[3], 255);
    }
}

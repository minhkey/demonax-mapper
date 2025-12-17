use crate::ObjectDatabase;
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub object_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapData {
    pub floor: u8,
    pub tiles: Vec<Tile>,
    pub min_sector_x: u32,
    pub max_sector_x: u32,
    pub min_sector_y: u32,
    pub max_sector_y: u32,
}

pub fn parse_map<P: AsRef<Path>>(
    game_path: P,
    floor: u8,
    objects: &ObjectDatabase,
) -> Result<MapData> {
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

    let (min_sector_x, max_sector_x, min_sector_y, max_sector_y) =
        calculate_bounds(&sec_files, floor)?;

    let all_tiles: Vec<Vec<Tile>> = sec_files
        .par_iter()
        .filter_map(|path| parse_sector_file(path, min_sector_x, min_sector_y, objects).ok())
        .collect();

    let tiles: Vec<Tile> = all_tiles.into_iter().flatten().collect();

    Ok(MapData {
        floor,
        tiles,
        min_sector_x,
        max_sector_x,
        min_sector_y,
        max_sector_y,
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

fn calculate_bounds(files: &[PathBuf], floor: u8) -> Result<(u32, u32, u32, u32)> {
    let mut min_x = u32::MAX;
    let mut max_x = 0;
    let mut min_y = u32::MAX;
    let mut max_y = 0;

    for path in files {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if let Some((x, y, z)) = parse_sector_coords(filename) {
                if z == floor {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
    }

    Ok((min_x, max_x, min_y, max_y))
}

fn parse_sector_file(
    path: &Path,
    min_sector_x: u32,
    min_sector_y: u32,
    objects: &ObjectDatabase,
) -> Result<Vec<Tile>> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    let (sector_x, sector_y, _) = parse_sector_coords(filename)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse sector coordinates"))?;

    let content = fs::read_to_string(path)?;
    let mut tiles = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains("Content=") {
            continue;
        }

        if let Some((local_x, local_y, obj_ids)) = parse_content_line(line) {
            if let Some(display_id) = select_display_object(&obj_ids, objects) {
                let world_x = (sector_x - min_sector_x) * 32 + local_x;
                let world_y = (sector_y - min_sector_y) * 32 + local_y;

                tiles.push(Tile {
                    x: world_x,
                    y: world_y,
                    object_id: display_id,
                });
            }
        }
    }

    Ok(tiles)
}

fn parse_content_line(line: &str) -> Option<(u32, u32, Vec<u32>)> {
    let parts: Vec<&str> = line.split(':').collect();
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
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    Some((local_x, local_y, obj_ids))
}

fn select_display_object(obj_ids: &[u32], objects: &ObjectDatabase) -> Option<u32> {
    if obj_ids.is_empty() {
        return None;
    }

    for &id in obj_ids.iter().rev() {
        if let Some(obj) = objects.get(&id) {
            if obj.is_impassable {
                return Some(id);
            }
        }
    }

    for &id in obj_ids {
        if let Some(obj) = objects.get(&id) {
            if obj.is_ground {
                return Some(id);
            }
        }
    }

    Some(obj_ids[0])
}

use crate::{ColorMap, MapData, Rgb};
use anyhow::Result;
use image::{ImageBuffer, Rgb as ImageRgb, RgbImage};
use rayon::prelude::*;
use std::fs;
use std::path::Path;

const TILE_SIZE: u32 = 256;

pub fn generate_tiles<P: AsRef<Path>>(
    map_data: &MapData,
    color_map: &ColorMap,
    output_path: P,
    floor: u8,
    min_zoom: u8,
    max_zoom: u8,
) -> Result<usize> {
    let width_sectors = map_data.max_sector_x - map_data.min_sector_x + 1;
    let height_sectors = map_data.max_sector_y - map_data.min_sector_y + 1;
    let width_tiles = width_sectors * 32;
    let height_tiles = height_sectors * 32;

    let mut total_tiles = 0;

    for zoom in min_zoom..=max_zoom {
        let tiles_generated = render_zoom_level(
            map_data,
            color_map,
            &output_path,
            floor,
            zoom,
            width_tiles,
            height_tiles,
        )?;
        total_tiles += tiles_generated;
    }

    Ok(total_tiles)
}

fn render_zoom_level<P: AsRef<Path>>(
    map_data: &MapData,
    color_map: &ColorMap,
    output_path: P,
    floor: u8,
    zoom: u8,
    map_width: u32,
    map_height: u32,
) -> Result<usize> {
    let scale = 2u32.pow(zoom as u32);
    let tile_width = (map_width * scale + TILE_SIZE - 1) / TILE_SIZE;
    let tile_height = (map_height * scale + TILE_SIZE - 1) / TILE_SIZE;

    let zoom_dir = output_path
        .as_ref()
        .join(floor.to_string())
        .join(zoom.to_string());
    fs::create_dir_all(&zoom_dir)?;

    let tile_coords: Vec<(u32, u32)> = (0..tile_width)
        .flat_map(|x| (0..tile_height).map(move |y| (x, y)))
        .collect();

    tile_coords
        .par_iter()
        .try_for_each(|(x, y)| {
            render_single_tile(
                map_data,
                color_map,
                &zoom_dir,
                *x,
                *y,
                zoom,
                map_width,
                map_height,
                scale,
            )
        })?;

    Ok((tile_width * tile_height) as usize)
}

fn render_single_tile(
    map_data: &MapData,
    color_map: &ColorMap,
    output_dir: &Path,
    tile_x: u32,
    tile_y: u32,
    _zoom: u8,
    map_width: u32,
    map_height: u32,
    scale: u32,
) -> Result<()> {
    let pixels_per_tile = scale;
    let tile_start_x = tile_x * TILE_SIZE / pixels_per_tile;
    let tile_start_y = tile_y * TILE_SIZE / pixels_per_tile;
    let tile_end_x = ((tile_x + 1) * TILE_SIZE / pixels_per_tile).min(map_width);
    let tile_end_y = ((tile_y + 1) * TILE_SIZE / pixels_per_tile).min(map_height);

    let mut img: RgbImage = ImageBuffer::from_pixel(TILE_SIZE, TILE_SIZE, ImageRgb([0, 0, 0]));

    for tile in &map_data.tiles {
        if tile.x >= tile_start_x
            && tile.x < tile_end_x
            && tile.y >= tile_start_y
            && tile.y < tile_end_y
        {
            let color = color_map
                .get(&tile.object_id)
                .copied()
                .unwrap_or(Rgb::new(0, 0, 0));

            let px_start_x = (tile.x - tile_start_x) * pixels_per_tile;
            let py_start_y = (tile.y - tile_start_y) * pixels_per_tile;

            for dy in 0..pixels_per_tile {
                for dx in 0..pixels_per_tile {
                    let px = px_start_x + dx;
                    let py = py_start_y + dy;

                    if px < TILE_SIZE && py < TILE_SIZE {
                        img.put_pixel(px, py, ImageRgb([color.r, color.g, color.b]));
                    }
                }
            }
        }
    }

    let x_dir = output_dir.join(tile_x.to_string());
    fs::create_dir_all(&x_dir)?;

    let tile_path = x_dir.join(format!("{}.png", tile_y));
    img.save(&tile_path)?;

    Ok(())
}

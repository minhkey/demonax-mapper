use crate::ObjectDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub type ColorMap = HashMap<u32, Rgb>;

pub fn get_tibia_palette() -> HashMap<u8, Rgb> {
    [
        (0x00, Rgb::new(0, 0, 0)),           // black (unexplored)
        (0x0C, Rgb::new(0, 128, 0)),         // dark green (trees)
        (0x18, Rgb::new(0, 255, 0)),         // light green (grass)
        (0x33, Rgb::new(0, 128, 255)),       // blue (water)
        (0x56, Rgb::new(128, 128, 128)),     // dark gray (rock/mountain)
        (0x72, Rgb::new(139, 69, 19)),       // dark brown (earth/stalagmite)
        (0x79, Rgb::new(165, 42, 42)),       // brown (earth)
        (0x81, Rgb::new(192, 192, 192)),     // gray (stone tile)
        (0x8C, Rgb::new(144, 238, 144)),     // light green variant
        (0xB3, Rgb::new(173, 216, 230)),     // light blue (ice)
        (0xBA, Rgb::new(255, 0, 0)),         // red (wall)
        (0xC0, Rgb::new(255, 140, 0)),       // orange (lava)
        (0xCF, Rgb::new(245, 222, 179)),     // beige (sand)
        (0xD2, Rgb::new(255, 255, 0)),       // yellow (ladder/stairs)
        (0xD7, Rgb::new(255, 255, 255)),     // white (snow)
    ]
    .into_iter()
    .collect()
}

pub fn create_color_map(objects: &ObjectDatabase) -> ColorMap {
    objects
        .iter()
        .map(|(id, obj)| {
            let color = object_name_to_color(&obj.name, obj.is_ground, obj.is_impassable);
            (*id, color)
        })
        .collect()
}

fn object_name_to_color(name: &str, is_ground: bool, is_impassable: bool) -> Rgb {
    let name_lower = name.to_lowercase();

    if name_lower.contains("water") || name_lower.contains("sea") {
        return Rgb::new(33, 66, 99);
    }

    if name_lower.contains("swamp") {
        return Rgb::new(64, 80, 48);
    }

    if name_lower.contains("tar") {
        return Rgb::new(32, 32, 32);
    }

    if name_lower.contains("lava") {
        return Rgb::new(255, 140, 0);
    }

    if name_lower.contains("sand") || name_lower.contains("desert") {
        return Rgb::new(210, 180, 140);
    }

    if name_lower.contains("snow") || name_lower.contains("ice") {
        return Rgb::new(230, 240, 250);
    }

    if name_lower.contains("grass") {
        return Rgb::new(0, 255, 0);
    }

    if is_impassable {
        if name_lower.contains("wall") || name_lower.contains("brick") {
            return Rgb::new(255, 0, 0);
        }
        if name_lower.contains("tree") || name_lower.contains("trunk") {
            return Rgb::new(0, 128, 0);
        }
        if name_lower.contains("mountain") || name_lower.contains("stone") || name_lower.contains("rock") {
            return Rgb::new(128, 128, 128);
        }
        return Rgb::new(192, 192, 192);
    }

    if is_ground {
        if name_lower.contains("dirt") || name_lower.contains("earth") || name_lower.contains("soil") {
            return Rgb::new(165, 42, 42);
        }
        if name_lower.contains("gravel") {
            return Rgb::new(128, 128, 128);
        }
        if name_lower.contains("floor") || name_lower.contains("pavement") || name_lower.contains("cobble") {
            return Rgb::new(169, 169, 169);
        }
        return Rgb::new(144, 238, 144);
    }

    Rgb::new(0, 0, 0)
}

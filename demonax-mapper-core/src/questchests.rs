use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestChest {
    pub quest_number: u32,
    pub x: u32,
    pub y: u32,
    pub z: u8,
    pub chest_object_id: u32,
    pub quest_name: Option<String>,
}

#[derive(Serialize)]
struct QuestChestOutput {
    quest_number: u32,
    x: u32,
    y: u32,
    quest_name: Option<String>,
}

pub fn parse_quest_csv<P: AsRef<Path>>(csv_path: P) -> Result<HashMap<u32, String>> {
    let content = fs::read_to_string(csv_path.as_ref())
        .with_context(|| format!("Failed to read quest CSV from {:?}", csv_path.as_ref()))?;

    let mut quest_names = HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        if line_num == 0 {
            continue;
        }

        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() < 2 {
            tracing::warn!("Line {}: Invalid CSV format", line_num + 1);
            continue;
        }

        let quest_value = match parts[0].trim().parse::<u32>() {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Line {}: Failed to parse quest_value: {}", line_num + 1, e);
                continue;
            }
        };

        let quest_name = parts[1].trim().to_string();

        quest_names.insert(quest_value, quest_name);
    }

    tracing::info!("Loaded {} quest names from CSV", quest_names.len());
    Ok(quest_names)
}

pub fn parse_questchests_from_sectors<P: AsRef<Path>>(
    map_dir: P,
    floors: &[u8],
    quest_names: &HashMap<u32, String>,
) -> Result<Vec<QuestChest>> {
    let map_dir = map_dir.as_ref();
    let mut quest_chests = Vec::new();

    for entry in fs::read_dir(map_dir)
        .with_context(|| format!("Failed to read map directory: {:?}", map_dir))?
    {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(f) => f,
            None => continue,
        };

        if !filename.ends_with(".sec") {
            continue;
        }

        let (sector_x, sector_y, z) = match parse_sector_coords(filename) {
            Some(coords) => coords,
            None => continue,
        };

        if !floors.contains(&z) {
            continue;
        }

        let content = match fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => {
                tracing::warn!("Failed to read {:?}: {}", path, e);
                continue;
            }
        };

        for (line_num, line) in content.lines().enumerate() {
            if !line.contains("ChestQuestNumber=") {
                continue;
            }

            match parse_questchest_line(line, sector_x, sector_y, z, quest_names) {
                Ok(Some(chest)) => quest_chests.push(chest),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        "{}:{}: Failed to parse quest chest: {}",
                        filename,
                        line_num + 1,
                        e
                    );
                }
            }
        }
    }

    tracing::info!(
        "Parsed {} quest chests from .sec files",
        quest_chests.len()
    );
    Ok(quest_chests)
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

fn parse_questchest_line(
    line: &str,
    sector_x: u32,
    sector_y: u32,
    z: u8,
    quest_names: &HashMap<u32, String>,
) -> Result<Option<QuestChest>> {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() < 2 {
        return Ok(None);
    }

    let coords: Vec<&str> = parts[0].split('-').collect();
    if coords.len() != 2 {
        return Ok(None);
    }

    let local_x: u32 = coords[0]
        .trim()
        .parse()
        .with_context(|| format!("Failed to parse local X coordinate: {}", coords[0]))?;

    let local_y: u32 = coords[1]
        .trim()
        .parse()
        .with_context(|| format!("Failed to parse local Y coordinate: {}", coords[1]))?;

    let content_part = parts[1];

    let quest_number = extract_quest_number(content_part)?;

    let chest_object_id = extract_chest_object_id(content_part).unwrap_or(0);

    let world_x = sector_x * 32 + local_x;
    let world_y = sector_y * 32 + local_y;

    let quest_name = quest_names.get(&quest_number).cloned();

    Ok(Some(QuestChest {
        quest_number,
        x: world_x,
        y: world_y,
        z,
        chest_object_id,
        quest_name,
    }))
}

fn extract_quest_number(content: &str) -> Result<u32> {
    let prefix = "ChestQuestNumber=";
    let start = content
        .find(prefix)
        .with_context(|| "ChestQuestNumber= not found")?;

    let value_start = start + prefix.len();
    let rest = &content[value_start..];

    let number_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();

    number_str
        .parse()
        .with_context(|| format!("Failed to parse quest number: {}", number_str))
}

fn extract_chest_object_id(content: &str) -> Option<u32> {
    let content_start = content.find("Content={")?;
    let ids_str = &content[content_start + 9..];
    let first_close = ids_str.find('}')?;
    let ids_part = &ids_str[..first_close];

    let items: Vec<&str> = ids_part.split(',').collect();
    for item in items.iter().take(3) {
        let trimmed = item.trim();
        let id_part = trimmed.split_whitespace().next()?;
        if let Ok(id) = id_part.parse::<u32>() {
            if id >= 2543 && id <= 2560 {
                return Some(id);
            }
        }
    }

    None
}

pub fn generate_questchests_json(
    chests: &[QuestChest],
    floors: &[u8],
) -> Result<String> {
    let mut chests_by_floor: HashMap<u8, Vec<QuestChestOutput>> = HashMap::new();

    for chest in chests {
        if floors.contains(&chest.z) {
            let chest_output = QuestChestOutput {
                quest_number: chest.quest_number,
                x: chest.x,
                y: chest.y,
                quest_name: chest.quest_name.clone(),
            };

            chests_by_floor
                .entry(chest.z)
                .or_insert_with(Vec::new)
                .push(chest_output);
        }
    }

    let output = serde_json::json!({
        "questchests_by_floor": chests_by_floor
    });

    let json = serde_json::to_string(&output)
        .with_context(|| "Failed to serialize quest chest data to JSON")?;

    Ok(json)
}

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterSpawn {
    pub race: u32,
    pub x: u32,
    pub y: u32,
    pub z: u8,
    pub radius: u32,
    pub amount: u32,
    pub regen: u32,
}

pub fn parse_monster_db<P: AsRef<Path>>(path: P) -> Result<Vec<MonsterSpawn>> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read monster.db from {:?}", path.as_ref()))?;

    let mut spawns = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let mut line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Strip inline comments
        if let Some(comment_pos) = line.find('#') {
            line = line[..comment_pos].trim();
            if line.is_empty() {
                continue;
            }
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 7 {
            // Skip the "0" end marker line
            if parts.len() == 1 && parts[0] == "0" {
                tracing::debug!("Found end marker at line {}", line_num + 1);
                break;
            }
            tracing::warn!(
                "Line {}: Invalid monster.db format, expected 7 fields, got {}",
                line_num + 1,
                parts.len()
            );
            continue;
        }

        let race = parts[0].parse::<u32>().with_context(|| {
            format!(
                "Line {}: Failed to parse race ID '{}'",
                line_num + 1,
                parts[0]
            )
        })?;

        let x = parts[1].parse::<u32>().with_context(|| {
            format!("Line {}: Failed to parse X coordinate '{}'", line_num + 1, parts[1])
        })?;

        let y = parts[2].parse::<u32>().with_context(|| {
            format!("Line {}: Failed to parse Y coordinate '{}'", line_num + 1, parts[2])
        })?;

        let z = parts[3].parse::<u8>().with_context(|| {
            format!("Line {}: Failed to parse Z coordinate '{}'", line_num + 1, parts[3])
        })?;

        let radius = parts[4].parse::<u32>().with_context(|| {
            format!("Line {}: Failed to parse radius '{}'", line_num + 1, parts[4])
        })?;

        let amount = parts[5].parse::<u32>().with_context(|| {
            format!("Line {}: Failed to parse amount '{}'", line_num + 1, parts[5])
        })?;

        let regen = parts[6].parse::<u32>().with_context(|| {
            format!("Line {}: Failed to parse regen '{}'", line_num + 1, parts[6])
        })?;

        spawns.push(MonsterSpawn {
            race,
            x,
            y,
            z,
            radius,
            amount,
            regen,
        });
    }

    tracing::info!("Parsed {} monster spawns from monster.db", spawns.len());
    Ok(spawns)
}

pub fn parse_monster_names<P: AsRef<Path>>(mon_dir: P) -> Result<HashMap<u32, String>> {
    let mon_dir = mon_dir.as_ref();
    let mut monster_names = HashMap::new();

    let entries = fs::read_dir(mon_dir)
        .with_context(|| format!("Failed to read monster directory: {:?}", mon_dir))?;

    for entry_result in entries {
        let entry = entry_result?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("mon") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read .mon file: {:?}", path))?;

        let mut race_number: Option<u32> = None;
        let mut name: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("RaceNumber") {
                if let Some(value) = line.split('=').nth(1) {
                    race_number = value.trim().parse().ok();
                }
            } else if line.starts_with("Name") {
                if let Some(value) = line.split('=').nth(1) {
                    name = Some(value.trim().trim_matches('"').to_string());
                }
            }

            if race_number.is_some() && name.is_some() {
                break;
            }
        }

        if let (Some(race_id), Some(monster_name)) = (race_number, name) {
            monster_names.insert(race_id, monster_name);
        } else {
            tracing::warn!("Incomplete monster data in file: {:?}", path);
        }
    }

    tracing::info!("Loaded {} monster names from .mon files", monster_names.len());
    Ok(monster_names)
}

#[derive(Serialize)]
struct SpawnOutput {
    race: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    x: u32,
    y: u32,
    amount: u32,
    radius: u32,
}

pub fn generate_spawn_json(
    spawns: &[MonsterSpawn],
    floors: &[u8],
    monster_names: &HashMap<u32, String>,
) -> Result<String> {
    let mut spawns_by_floor: HashMap<u8, Vec<SpawnOutput>> = HashMap::new();

    for spawn in spawns {
        if floors.contains(&spawn.z) {
            let spawn_output = SpawnOutput {
                race: spawn.race,
                name: monster_names.get(&spawn.race).cloned(),
                x: spawn.x,
                y: spawn.y,
                amount: spawn.amount,
                radius: spawn.radius,
            };

            spawns_by_floor
                .entry(spawn.z)
                .or_insert_with(Vec::new)
                .push(spawn_output);
        }
    }

    let output = serde_json::json!({
        "spawns_by_floor": spawns_by_floor
    });

    let json = serde_json::to_string(&output)
        .with_context(|| "Failed to serialize spawn data to JSON")?;

    Ok(json)
}

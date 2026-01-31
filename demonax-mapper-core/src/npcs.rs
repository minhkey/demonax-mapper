use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcLocation {
    pub id: i32,
    pub file_name: String,
    pub npc_name: String,
    pub x: u32,
    pub y: u32,
    pub z: u8,
}

pub fn parse_npc_csv<P: AsRef<Path>>(csv_path: P) -> Result<Vec<NpcLocation>> {
    let content = fs::read_to_string(csv_path.as_ref())
        .with_context(|| format!("Failed to read NPC CSV: {:?}", csv_path.as_ref()))?;

    let mut npcs = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        // Skip header line
        if line_num == 0 {
            continue;
        }

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Split on comma, limit to 6 parts to allow commas in npc_name
        let parts: Vec<&str> = line.splitn(6, ',').collect();

        if parts.len() < 6 {
            tracing::warn!("Line {}: Invalid CSV format, expected 6 fields, got {}",
                line_num + 1, parts.len());
            continue;
        }

        let id = match parts[0].trim().parse::<i32>() {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Line {}: Failed to parse id '{}': {}",
                    line_num + 1, parts[0], e);
                continue;
            }
        };

        let file_name = parts[1].trim().trim_matches('"').to_string();
        let npc_name = parts[2].trim().trim_matches('"').to_string();

        let x = match parts[3].trim().parse::<u32>() {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Line {}: Failed to parse x '{}': {}",
                    line_num + 1, parts[3], e);
                continue;
            }
        };

        let y = match parts[4].trim().parse::<u32>() {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Line {}: Failed to parse y '{}': {}",
                    line_num + 1, parts[4], e);
                continue;
            }
        };

        let z = match parts[5].trim().parse::<u8>() {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Line {}: Failed to parse z '{}': {}",
                    line_num + 1, parts[5], e);
                continue;
            }
        };

        npcs.push(NpcLocation {
            id,
            file_name,
            npc_name,
            x,
            y,
            z,
        });
    }

    tracing::info!("Parsed {} NPCs from CSV", npcs.len());
    Ok(npcs)
}

#[derive(Serialize)]
struct NpcOutput {
    id: i32,
    file_name: String,
    npc_name: String,
    x: u32,
    y: u32,
}

pub fn generate_npc_json(npcs: &[NpcLocation], floors: &[u8]) -> Result<String> {
    let mut npcs_by_floor: HashMap<u8, Vec<NpcOutput>> = HashMap::new();

    for npc in npcs {
        if floors.contains(&npc.z) {
            let npc_output = NpcOutput {
                id: npc.id,
                file_name: npc.file_name.clone(),
                npc_name: npc.npc_name.clone(),
                x: npc.x,
                y: npc.y,
            };

            npcs_by_floor
                .entry(npc.z)
                .or_insert_with(Vec::new)
                .push(npc_output);
        }
    }

    let output = serde_json::json!({
        "npcs_by_floor": npcs_by_floor
    });

    let json = serde_json::to_string(&output)
        .with_context(|| "Failed to serialize NPC data to JSON")?;

    Ok(json)
}

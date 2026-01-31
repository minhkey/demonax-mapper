use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

pub fn parse_npc_db<P: AsRef<Path>>(path: P) -> Result<Vec<NpcLocation>> {
    let conn = Connection::open(path.as_ref())
        .with_context(|| format!("Failed to open NPC database: {:?}", path.as_ref()))?;

    let mut stmt = conn
        .prepare("SELECT id, file_name, npc_name, x, y, z FROM npc_locations")
        .with_context(|| "Failed to prepare SQL query for npc_locations table")?;

    let npc_iter = stmt
        .query_map([], |row| {
            Ok(NpcLocation {
                id: row.get(0)?,
                file_name: row.get(1)?,
                npc_name: row.get(2)?,
                x: row.get(3)?,
                y: row.get(4)?,
                z: row.get(5)?,
            })
        })
        .with_context(|| "Failed to query npc_locations table")?;

    let mut npcs = Vec::new();
    for (idx, npc_result) in npc_iter.enumerate() {
        match npc_result {
            Ok(npc) => npcs.push(npc),
            Err(e) => {
                tracing::warn!("Skipping invalid NPC row {}: {}", idx + 1, e);
            }
        }
    }

    tracing::info!("Parsed {} NPCs from database", npcs.len());
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

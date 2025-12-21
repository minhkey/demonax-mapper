use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameObject {
    pub id: u32,
    pub name: String,
    pub flags: Vec<String>,
    pub waypoints: u32,
    pub is_ground: bool,
    pub is_impassable: bool,
    pub disguise_target: Option<u32>,
}

pub type ObjectDatabase = HashMap<u32, GameObject>;

pub fn parse_objects<P: AsRef<Path>>(path: P) -> Result<ObjectDatabase> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read objects file: {:?}", path.as_ref()))?;

    let lines: Vec<&str> = content.lines().collect();
    let type_id_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with("TypeID"))
        .map(|(i, _)| i)
        .collect();

    let mut objects = HashMap::with_capacity(type_id_indices.len());

    for (idx, &start) in type_id_indices.iter().enumerate() {
        let end = type_id_indices
            .get(idx + 1)
            .copied()
            .unwrap_or(lines.len());

        let obj = parse_object_block(&lines[start..end])?;
        objects.insert(obj.id, obj);
    }

    Ok(objects)
}

fn parse_object_block(lines: &[&str]) -> Result<GameObject> {
    let mut id = 0;
    let mut name = String::new();
    let mut flags = Vec::new();
    let mut waypoints = 0;
    let mut disguise_target = None;

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(value) = line.strip_prefix("TypeID") {
            let value = value.trim().trim_start_matches('=').trim();
            let value = value.split('#').next().unwrap_or(value).trim();
            id = value.parse().context("Failed to parse TypeID")?;
        } else if let Some(value) = line.strip_prefix("Name") {
            name = value
                .trim()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"')
                .to_string();
        } else if let Some(value) = line.strip_prefix("Flags") {
            let value = value.trim().trim_start_matches('=').trim();
            let value = value.trim_matches(|c| c == '{' || c == '}');
            flags = value.split(',').map(|s| s.trim().to_string()).collect();
        } else if let Some(value) = line.strip_prefix("Attributes") {
            let value = value.trim().trim_start_matches('=').trim();
            if let Some(wp) = extract_waypoints(value) {
                waypoints = wp;
            }
            if let Some(dt) = extract_disguise_target(value) {
                disguise_target = Some(dt);
            }
        }
    }

    let has_unpass = flags.iter().any(|f| f == "Unpass");
    let is_ground = waypoints > 0 && !has_unpass;
    let is_impassable = has_unpass || waypoints == 0;

    Ok(GameObject {
        id,
        name,
        flags,
        waypoints,
        is_ground,
        is_impassable,
        disguise_target,
    })
}

fn extract_waypoints(attributes: &str) -> Option<u32> {
    attributes
        .split(',')
        .find(|s| s.contains("Waypoints"))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| s.trim().trim_matches('}').parse().ok())
}

fn extract_disguise_target(attributes: &str) -> Option<u32> {
    attributes
        .split(',')
        .find(|s| s.contains("DisguiseTarget"))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| s.trim().trim_matches('}').parse().ok())
}

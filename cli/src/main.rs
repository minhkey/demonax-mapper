use anyhow::Result;
use clap::{Parser, Subcommand};
use demonax_mapper_core::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::fs;

#[derive(Parser)]
#[command(name = "demonax-mapper")]
#[command(about = "Generate map tiles for Demonax game server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    ParseObjects {
        #[arg(help = "Path to objects.srv file")]
        input: PathBuf,

        #[arg(short, long, default_value = ".demonax-cache/objects.json")]
        output: PathBuf,
    },

    Build {
        #[arg(help = "Path to game directory")]
        game_path: PathBuf,

        #[arg(short, long, help = "Path to sprite directory")]
        sprite_path: PathBuf,

        #[arg(short, long, default_value = "output")]
        output: PathBuf,

        #[arg(short, long, help = "Floors to generate (e.g. 0-15 or 7)")]
        floors: String,

        #[arg(long, default_value = "0")]
        min_zoom: u8,

        #[arg(long, default_value = "5")]
        max_zoom: u8,

        #[arg(long, help = "Path to demonax-data repository (for monster.db)")]
        data_path: Option<PathBuf>,

        #[arg(long, help = "Path to monster sprite directory (PNG files named by race ID)")]
        monster_sprites: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    match cli.command {
        Commands::ParseObjects { input, output } => {
            cmd_parse_objects(input, output)?;
        }
        Commands::Build {
            game_path,
            sprite_path,
            output,
            floors,
            min_zoom,
            max_zoom,
            data_path,
            monster_sprites,
        } => {
            cmd_build(game_path, sprite_path, output, floors, min_zoom, max_zoom, data_path, monster_sprites)?;
        }
    }

    Ok(())
}

fn cmd_parse_objects(input: PathBuf, output: PathBuf) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    pb.set_message("Parsing objects.srv...");

    let objects = parse_objects(&input)?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_string_pretty(&objects)?)?;

    pb.finish_with_message(format!("Parsed {} objects → {:?}", objects.len(), output));
    Ok(())
}

fn parse_sector_coords_from_filename(filename: &str) -> Option<(u32, u32, u8)> {
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

fn calculate_global_bounds(
    map_dir: &std::path::Path,
    floors: &[u8],
) -> Result<(u32, u32, u32, u32)> {
    let mut global_min_x = u32::MAX;
    let mut global_max_x = 0;
    let mut global_min_y = u32::MAX;
    let mut global_max_y = 0;

    for entry in fs::read_dir(map_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if let Some((x, y, z)) = parse_sector_coords_from_filename(filename) {
                if floors.contains(&z) {
                    global_min_x = global_min_x.min(x);
                    global_max_x = global_max_x.max(x);
                    global_min_y = global_min_y.min(y);
                    global_max_y = global_max_y.max(y);
                }
            }
        }
    }

    if global_min_x == u32::MAX {
        anyhow::bail!("No map sectors found for specified floors");
    }

    Ok((global_min_x, global_max_x, global_min_y, global_max_y))
}

fn cmd_build(
    game_path: PathBuf,
    sprite_path: PathBuf,
    output: PathBuf,
    floors_str: String,
    min_zoom: u8,
    max_zoom: u8,
    data_path: Option<PathBuf>,
    monster_sprites: Option<PathBuf>,
) -> Result<()> {
    let floors = parse_floor_range(&floors_str)?;

    let cache_dir = PathBuf::from(".demonax-cache");
    fs::create_dir_all(&cache_dir.join("maps"))?;
    fs::create_dir_all(&output)?;

    let objects_path = cache_dir.join("objects.json");

    if !objects_path.exists() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
        pb.set_message("Parsing objects.srv...");
        let objects = parse_objects(game_path.join("dat/objects.srv"))?;
        fs::write(&objects_path, serde_json::to_string(&objects)?)?;
        pb.finish_with_message(format!("Cached {} objects", objects.len()));
    }

    let objects: ObjectDatabase = serde_json::from_str(&fs::read_to_string(&objects_path)?)?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    pb.set_message("Initializing sprite cache...");
    let sprite_cache = SpriteCache::new(&sprite_path)?;
    pb.finish_with_message("Sprite cache initialized");

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    pb.set_message("Preloading sprites...");
    let mut all_sprite_ids: Vec<u32> = objects.keys().copied().collect();

    // Also preload DisguiseTarget sprites
    let disguise_targets: Vec<u32> = objects
        .values()
        .filter_map(|obj| obj.disguise_target)
        .collect();
    all_sprite_ids.extend(disguise_targets);
    all_sprite_ids.sort_unstable();
    all_sprite_ids.dedup();

    sprite_cache.preload_sprites(&all_sprite_ids)?;
    pb.finish_with_message(format!("Loaded {} sprites", sprite_cache.cache_size()));

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    pb.set_message("Calculating map bounds...");

    let map_dir = game_path.join("map");
    let (global_min_sector_x, global_max_sector_x, global_min_sector_y, global_max_sector_y) =
        calculate_global_bounds(&map_dir, &floors)?;

    pb.finish_with_message(format!(
        "Map bounds: sectors ({}-{}, {}-{})",
        global_min_sector_x, global_max_sector_x,
        global_min_sector_y, global_max_sector_y
    ));

    for floor in &floors {
        let map_path = cache_dir.join(format!("maps/floor_{:02}_sprite.json", floor));

        if !map_path.exists() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
            pb.set_message(format!("Parsing floor {}...", floor));
            let map_data = parse_sprite_map(
                &game_path,
                *floor,
                global_min_sector_x,
                global_min_sector_y
            )?;
            fs::write(&map_path, serde_json::to_string(&map_data)?)?;
            pb.finish_with_message(format!("Cached floor {} ({} tiles)", floor, map_data.tiles.len()));
        }

        let map_data: SpriteMapData = serde_json::from_str(&fs::read_to_string(&map_path)?)?;

        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
        pb.set_message(format!("Generating tiles for floor {}...", floor));
        let n_tiles = generate_sprite_tiles(
            &map_data,
            &sprite_cache,
            &objects,
            &output,
            *floor,
            min_zoom,
            max_zoom,
        )?;
        pb.finish_with_message(format!("Floor {}: {} tiles", floor, n_tiles));
    }

    let min_tile_x = global_min_sector_x * 32;
    let max_tile_x = (global_max_sector_x + 1) * 32 - 1;
    let min_tile_y = global_min_sector_y * 32;
    let max_tile_y = (global_max_sector_y + 1) * 32 - 1;

    generate_html(&output, &floors, min_zoom, max_zoom, min_tile_x, max_tile_x, min_tile_y, max_tile_y)?;

    let data_path_clone = data_path.clone();

    if let (Some(data_path), Some(monster_sprites)) = (data_path, monster_sprites) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
        pb.set_message("Parsing monster data...");

        let monster_db_path = data_path.join("game/dat/monster.db");
        let spawns = parse_monster_db(&monster_db_path)?;

        pb.set_message("Copying monster sprites...");
        let monsters_dir = output.join("monsters");
        fs::create_dir_all(&monsters_dir)?;

        // Copy PNG files (named by race ID)
        let mut copied_count = 0;
        for spawn in &spawns {
            let race_id = spawn.race;
            let src = monster_sprites.join(format!("{}.png", race_id));
            let dst = monsters_dir.join(format!("{}.png", race_id));

            if src.exists() {
                fs::copy(&src, &dst)?;
                copied_count += 1;
            } else {
                tracing::warn!("Missing PNG for race ID {}: {:?}", race_id, src);
            }
        }

        pb.set_message("Loading monster names...");
        let monster_names = {
            let mon_dir = data_path.join("game/mon");
            if mon_dir.exists() {
                match parse_monster_names(&mon_dir) {
                    Ok(names) => names,
                    Err(e) => {
                        tracing::warn!("Failed to load monster names: {}", e);
                        Default::default()
                    }
                }
            } else {
                tracing::warn!("Monster directory not found: {:?}", mon_dir);
                Default::default()
            }
        };

        pb.set_message("Generating spawn data...");
        let spawn_json = generate_spawn_json(&spawns, &floors, &monster_names)?;
        fs::write(output.join("spawns.json"), spawn_json)?;

        pb.finish_with_message(format!(
            "Monster spawns: {} spawns, {} sprites copied",
            spawns.len(),
            copied_count
        ));
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    pb.set_message("Parsing quest chests...");

    let quest_names = if let Some(ref data_path) = data_path_clone {
        let quest_csv_path = data_path.join("csv/quest_overview.csv");
        if quest_csv_path.exists() {
            pb.set_message("Loading quest names from CSV...");
            match parse_quest_csv(&quest_csv_path) {
                Ok(names) => names,
                Err(e) => {
                    tracing::warn!("Failed to load quest names: {}", e);
                    Default::default()
                }
            }
        } else {
            Default::default()
        }
    } else {
        Default::default()
    };

    let quest_chests = parse_questchests_from_sectors(&map_dir, &floors, &quest_names)?;

    pb.set_message("Generating quest chest data...");
    let questchests_json = generate_questchests_json(&quest_chests, &floors)?;
    fs::write(output.join("questchests.json"), questchests_json)?;

    pb.finish_with_message(format!("Quest chests: {} found", quest_chests.len()));

    println!("✓ Build complete → {:?}/index.html", output);

    Ok(())
}

fn parse_floor_range(s: &str) -> Result<Vec<u8>> {
    if s.contains('-') {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 2 {
            let start: u8 = parts[0].parse()?;
            let end: u8 = parts[1].parse()?;
            return Ok((start..=end).collect());
        }
    }
    Ok(vec![s.parse()?])
}

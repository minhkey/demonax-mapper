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
    let all_object_ids: Vec<u32> = objects.keys().copied().collect();
    sprite_cache.preload_sprites(&all_object_ids)?;
    pb.finish_with_message(format!("Loaded {} sprites", sprite_cache.cache_size()));

    let mut global_min_sector_x = u32::MAX;
    let mut global_max_sector_x = 0;
    let mut global_min_sector_y = u32::MAX;
    let mut global_max_sector_y = 0;

    for floor in &floors {
        let map_path = cache_dir.join(format!("maps/floor_{:02}_sprite.json", floor));

        if !map_path.exists() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
            pb.set_message(format!("Parsing floor {}...", floor));
            let map_data = parse_sprite_map(&game_path, *floor)?;
            fs::write(&map_path, serde_json::to_string(&map_data)?)?;
            pb.finish_with_message(format!("Cached floor {} ({} tiles)", floor, map_data.tiles.len()));
        }

        let map_data: SpriteMapData = serde_json::from_str(&fs::read_to_string(&map_path)?)?;

        global_min_sector_x = global_min_sector_x.min(map_data.min_sector_x);
        global_max_sector_x = global_max_sector_x.max(map_data.max_sector_x);
        global_min_sector_y = global_min_sector_y.min(map_data.min_sector_y);
        global_max_sector_y = global_max_sector_y.max(map_data.max_sector_y);

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

        pb.set_message("Generating spawn data...");
        let spawn_json = generate_spawn_json(&spawns, &floors)?;
        fs::write(output.join("spawns.json"), spawn_json)?;

        pb.finish_with_message(format!(
            "Monster spawns: {} spawns, {} sprites copied",
            spawns.len(),
            copied_count
        ));
    }

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

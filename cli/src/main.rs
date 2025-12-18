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

    ParseMap {
        #[arg(help = "Path to game directory")]
        game_path: PathBuf,

        #[arg(short, long, help = "Floor number (0-15)")]
        floor: u8,

        #[arg(short, long, default_value = ".demonax-cache/objects.json")]
        objects: PathBuf,

        #[arg(short = 'o', long, help = "Output JSON file")]
        output: Option<PathBuf>,
    },

    GenerateTiles {
        #[arg(help = "Path to map data JSON")]
        map_data: PathBuf,

        #[arg(short, long, default_value = ".demonax-cache/color_map.json")]
        color_map: PathBuf,

        #[arg(short, long, help = "Floor number")]
        floor: u8,

        #[arg(short = 'o', long, default_value = "output")]
        output: PathBuf,

        #[arg(long, default_value = "0")]
        min_zoom: u8,

        #[arg(long, default_value = "5")]
        max_zoom: u8,
    },

    Build {
        #[arg(help = "Path to game directory")]
        game_path: PathBuf,

        #[arg(short, long, default_value = "output")]
        output: PathBuf,

        #[arg(short, long, help = "Floors to generate (e.g. 0-15 or 7)")]
        floors: String,

        #[arg(long, default_value = "0")]
        min_zoom: u8,

        #[arg(long, default_value = "5")]
        max_zoom: u8,
    },

    BuildSprite {
        #[arg(help = "Path to game directory")]
        game_path: PathBuf,

        #[arg(short, long, help = "Path to sprite directory")]
        sprite_path: PathBuf,

        #[arg(short, long, default_value = "output-sprite")]
        output: PathBuf,

        #[arg(short, long, help = "Floors to generate (e.g. 0-15 or 7)")]
        floors: String,

        #[arg(long, default_value = "0")]
        min_zoom: u8,

        #[arg(long, default_value = "5")]
        max_zoom: u8,
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
        Commands::ParseMap {
            game_path,
            floor,
            objects,
            output,
        } => {
            cmd_parse_map(game_path, floor, objects, output)?;
        }
        Commands::GenerateTiles {
            map_data,
            color_map,
            floor,
            output,
            min_zoom,
            max_zoom,
        } => {
            cmd_generate_tiles(map_data, color_map, floor, output, min_zoom, max_zoom)?;
        }
        Commands::Build {
            game_path,
            output,
            floors,
            min_zoom,
            max_zoom,
        } => {
            cmd_build(game_path, output, floors, min_zoom, max_zoom)?;
        }
        Commands::BuildSprite {
            game_path,
            sprite_path,
            output,
            floors,
            min_zoom,
            max_zoom,
        } => {
            cmd_build_sprite(game_path, sprite_path, output, floors, min_zoom, max_zoom)?;
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

fn cmd_parse_map(
    game_path: PathBuf,
    floor: u8,
    objects_path: PathBuf,
    output: Option<PathBuf>,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    pb.set_message("Loading objects...");
    let objects: ObjectDatabase = serde_json::from_str(&fs::read_to_string(&objects_path)?)?;

    pb.set_message(format!("Parsing floor {}...", floor));
    let map_data = parse_map(&game_path, floor, &objects)?;

    let output = output.unwrap_or_else(|| {
        PathBuf::from(format!(".demonax-cache/maps/floor_{:02}.json", floor))
    });

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_string_pretty(&map_data)?)?;

    pb.finish_with_message(format!(
        "Parsed {} tiles → {:?}",
        map_data.tiles.len(),
        output
    ));
    Ok(())
}

fn cmd_generate_tiles(
    map_data_path: PathBuf,
    color_map_path: PathBuf,
    floor: u8,
    output: PathBuf,
    min_zoom: u8,
    max_zoom: u8,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    pb.set_message("Loading map data...");
    let map_data: MapData = serde_json::from_str(&fs::read_to_string(&map_data_path)?)?;

    pb.set_message("Loading color map...");
    let color_map: ColorMap = serde_json::from_str(&fs::read_to_string(&color_map_path)?)?;

    pb.set_message(format!("Generating tiles for floor {}...", floor));
    let n_tiles = generate_tiles(&map_data, &color_map, &output, floor, min_zoom, max_zoom)?;

    pb.finish_with_message(format!("Generated {} tiles → {:?}", n_tiles, output));
    Ok(())
}

fn cmd_build(
    game_path: PathBuf,
    output: PathBuf,
    floors_str: String,
    min_zoom: u8,
    max_zoom: u8,
) -> Result<()> {
    let floors = parse_floor_range(&floors_str)?;

    let cache_dir = PathBuf::from(".demonax-cache");
    fs::create_dir_all(&cache_dir.join("maps"))?;
    fs::create_dir_all(&output)?;

    let objects_path = cache_dir.join("objects.json");
    let color_map_path = cache_dir.join("color_map.json");

    if !objects_path.exists() {
        let pb = ProgressBar::new_spinner();
        pb.set_message("Parsing objects.srv...");
        let objects = parse_objects(game_path.join("dat/objects.srv"))?;
        fs::write(&objects_path, serde_json::to_string(&objects)?)?;
        pb.finish_with_message(format!("Cached {} objects", objects.len()));
    }

    let objects: ObjectDatabase = serde_json::from_str(&fs::read_to_string(&objects_path)?)?;

    if !color_map_path.exists() {
        let color_map = create_color_map(&objects);
        fs::write(&color_map_path, serde_json::to_string(&color_map)?)?;
    }

    let color_map: ColorMap = serde_json::from_str(&fs::read_to_string(&color_map_path)?)?;

    let mut global_min_sector_x = u32::MAX;
    let mut global_max_sector_x = 0;
    let mut global_min_sector_y = u32::MAX;
    let mut global_max_sector_y = 0;

    for floor in &floors {
        let map_path = cache_dir.join(format!("maps/floor_{:02}.json", floor));

        if !map_path.exists() {
            let pb = ProgressBar::new_spinner();
            pb.set_message(format!("Parsing floor {}...", floor));
            let map_data = parse_map(&game_path, *floor, &objects)?;
            fs::write(&map_path, serde_json::to_string(&map_data)?)?;
            pb.finish_with_message(format!("Cached floor {} ({} tiles)", floor, map_data.tiles.len()));
        }

        let map_data: MapData = serde_json::from_str(&fs::read_to_string(&map_path)?)?;

        global_min_sector_x = global_min_sector_x.min(map_data.min_sector_x);
        global_max_sector_x = global_max_sector_x.max(map_data.max_sector_x);
        global_min_sector_y = global_min_sector_y.min(map_data.min_sector_y);
        global_max_sector_y = global_max_sector_y.max(map_data.max_sector_y);

        let pb = ProgressBar::new_spinner();
        pb.set_message(format!("Generating tiles for floor {}...", floor));
        let n_tiles = generate_tiles(&map_data, &color_map, &output, *floor, min_zoom, max_zoom)?;
        pb.finish_with_message(format!("Floor {}: {} tiles", floor, n_tiles));
    }

    let min_tile_x = global_min_sector_x * 32;
    let max_tile_x = (global_max_sector_x + 1) * 32 - 1;
    let min_tile_y = global_min_sector_y * 32;
    let max_tile_y = (global_max_sector_y + 1) * 32 - 1;

    generate_html(&output, &floors, min_zoom, max_zoom, min_tile_x, max_tile_x, min_tile_y, max_tile_y)?;
    println!("✓ Build complete → {:?}/index.html", output);

    Ok(())
}

fn cmd_build_sprite(
    game_path: PathBuf,
    sprite_path: PathBuf,
    output: PathBuf,
    floors_str: String,
    min_zoom: u8,
    max_zoom: u8,
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
            pb.set_message(format!("Parsing floor {} (sprite mode)...", floor));
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
        pb.set_message(format!("Generating sprite tiles for floor {}...", floor));
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
    println!("✓ Sprite-based build complete → {:?}/index.html", output);

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

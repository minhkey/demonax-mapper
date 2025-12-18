# Demonax Mapper

A high-performance map tile generator for Demonax/Tibia game servers. Generates zoomable web maps from game data using actual sprite images.

## Features

- **Sprite-based rendering** - Uses actual game sprites for authentic map visualization
- **Multi-zoom support** - Generates tiles at multiple zoom levels (0-5)
- **Multi-floor support** - Generate maps for any floor (0-15)
- **Fast generation** - Optimized parallel processing with caching
- **Web viewer** - Includes interactive HTML map viewer with Leaflet.js
- **Correct Z-ordering** - Proper isometric rendering with accurate sprite layering

## Prerequisites

- Rust 2024 edition or later
- Game server files:
  - `dat/objects.srv` - Object definitions
  - `map/*.sec` - Sector files
- Sprite images (32x32 or 64x64 PNG files)

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/demonax-mapper`.

## Generating Sprite Images

Sprite images should be extracted from your game's `.spr` and `.dat` files. You can use:
- [OTS Item Images Generator](https://item-images.ots.me/generator/)
- Other sprite extraction tools

Place extracted sprites (PNG format) in a directory, named by object ID (e.g., `1234.png`).

## Usage

### Basic Map Generation

Generate a map for floor 7 with default zoom levels (0-5):

```bash
./target/release/demonax-mapper build \
    /path/to/game \
    --sprite-path /path/to/sprites \
    --floors 7
```

### Multiple Floors

Generate maps for floors 0-15:

```bash
./target/release/demonax-mapper build \
    /path/to/game \
    --sprite-path /path/to/sprites \
    --floors 0-15
```

Or specific floors:

```bash
./target/release/demonax-mapper build \
    /path/to/game \
    --sprite-path /path/to/sprites \
    --floors "0,7,8,15"
```

### Custom Zoom Levels

Generate only specific zoom levels:

```bash
./target/release/demonax-mapper build \
    /path/to/game \
    --sprite-path /path/to/sprites \
    --floors 7 \
    --min-zoom 3 \
    --max-zoom 5
```

### Custom Output Directory

```bash
./target/release/demonax-mapper build \
    /path/to/game \
    --sprite-path /path/to/sprites \
    --floors 7 \
    --output my-map
```

### Verbose Output

Add `-v` flags for more detailed logging:

```bash
# Info level
./target/release/demonax-mapper -v build ...

# Debug level
./target/release/demonax-mapper -vv build ...

# Trace level
./target/release/demonax-mapper -vvv build ...
```

## Commands

### `build`

Main command for generating map tiles.

**Required arguments:**
- `<GAME_PATH>` - Path to game directory (contains `dat/` and `map/`)
- `--sprite-path <PATH>` - Path to directory containing sprite PNG files
- `--floors <RANGE>` - Floor numbers to generate (e.g., `7` or `0-15`)

**Optional arguments:**
- `--output <PATH>` - Output directory (default: `output`)
- `--min-zoom <LEVEL>` - Minimum zoom level (default: `0`)
- `--max-zoom <LEVEL>` - Maximum zoom level (default: `5`)

### `parse-objects`

Parse objects.srv file (rarely needed - happens automatically):

```bash
./target/release/demonax-mapper parse-objects \
    /path/to/game/dat/objects.srv \
    --output .demonax-cache/objects.json
```

## Output Structure

After generation, the output directory contains:

```
output/
├── index.html          # Interactive map viewer
├── 7/                  # Floor 7
│   ├── 0/             # Zoom level 0
│   │   ├── 0/         # Tile column 0
│   │   │   ├── 0.png
│   │   │   ├── 1.png
│   │   │   └── ...
│   │   └── ...
│   ├── 1/             # Zoom level 1
│   └── ...
└── ...
```

## Caching

The mapper caches parsed data in `.demonax-cache/`:

- `objects.json` - Parsed object definitions
- `maps/floor_XX_sprite.json` - Parsed map data per floor

Delete the cache directory to force re-parsing:

```bash
rm -rf .demonax-cache
```

## Performance

Typical performance for a single floor at zoom levels 0-5:
- ~30-35 seconds for ~36,000 tiles
- Utilizes parallel processing for optimal speed
- Memory usage scales with sprite cache size

## Rendering Details

### Sprite Positioning

- Sprites use **anchor point positioning** (bottom-right corner)
- Multi-tile sprites (64x64, 64x32, 32x64) automatically extend from their anchor
- Sprites at sector boundaries correctly render across edges

### Layer Ordering

Objects are rendered in the following layer order:

1. **Ground** - Objects with `is_ground=true` or `Bank` flag (floors, water, swamp)
2. **Clip** - Objects with `Clip` flag (grass overlays, small decorations)
3. **Bottom** - Objects with `Bottom` or `Text` flag (walls, doors, signs)
4. **Normal** - All other objects
5. **Top** - Objects with `Top` flag (open doors, hangings)

### Z-Ordering

Tiles are sorted by `(Y ascending, X ascending)` to ensure correct isometric perspective:
- Objects farther north (lower Y) render first
- Objects farther west (lower X) render first
- Objects closer to the viewer (higher Y, higher X) render on top

## Troubleshooting

### Missing sprites show as magenta squares

The sprite file doesn't exist or couldn't be loaded. Check:
- Sprite files are named by object ID (e.g., `1234.png`)
- Sprite path is correct
- Files are valid PNG format

### Sprites rendering in wrong order

Ensure you're using the latest version with the tile sorting fix.

### Out of memory errors

Try:
- Generating one floor at a time
- Reducing the zoom level range
- Increasing system memory

### Sprites cut off at map edges

This is fixed in the latest version. Ensure negative coordinate handling is enabled.

## Technical Details

### Sprite Format

- **Standard**: 32x32 pixels, RGBA PNG
- **Large sprites**: 64x64, 64x32, or 32x64 pixels (walls, doors, trees, etc.)
- Automatically scaled for each zoom level

### Zoom Levels

- **Level 0**: Largest (1:1 pixel ratio)
- **Level 5**: Smallest (1:32 pixel ratio)
- Each level is 2x smaller than the previous

### Tile Size

- Standard tile size: 256x256 pixels
- Leaflet.js compatible format

## Development

### Project Structure

```
demonax-mapper/
├── cli/                    # Command-line interface
│   └── src/main.rs
├── demonax-mapper-core/   # Core library
│   └── src/
│       ├── objects.rs     # Object parsing
│       ├── sprites.rs     # Sprite caching
│       ├── tiles_sprite.rs # Tile generation
│       └── html.rs        # HTML viewer generation
└── Cargo.toml
```

### Building from Source

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

## License

(Add your license here)

## Contributing

(Add contribution guidelines here)

# Demonax mapper

Map tile generator for Demonax/Tibia-style game servers. Generates zoomable web maps from game data using sprite images.

## Features

- **Sprite-based rendering**: Uses game sprites for map visualization
- **Multi-zoom support**: Generates tiles at multiple zoom levels (0-5)
- **Multi-floor support**: Generate maps for any floor (0-15)
- **Monster spawn visualization**: Displays spawn points from `monster.db` with creature images
- **Parallel generation**: Optimized parallel processing with caching
- **Web viewer**: Includes interactive HTML map viewer with Leaflet.js
- **Map coordinate display**: Shows position hash and corresponding `.sec` file for each location
- **Quest locations**: Clickable button shows quest locations

## Prerequisites

- Rust 2024 edition or later
- Game server files:
  - `objects.srv` for object definitions
  - `map/*.sec` for map sector files
- Sprite images (32x32 or 64x64 PNG files)

Optional for additional features:
- `monster.db` for monster spawn data
- Monster sprite PNG files named by race ID (e.g., `1.png`, `2.png`)
- `quest_overview.csv` for quest names

## Input files structure

### Directory layout example

```
game/                           # Game server directory
├── dat/
│   └── objects.srv            # Object definitions (--objects-path)
└── map/                       # Map directory (--map-path)
    ├── 0-0-7.sec              # Sector files
    ├── 0-1-7.sec
    └── ...

sprites/                        # Sprite directory (--sprite-path)
├── 1.png                      # Object sprite files named by object ID
├── 2.png
├── 1234.png
└── ...

demonax-data/                   # Optional: demonax-data repository
├── game/
│   ├── dat/
│   │   └── monster.db         # Monster spawn database (--monster-db)
│   └── mon/                   # Monster names directory (--monster-names-dir)
│       ├── 1.mon
│       └── ...
└── csv/
    └── quest_overview.csv     # Quest names (--quest-csv)

monster-sprites/                # Optional: Monster sprites (--monster-sprites)
├── 1.png                      # Monster sprite files named by race ID
├── 2.png
├── 3.png
└── ...
```

### Game Data
- `objects.srv`: Object definitions file from game server
- `*.sec` files: Map sector files in `map/` directory

### Sprite Images
- PNG files named by object ID (e.g., `1234.png`)
- Standard size: 32x32 pixels
- Large sprites: 64x64, 64x32, or 32x64 pixels (walls, doors, trees)

### Monster Data (optional)
- `monster.db`: Monster spawn database
- `.mon` files: Monster name definitions
- Monster sprite PNGs: Named by race ID (e.g., `1.png`, `2.png`)

### Quest Data (optional)
- `quest_overview.csv`: CSV file with columns `quest_value,quest_name`
- Header row is skipped
- Used to display quest names for quest chests

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
    --objects-path /path/to/game/dat/objects.srv \
    --map-path /path/to/game/map \
    --sprite-path /path/to/sprites \
    --floors 7
```

### Multiple Floors

Generate maps for floors 0-15:

```bash
./target/release/demonax-mapper build \
    --objects-path /path/to/game/dat/objects.srv \
    --map-path /path/to/game/map \
    --sprite-path /path/to/sprites \
    --floors 0-15
```

### Custom Zoom Levels

Generate only specific zoom levels:

```bash
./target/release/demonax-mapper build \
    --objects-path /path/to/game/dat/objects.srv \
    --map-path /path/to/game/map \
    --sprite-path /path/to/sprites \
    --floors 7 \
    --min-zoom 3 \
    --max-zoom 5
```

### Custom Output Directory

```bash
./target/release/demonax-mapper build \
    --objects-path /path/to/game/dat/objects.srv \
    --map-path /path/to/game/map \
    --sprite-path /path/to/sprites \
    --floors 7 \
    --output my-map
```

### Including Monster Spawns

Generate map with monster spawn points:

```bash
./target/release/demonax-mapper build \
    --objects-path /path/to/game/dat/objects.srv \
    --map-path /path/to/game/map \
    --sprite-path /path/to/sprites \
    --floors 7 \
    --monster-db /path/to/monster.db \
    --monster-names-dir /path/to/mon \
    --monster-sprites /path/to/monster-sprites
```

Monster spawns will be displayed as markers on the map with creature images.

**Note:** Both `--monster-db` and `--monster-sprites` are required for monster spawn visualization.

### Controlling Thread Count

By default, the mapper uses all available CPU cores. You can limit this with `--threads` or `-j`:

```bash
./target/release/demonax-mapper build \
    --objects-path /path/to/game/dat/objects.srv \
    --map-path /path/to/game/map \
    --sprite-path /path/to/sprites \
    --floors 0-15 \
    --threads 4
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
- `--objects-path <FILE>` - Path to objects.srv file
- `--map-path <DIR>` - Path to map directory containing .sec files
- `--sprite-path <DIR>` - Path to directory containing sprite PNG files
- `--floors <RANGE>` - Floor numbers to generate (e.g., `7` or `0-15`)

**Optional arguments:**
- `--output <PATH>` - Output directory (default: `output`)
- `--min-zoom <LEVEL>` - Minimum zoom level (default: `0`)
- `--max-zoom <LEVEL>` - Maximum zoom level (default: `5`)
- `--monster-db <FILE>` - Path to monster.db file
- `--monster-names-dir <DIR>` - Path to directory with .mon files for monster names
- `--monster-sprites <DIR>` - Path to monster sprite directory (PNG files named by race ID)
- `--quest-csv <FILE>` - Path to quest_overview.csv file
- `--threads <N>` / `-j <N>` - Number of worker threads (default: all cores)

**Note:** Both `--monster-db` and `--monster-sprites` must be provided together to enable monster spawn visualization.

### `parse-objects`

Parse objects.srv file (rarely needed - happens automatically):

```bash
./target/release/demonax-mapper parse-objects \
    /path/to/game/dat/objects.srv \
    --output .demonax-cache/objects.json
```

## Full Example

```bash
./target/release/demonax-mapper build \
    --objects-path /path/to/objects.srv \
    --map-path /path/to/map \
    --sprite-path /path/to/sprites \
    --monster-db /path/to/monster.db \
    --monster-names-dir /path/to/mon \
    --monster-sprites /path/to/outfits \
    --quest-csv /path/to/quest_overview.csv \
    --threads 4 \
    --floors 0-15 \
    --output ./output
```

## Testing Locally

After generating the map, you can test it locally using Python's built-in HTTP server:

```bash
cd output
python3 -m http.server 8000
```

Then open your browser to `http://localhost:8000` to view the interactive map.

**Note:** A local web server is required because the map tiles are loaded via HTTP requests. Simply opening `index.html` in a browser won't work due to CORS restrictions.

## Map Viewer Features

The generated interactive map includes:

- **Floor Selection** - Dropdown to switch between different floor levels
- **Coordinate Display** - Shows current mouse position in game coordinates (X, Y, Z)
- **Position Hash** - Displays the position hash for the current location
- **Sector File** - Shows the corresponding `.sec` file name for the current position
- **Copy Coordinates** - Click on the map to copy coordinates and sector information to clipboard
- **Toggle Crosshair** - Button to show/hide a crosshair at the center of the map
- **Show Sector Borders** - Button to toggle visibility of sector boundary lines
- **Monster Spawns** - Clickable markers showing spawn points with creature images (when `--monster-db` and `--monster-sprites` are provided)
- **Zoom Controls** - Navigate between 6 zoom levels with smooth transitions
- **Pan & Zoom** - Click and drag to pan, scroll wheel to zoom

## Output Structure

After generation, the output directory contains:

```
output/
├── index.html          # Interactive map viewer
├── spawns.json         # Monster spawn data (optional, when using --monster-db)
├── monsters/           # Monster sprite images (optional, when using --monster-sprites)
│   ├── 1.png          # PNG files named by race ID
│   ├── 2.png
│   └── ...
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

## Deployment

### Server Requirements

**No PHP or backend server required!** The map is pure static HTML/CSS/JavaScript and works with any static web hosting. However:

- **Web server required**: You cannot open `index.html` directly in a browser (file:// protocol) because the map uses `fetch()` to load JSON data files, which requires HTTP/HTTPS
- **Any static server works**: Python's `http.server`, Nginx, Apache, GitHub Pages, Netlify, Vercel, Cloudflare Pages, etc.
- **CDN dependencies**: The map loads Leaflet.js from unpkg.com CDN, so users need internet access to view the map

### Multiple Floors

When generating multiple floors (e.g., `--floors 0-15`), all floors are included in a **single interactive map**:

- All floor data is generated in subdirectories (e.g., `output/0/`, `output/7/`, `output/15/`)
- The map includes a dropdown menu to switch between floors
- Only one `index.html` file is generated - it handles all floors
- Each floor maintains its own zoom levels and tiles
- The URL hash updates when switching floors (e.g., `#1024,1024,7,3` for floor 7)

**Example multi-floor output structure:**
```
output/
├── index.html              # Single viewer for all floors
├── spawns.json            # All monster spawns across floors
├── questchests.json       # All quest chests across floors
├── monsters/              # Shared monster sprites
├── 0/                     # Floor 0 tiles
│   ├── 0/                 # Zoom level 0
│   ├── 1/                 # Zoom level 1
│   └── ...
├── 7/                     # Floor 7 tiles (ground level)
│   ├── 0/
│   └── ...
├── 8/                     # Floor 8 tiles
│   └── ...
└── 15/                    # Floor 15 tiles
    └── ...
```

### Deploying to a Quarto Website

To deploy the map to a Quarto-based website (like demonax-web):

1. Generate the map with your desired settings:
   ```bash
   ./target/release/demonax-mapper build \
       --objects-path /path/to/game/dat/objects.srv \
       --map-path /path/to/game/map \
       --sprite-path /path/to/sprites \
       --floors 0-15 \
       --monster-db /path/to/monster.db \
       --monster-names-dir /path/to/mon \
       --monster-sprites /path/to/monster-sprites \
       --quest-csv /path/to/quest_overview.csv \
       --output /tmp/map-output
   ```

2. Copy the generated files to the website's resources directory:
   ```bash
   # Copy to the Quarto project's resources directory
   mkdir -p ~/repos/demonax-web/dynamic/map
   cp -r /tmp/map-output/* ~/repos/demonax-web/dynamic/map/
   ```

3. Rebuild the Quarto site:
   ```bash
   cd ~/repos/demonax-web
   quarto render
   ```

4. The map will be accessible at `/dynamic/map/index.html` on your published site.

**Note:** Make sure the `dynamic` directory is listed in your `_quarto.yml` resources section:
```yaml
project:
  resources:
    - "dynamic"
```

#### Embedding the Map in Quarto Pages

You have two options for including the map in your Quarto pages:

**Option 1: Direct link** (simplest)
```markdown
[View Interactive Map](/dynamic/map/index.html)
```

**Option 2: Embedded iframe** (shows map inline)
```markdown
<iframe src="/dynamic/map/index.html"
        width="100%"
        height="600px"
        style="border: 1px solid #ccc;">
</iframe>
```

**Option 3: Full-page iframe** (recommended for best experience)
```markdown
<iframe src="/dynamic/map/index.html"
        width="100%"
        height="100vh"
        style="border: none; position: absolute; top: 0; left: 0;">
</iframe>
```

For a full-page map experience, create a dedicated Quarto page (e.g., `map.qmd`) with minimal styling:

```yaml
---
title: "Interactive Map"
format:
  html:
    page-layout: custom
---

<iframe src="/dynamic/map/index.html"
        width="100%"
        height="100vh"
        style="border: none;">
</iframe>
```

#### Linking to Specific Locations

You can link directly to specific map coordinates using URL hash parameters:

```markdown
[View spawn point at X=1024, Y=1024, Floor 7, Zoom 4](/dynamic/map/index.html#1024,1024,7,4)
```

Format: `#X,Y,Z,ZOOM` where:
- X, Y = world coordinates
- Z = floor number
- ZOOM = zoom level (0-5)

### Deploying to Static Web Hosting

The output can be deployed to any static web host (GitHub Pages, Netlify, Vercel, etc.) by simply uploading the contents of the output directory.

**Example for GitHub Pages:**

```bash
# Generate map
./target/release/demonax-mapper build \
    --objects-path /path/to/game/dat/objects.srv \
    --map-path /path/to/game/map \
    --sprite-path /path/to/sprites \
    --floors 0-15 \
    --output ./gh-pages

# Deploy
cd gh-pages
git init
git add .
git commit -m "Deploy map"
git remote add origin https://github.com/yourusername/your-map-repo.git
git push -u origin main
```

Then enable GitHub Pages in your repository settings, pointing to the `main` branch.

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

You can control the number of threads used with `--threads` / `-j` argument.

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
- Reducing thread count with `--threads`

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

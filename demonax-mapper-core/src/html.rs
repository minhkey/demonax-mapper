use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn generate_html<P: AsRef<Path>>(
    output_path: P,
    floors: &[u8],
    min_zoom: u8,
    max_zoom: u8,
    min_tile_x: u32,
    max_tile_x: u32,
    min_tile_y: u32,
    max_tile_y: u32,
) -> Result<()> {
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Demonax Map</title>
    <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
    <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
    <style>
        body {{
            margin: 0;
            padding: 0;
            font-family: Arial, sans-serif;
        }}
        #map {{
            position: absolute;
            top: 50px;
            bottom: 0;
            width: 100%;
            background-color: #214263;
        }}
        #controls {{
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 50px;
            background: #222;
            color: #fff;
            padding: 10px;
            box-sizing: border-box;
            display: flex;
            align-items: center;
            gap: 15px;
            z-index: 1000;
            font-family: monospace;
        }}
        .control-group {{
            display: flex;
            align-items: center;
            gap: 5px;
        }}
        label {{
            font-weight: bold;
        }}
        select {{
            padding: 5px 10px;
            font-family: monospace;
            background: #444;
            color: white;
            border: 1px solid #666;
            border-radius: 3px;
        }}
        #coords {{
            margin-left: auto;
            font-size: 14px;
        }}
    </style>
</head>
<body>
    <div id="controls">
        <div class="control-group">
            <label for="floor-select">Floor:</label>
            <select id="floor-select">
{floor_options}
            </select>
        </div>
        <div id="coords">
            X: <span id="coord-x">-</span>, Y: <span id="coord-y">-</span>, Z: <span id="coord-z">-</span>
        </div>
    </div>
    <div id="map"></div>

    <script>
        const floors = {floors_json};
        const minZoom = {min_zoom};
        const maxZoom = {max_zoom};
        const minTileX = {min_tile_x};
        const maxTileX = {max_tile_x};
        const minTileY = {min_tile_y};
        const maxTileY = {max_tile_y};

        let currentFloor = {default_floor};
        let tileLayer = null;

        const CustomCRS = L.extend({{}}, L.CRS.Simple, {{
            transformation: new L.Transformation(1, 0, -1, 1536)
        }});

        const map = L.map('map', {{
            crs: CustomCRS,
            minZoom: minZoom,
            maxZoom: maxZoom,
            attributionControl: false
        }});

        function loadFloor(floor) {{
            if (tileLayer) {{
                map.removeLayer(tileLayer);
            }}

            tileLayer = L.tileLayer(floor + '/{{z}}/{{x}}/{{y}}.png', {{
                minZoom: minZoom,
                maxZoom: maxZoom,
                noWrap: true,
                bounds: [[0, 0], [1536, 1536]]
            }});

            tileLayer.addTo(map);
            currentFloor = floor;
        }}

        map.setView([768, 768], 0);
        loadFloor(currentFloor);

        map.on('mousemove', function(e) {{
            const latLng = e.latlng;
            const tileX = Math.floor(latLng.lng);
            const tileY = Math.floor(latLng.lat);

            const worldX = minTileX + tileX;
            const worldY = maxTileY - tileY;

            document.getElementById('coord-x').textContent = worldX;
            document.getElementById('coord-y').textContent = worldY;
            document.getElementById('coord-z').textContent = currentFloor;
        }});

        document.getElementById('floor-select').addEventListener('change', function(e) {{
            loadFloor(parseInt(e.target.value));
        }});
    </script>
</body>
</html>"#,
        floor_options = generate_floor_options(floors),
        floors_json = format!("{:?}", floors),
        min_zoom = min_zoom,
        max_zoom = max_zoom,
        min_tile_x = min_tile_x,
        max_tile_x = max_tile_x,
        min_tile_y = min_tile_y,
        max_tile_y = max_tile_y,
        default_floor = floors.first().copied().unwrap_or(7)
    );

    let html_path = output_path.as_ref().join("index.html");
    fs::write(html_path, html)?;

    Ok(())
}

fn generate_floor_options(floors: &[u8]) -> String {
    floors
        .iter()
        .map(|&f| {
            let label = match f {
                7 => format!("Ground ({})", f),
                f if f < 7 => format!("Sky {} ({})", 7 - f, f),
                f => format!("Underground {} ({})", f - 7, f),
            };
            format!(r#"            <option value="{}">{}</option>"#, f, label)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

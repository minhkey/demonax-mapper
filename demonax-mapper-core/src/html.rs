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
        .leaflet-marker-icon.spawn-marker {{
            width: 32px !important;
            height: 32px !important;
            margin: 0 !important;
            padding: 0 !important;
            display: flex !important;
            align-items: center !important;
            justify-content: center !important;
            position: absolute !important;
            border: none !important;
            background: none !important;
        }}
        .spawn-marker img {{
            max-width: 32px;
            max-height: 32px;
            width: auto;
            height: auto;
            image-rendering: pixelated;
        }}
        .spawn-amount {{
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            color: white;
            text-shadow: -1px -1px 0 #000, 1px -1px 0 #000, -1px 1px 0 #000, 1px 1px 0 #000;
            font-weight: bold;
            font-size: 14px;
            pointer-events: none;
        }}
        input[type="checkbox"] {{
            cursor: pointer;
        }}
        .control-group input[type="checkbox"] {{
            margin-right: 5px;
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
        <div class="control-group">
            <label>
                <input type="checkbox" id="spawn-toggle" />
                Show Spawns
            </label>
        </div>
        <div class="control-group">
            <label>
                <input type="checkbox" id="questchest-toggle" />
                Show Questboxes
            </label>
        </div>
        <div id="coords">
            X: <span id="coord-x">-</span>, Y: <span id="coord-y">-</span>, Z: <span id="coord-z">-</span> | <span id="sector-file">-</span>
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
            transformation: new L.Transformation(1, 0, 1, 0)
        }});

        const map = L.map('map', {{
            crs: CustomCRS,
            minZoom: minZoom,
            maxZoom: maxZoom,
            attributionControl: false
        }});

        function parseHash() {{
            const hash = window.location.hash.substring(1);
            if (!hash) return null;

            const parts = hash.split(',');
            if (parts.length !== 4) return null;

            const [x, y, z, zoom] = parts.map(p => parseInt(p, 10));

            if (isNaN(x) || isNaN(y) || isNaN(z) || isNaN(zoom)) return null;
            if (!floors.includes(z)) return null;
            if (zoom < minZoom || zoom > maxZoom) return null;

            return {{ x, y, z, zoom }};
        }}

        function worldToTile(worldX, worldY) {{
            const tileX = worldX - minTileX;
            const tileY = worldY - minTileY;
            return {{ tileX, tileY }};
        }}

        function updateHash() {{
            const center = map.getCenter();
            const zoom = map.getZoom();

            const tileX = Math.floor(center.lng);
            const tileY = Math.floor(center.lat);

            const worldX = minTileX + tileX;
            const worldY = minTileY + tileY;

            const hash = `#${{worldX}},${{worldY}},${{currentFloor}},${{zoom}}`;
            history.replaceState(null, '', hash);
        }}

        function loadFloor(floor) {{
            if (tileLayer) {{
                map.removeLayer(tileLayer);
            }}

            tileLayer = L.tileLayer(floor + '/{{z}}/{{x}}/{{y}}.png', {{
                minZoom: minZoom,
                maxZoom: maxZoom,
                noWrap: true,
                bounds: [[0, 0], [{max_tile_y} - {min_tile_y}, {max_tile_x} - {min_tile_x}]]
            }});

            tileLayer.addTo(map);
            currentFloor = floor;
        }}

        const hashParams = parseHash();

        if (hashParams) {{
            currentFloor = hashParams.z;
            loadFloor(currentFloor);

            document.getElementById('floor-select').value = currentFloor;

            const {{ tileX, tileY }} = worldToTile(hashParams.x, hashParams.y);
            map.setView([tileY, tileX], hashParams.zoom);
        }} else {{
            map.setView([({max_tile_y} - {min_tile_y}) / 2, ({max_tile_x} - {min_tile_x}) / 2], 0);
            loadFloor(currentFloor);
        }}

        map.on('mousemove', function(e) {{
            const latLng = e.latlng;
            const tileX = Math.floor(latLng.lng);
            const tileY = Math.floor(latLng.lat);

            const worldX = minTileX + tileX;
            const worldY = minTileY + tileY;

            const sectorX = Math.floor(worldX / 32);
            const sectorY = Math.floor(worldY / 32);
            const sectorFile = `${{sectorX.toString().padStart(4, '0')}}-${{sectorY.toString().padStart(4, '0')}}-${{currentFloor.toString().padStart(2, '0')}}.sec`;

            document.getElementById('coord-x').textContent = worldX;
            document.getElementById('coord-y').textContent = worldY;
            document.getElementById('coord-z').textContent = currentFloor;
            document.getElementById('sector-file').textContent = sectorFile;
        }});

        let updateHashTimeout;
        map.on('moveend', function() {{
            clearTimeout(updateHashTimeout);
            updateHashTimeout = setTimeout(updateHash, 100);
        }});

        map.on('zoomend', updateHash);

        document.getElementById('floor-select').addEventListener('change', function(e) {{
            loadFloor(parseInt(e.target.value));
            updateHash();
        }});

        window.addEventListener('hashchange', function() {{
            const hashParams = parseHash();
            if (hashParams) {{
                if (hashParams.z !== currentFloor) {{
                    loadFloor(hashParams.z);
                    document.getElementById('floor-select').value = hashParams.z;
                }}

                const {{ tileX, tileY }} = worldToTile(hashParams.x, hashParams.y);
                map.setView([tileY, tileX], hashParams.zoom);
            }}
        }});

        // Monster spawn overlay
        let spawnData = null;
        let spawnMarkers = [];

        fetch('spawns.json')
            .then(response => {{
                if (!response.ok) {{
                    throw new Error('Spawn data not found');
                }}
                return response.json();
            }})
            .then(data => {{
                spawnData = data;
                updateSpawnLayer();
            }})
            .catch(err => {{
                console.warn('Monster spawns unavailable:', err);
                const toggle = document.getElementById('spawn-toggle');
                if (toggle) {{
                    toggle.disabled = true;
                    toggle.parentElement.title = 'Monster spawn data not available';
                }}
            }});

        // Quest chest overlay
        let questChestData = null;
        let questChestMarkers = [];

        fetch('questchests.json')
            .then(response => {{
                if (!response.ok) {{
                    throw new Error('Quest chest data not found');
                }}
                return response.json();
            }})
            .then(data => {{
                questChestData = data;
                updateQuestChestLayer();
            }})
            .catch(err => {{
                console.warn('Quest chests unavailable:', err);
                const toggle = document.getElementById('questchest-toggle');
                if (toggle) {{
                    toggle.disabled = true;
                    toggle.parentElement.title = 'Quest chest data not available';
                }}
            }});

        function worldToLatLng(worldX, worldY) {{
            const tileX = worldX - minTileX;
            const tileY = worldY - minTileY;
            return [tileY, tileX];
        }}

        function updateSpawnLayer() {{
            spawnMarkers.forEach(marker => map.removeLayer(marker));
            spawnMarkers = [];

            const toggle = document.getElementById('spawn-toggle');
            const showSpawns = toggle && toggle.checked;
            const currentZoom = map.getZoom();

            if (!showSpawns || !spawnData || currentZoom < 3) {{
                return;
            }}

            const floorSpawns = spawnData.spawns_by_floor[currentFloor] || [];
            const bounds = map.getBounds();

            const visibleSpawns = floorSpawns.filter(spawn => {{
                const [lat, lng] = worldToLatLng(spawn.x, spawn.y);
                return bounds.contains([lat, lng]);
            }});

            visibleSpawns.forEach(spawn => {{
                const [lat, lng] = worldToLatLng(spawn.x, spawn.y);

                const icon = L.divIcon({{
                    className: 'spawn-marker',
                    html: `
                        <img src="monsters/${{spawn.race}}.png" alt="Race ${{spawn.race}}" onerror="this.style.display='none'" />
                        <div class="spawn-amount">${{spawn.amount}}</div>
                    `,
                    iconSize: [32, 32],
                    iconAnchor: [16, 16],
                    popupAnchor: [0, -16]
                }});

                const marker = L.marker([lat, lng], {{ icon: icon }})
                    .bindPopup(`
                        <b>Race ID: ${{spawn.race}}</b><br/>
                        Spawn Amount: ${{spawn.amount}}<br/>
                        Position: ${{spawn.x}}, ${{spawn.y}}
                    `);

                marker.addTo(map);
                spawnMarkers.push(marker);
            }});
        }}

        function updateQuestChestLayer() {{
            questChestMarkers.forEach(marker => map.removeLayer(marker));
            questChestMarkers = [];

            const toggle = document.getElementById('questchest-toggle');
            const showQuestChests = toggle && toggle.checked;
            const currentZoom = map.getZoom();

            if (!showQuestChests || !questChestData || currentZoom < 3) {{
                return;
            }}

            const floorChests = questChestData.questchests_by_floor[currentFloor] || [];
            const bounds = map.getBounds();

            const visibleChests = floorChests.filter(chest => {{
                const [lat, lng] = worldToLatLng(chest.x, chest.y);
                return bounds.contains([lat, lng]);
            }});

            visibleChests.forEach(chest => {{
                // Center the marker on the tile by adding 0.5 offset
                const [lat, lng] = worldToLatLng(chest.x + 0.5, chest.y + 0.5);

                const marker = L.circleMarker([lat, lng], {{
                    radius: 10,
                    fillColor: '#FFD700',
                    color: '#FFD700',
                    weight: 3,
                    opacity: 0.9,
                    fillOpacity: 0.7
                }})
                .bindPopup(`
                    <b>Quest Chest ${{chest.quest_number}}</b><br/>
                    ${{chest.quest_name ? chest.quest_name : 'Unknown Quest'}}
                `);

                marker.addTo(map);
                questChestMarkers.push(marker);
            }});
        }}

        const spawnToggle = document.getElementById('spawn-toggle');
        if (spawnToggle) {{
            spawnToggle.addEventListener('change', updateSpawnLayer);
        }}

        const questChestToggle = document.getElementById('questchest-toggle');
        if (questChestToggle) {{
            questChestToggle.addEventListener('change', updateQuestChestLayer);
        }}

        map.on('moveend', function() {{
            updateSpawnLayer();
            updateQuestChestLayer();
        }});

        map.on('zoomend', function() {{
            updateSpawnLayer();
            updateQuestChestLayer();
        }});

        const originalLoadFloor = loadFloor;
        loadFloor = function(floor) {{
            originalLoadFloor(floor);
            updateSpawnLayer();
            updateQuestChestLayer();
        }};
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

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
            background-color: #000000;
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
        .leaflet-marker-icon.npc-marker {{
            width: 32px !important;
            height: 32px !important;
            display: flex !important;
            align-items: center !important;
            justify-content: center !important;
        }}
        .npc-marker img {{
            max-width: 32px;
            max-height: 32px;
            image-rendering: pixelated;
        }}
        input[type="checkbox"] {{
            cursor: pointer;
        }}
        .control-group input[type="checkbox"] {{
            margin-right: 5px;
        }}
        #crosshair {{
            position: absolute;
            top: calc(50% + 25px);
            left: 50%;
            transform: translate(-50%, -50%);
            pointer-events: none;
            z-index: 1000;
            display: none;
        }}
        #crosshair.visible {{
            display: block;
        }}
        #crosshair line {{
            stroke: #00ff00;
            stroke-width: 2;
            stroke-linecap: round;
        }}
        #copy-toast {{
            position: fixed;
            bottom: 20px;
            left: 50%;
            transform: translateX(-50%);
            background: rgba(0, 0, 0, 0.8);
            color: #fff;
            padding: 12px 24px;
            border-radius: 4px;
            font-family: monospace;
            font-size: 14px;
            z-index: 10000;
            opacity: 0;
            transition: opacity 0.3s;
            pointer-events: none;
        }}
        #copy-toast.show {{
            opacity: 1;
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
                Show spawns
            </label>
        </div>
        <div class="control-group">
            <label>
                <input type="checkbox" id="questchest-toggle" />
                Show quest locations
            </label>
        </div>
        <div class="control-group">
            <label>
                <input type="checkbox" id="npc-toggle" />
                Show NPCs
            </label>
        </div>
        <div class="control-group">
            <label>
                <input type="checkbox" id="crosshair-toggle" />
                Show crosshair
            </label>
        </div>
        <div class="control-group">
            <label>
                <input type="checkbox" id="sector-grid-toggle" />
                Show sector borders
            </label>
        </div>
        <div id="coords">
            X: <span id="coord-x">-</span>, Y: <span id="coord-y">-</span>, Z: <span id="coord-z">-</span> | <span id="sector-file">-</span>
        </div>
    </div>
    <div id="map"></div>
    <svg id="crosshair" width="40" height="40" viewBox="0 0 40 40">
        <line x1="20" y1="5" x2="20" y2="35" />
        <line x1="5" y1="20" x2="35" y2="20" />
    </svg>
    <div id="copy-toast"></div>

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

        function toTitleCase(str) {{
            return str.split(' ').map(word =>
                word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
            ).join(' ');
        }}

        function parseHash() {{
            const hash = window.location.hash.substring(1);
            if (!hash) return null;

            // Split hash and query parameters
            const [coords, queryString] = hash.split('?');
            const parts = coords.split(',');

            if (parts.length !== 4) return null;

            const [x, y, z, zoom] = parts.map(p => parseInt(p, 10));

            if (isNaN(x) || isNaN(y) || isNaN(z) || isNaN(zoom)) return null;
            if (!floors.includes(z)) return null;
            if (zoom < minZoom || zoom > maxZoom) return null;

            // Parse query parameters for toggle states
            const toggles = {{}};
            if (queryString) {{
                queryString.split('&').forEach(param => {{
                    const [key, value] = param.split('=');
                    toggles[key] = value === '1';
                }});
            }}

            return {{ x, y, z, zoom, toggles }};
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

            // Collect toggle states
            const toggleStates = [];
            const spawnToggle = document.getElementById('spawn-toggle');
            const npcToggle = document.getElementById('npc-toggle');
            const questToggle = document.getElementById('questchest-toggle');
            const crosshairToggle = document.getElementById('crosshair-toggle');
            const gridToggle = document.getElementById('sector-grid-toggle');

            if (spawnToggle && spawnToggle.checked) toggleStates.push('spawns=1');
            if (npcToggle && npcToggle.checked) toggleStates.push('npcs=1');
            if (questToggle && questToggle.checked) toggleStates.push('quests=1');
            if (crosshairToggle && crosshairToggle.checked) toggleStates.push('crosshair=1');
            if (gridToggle && gridToggle.checked) toggleStates.push('grid=1');

            const queryString = toggleStates.length > 0 ? '?' + toggleStates.join('&') : '';
            const hash = `#${{worldX}},${{worldY}},${{currentFloor}},${{zoom}}${{queryString}}`;
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

        // Apply saved toggle states from URL
        if (hashParams && hashParams.toggles) {{
            const {{ toggles }} = hashParams;

            const spawnToggle = document.getElementById('spawn-toggle');
            if (spawnToggle && toggles.spawns !== undefined) {{
                spawnToggle.checked = toggles.spawns;
            }}

            const npcToggle = document.getElementById('npc-toggle');
            if (npcToggle && toggles.npcs !== undefined) {{
                npcToggle.checked = toggles.npcs;
            }}

            const questToggle = document.getElementById('questchest-toggle');
            if (questToggle && toggles.quests !== undefined) {{
                questToggle.checked = toggles.quests;
            }}

            const crosshairToggle = document.getElementById('crosshair-toggle');
            const crosshair = document.getElementById('crosshair');
            if (crosshairToggle && crosshair && toggles.crosshair !== undefined) {{
                crosshairToggle.checked = toggles.crosshair;
                if (toggles.crosshair) {{
                    crosshair.classList.add('visible');
                }}
            }}

            const gridToggle = document.getElementById('sector-grid-toggle');
            if (gridToggle && toggles.grid !== undefined) {{
                gridToggle.checked = toggles.grid;
            }}
        }}

        let lastWorldX = 0;
        let lastWorldY = 0;
        let lastSectorFile = '';

        map.on('mousemove', function(e) {{
            const latLng = e.latlng;
            const tileX = Math.floor(latLng.lng);
            const tileY = Math.floor(latLng.lat);

            const worldX = minTileX + tileX;
            const worldY = minTileY + tileY;

            lastWorldX = worldX;
            lastWorldY = worldY;

            const sectorX = Math.floor(worldX / 32);
            const sectorY = Math.floor(worldY / 32);
            const sectorFile = `${{sectorX.toString().padStart(4, '0')}}-${{sectorY.toString().padStart(4, '0')}}-${{currentFloor.toString().padStart(2, '0')}}.sec`;

            lastSectorFile = sectorFile;

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

        // Sector grid overlay
        let sectorGridLines = [];

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

        // NPC overlay
        let npcData = null;
        let npcMarkers = [];

        fetch('npcs.json')
            .then(response => {{
                if (!response.ok) {{
                    throw new Error('NPC data not found');
                }}
                return response.json();
            }})
            .then(data => {{
                npcData = data;
                updateNpcLayer();
            }})
            .catch(err => {{
                console.warn('NPC data unavailable:', err);
                const toggle = document.getElementById('npc-toggle');
                if (toggle) {{
                    toggle.disabled = true;
                    toggle.parentElement.title = 'NPC data not available';
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
                        <b>${{spawn.name ? toTitleCase(spawn.name) : 'Race ID: ' + spawn.race}}</b><br/>
                        Spawn amount: ${{spawn.amount}}<br/>
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
                    <b>${{chest.quest_name ? chest.quest_name : 'Unknown quest'}}</b><br/>
                    Quest number: ${{chest.quest_number}}
                `);

                marker.addTo(map);
                questChestMarkers.push(marker);
            }});
        }}

        function updateNpcLayer() {{
            npcMarkers.forEach(marker => map.removeLayer(marker));
            npcMarkers = [];

            const toggle = document.getElementById('npc-toggle');
            const showNpcs = toggle && toggle.checked;
            const currentZoom = map.getZoom();

            if (!showNpcs || !npcData || currentZoom < 3) {{
                return;
            }}

            const floorNpcs = npcData.npcs_by_floor[currentFloor] || [];
            const bounds = map.getBounds();

            const visibleNpcs = floorNpcs.filter(npc => {{
                const [lat, lng] = worldToLatLng(npc.x, npc.y);
                return bounds.contains([lat, lng]);
            }});

            visibleNpcs.forEach(npc => {{
                const [lat, lng] = worldToLatLng(npc.x, npc.y);

                const icon = L.divIcon({{
                    className: 'npc-marker',
                    html: `<img src="npcs/${{npc.file_name}}.png" alt="${{npc.npc_name}}" onerror="this.style.display='none'" />`,
                    iconSize: [32, 32],
                    iconAnchor: [16, 16],
                    popupAnchor: [0, -16]
                }});

                const marker = L.marker([lat, lng], {{ icon: icon }})
                    .bindPopup(`<b>${{npc.npc_name}}</b><br/>Position: ${{npc.x}}, ${{npc.y}}`);

                marker.addTo(map);
                npcMarkers.push(marker);
            }});
        }}

        function updateSectorGridLayer() {{
            sectorGridLines.forEach(line => map.removeLayer(line));
            sectorGridLines = [];

            const toggle = document.getElementById('sector-grid-toggle');
            const showGrid = toggle && toggle.checked;
            const currentZoom = map.getZoom();

            if (!showGrid || currentZoom < 3) {{
                return;
            }}

            const bounds = map.getBounds();
            const minTileX_view = Math.floor(bounds.getSouthWest().lng);
            const maxTileX_view = Math.ceil(bounds.getNorthEast().lng);
            const minTileY_view = Math.floor(bounds.getSouthWest().lat);
            const maxTileY_view = Math.ceil(bounds.getNorthEast().lat);

            const minWorldX_view = minTileX + minTileX_view;
            const maxWorldX_view = minTileX + maxTileX_view;
            const minWorldY_view = minTileY + minTileY_view;
            const maxWorldY_view = minTileY + maxTileY_view;

            const minSectorX = Math.floor(minWorldX_view / 32);
            const maxSectorX = Math.ceil(maxWorldX_view / 32);
            const minSectorY = Math.floor(minWorldY_view / 32);
            const maxSectorY = Math.ceil(maxWorldY_view / 32);

            const mapMinWorldX = minTileX;
            const mapMaxWorldX = maxTileX;
            const mapMinWorldY = minTileY;
            const mapMaxWorldY = maxTileY;

            for (let sectorX = minSectorX; sectorX <= maxSectorX; sectorX++) {{
                const worldX = sectorX * 32;

                if (worldX < mapMinWorldX || worldX > mapMaxWorldX) {{
                    continue;
                }}

                const [latStart, lng] = worldToLatLng(worldX, Math.max(minWorldY_view, mapMinWorldY));
                const [latEnd, _] = worldToLatLng(worldX, Math.min(maxWorldY_view, mapMaxWorldY));

                const line = L.polyline(
                    [[latStart, lng], [latEnd, lng]],
                    {{
                        color: '#00FFFF',
                        weight: 1,
                        opacity: 0.3,
                        interactive: false
                    }}
                );

                line.addTo(map);
                sectorGridLines.push(line);
            }}

            for (let sectorY = minSectorY; sectorY <= maxSectorY; sectorY++) {{
                const worldY = sectorY * 32;

                if (worldY < mapMinWorldY || worldY > mapMaxWorldY) {{
                    continue;
                }}

                const [lat, lngStart] = worldToLatLng(Math.max(minWorldX_view, mapMinWorldX), worldY);
                const [_, lngEnd] = worldToLatLng(Math.min(maxWorldX_view, mapMaxWorldX), worldY);

                const line = L.polyline(
                    [[lat, lngStart], [lat, lngEnd]],
                    {{
                        color: '#00FFFF',
                        weight: 1,
                        opacity: 0.3,
                        interactive: false
                    }}
                );

                line.addTo(map);
                sectorGridLines.push(line);
            }}
        }}

        function showToast(message) {{
            const toast = document.getElementById('copy-toast');
            if (toast) {{
                toast.textContent = message;
                toast.classList.add('show');
                setTimeout(() => {{
                    toast.classList.remove('show');
                }}, 2000);
            }}
        }}

        async function copyToClipboard(text, label) {{
            try {{
                await navigator.clipboard.writeText(text);
                showToast(`Copied: ${{label}}`);
            }} catch (err) {{
                console.error('Failed to copy:', err);
                showToast('Copy failed - clipboard not available');
            }}
        }}

        const spawnToggle = document.getElementById('spawn-toggle');
        if (spawnToggle) {{
            spawnToggle.addEventListener('change', function() {{
                updateSpawnLayer();
                updateHash();
            }});
        }}

        const questChestToggle = document.getElementById('questchest-toggle');
        if (questChestToggle) {{
            questChestToggle.addEventListener('change', function() {{
                updateQuestChestLayer();
                updateHash();
            }});
        }}

        const npcToggle = document.getElementById('npc-toggle');
        if (npcToggle) {{
            npcToggle.addEventListener('change', function() {{
                updateNpcLayer();
                updateHash();
            }});
        }}

        const crosshairToggle = document.getElementById('crosshair-toggle');
        const crosshair = document.getElementById('crosshair');
        if (crosshairToggle && crosshair) {{
            crosshairToggle.addEventListener('change', function() {{
                if (this.checked) {{
                    crosshair.classList.add('visible');
                }} else {{
                    crosshair.classList.remove('visible');
                }}
                updateHash();
            }});
        }}

        const sectorGridToggle = document.getElementById('sector-grid-toggle');
        if (sectorGridToggle) {{
            sectorGridToggle.addEventListener('change', function() {{
                updateSectorGridLayer();
                updateHash();
            }});
        }}

        map.on('click', function(e) {{
            if (e.originalEvent.ctrlKey || e.originalEvent.metaKey) {{
                const coords = `${{lastWorldX}},${{lastWorldY}},${{currentFloor}}`;
                copyToClipboard(coords, coords);
                L.DomEvent.stopPropagation(e.originalEvent);
            }}
        }});

        map.getContainer().addEventListener('mousedown', function(e) {{
            if (e.button === 1) {{
                copyToClipboard(lastSectorFile, lastSectorFile);
                e.preventDefault();
            }}
        }});

        map.on('moveend', function() {{
            updateSpawnLayer();
            updateQuestChestLayer();
            updateNpcLayer();
            updateSectorGridLayer();
        }});

        map.on('zoomend', function() {{
            updateSpawnLayer();
            updateQuestChestLayer();
            updateNpcLayer();
            updateSectorGridLayer();
        }});

        const originalLoadFloor = loadFloor;
        loadFloor = function(floor) {{
            originalLoadFloor(floor);
            updateSpawnLayer();
            updateQuestChestLayer();
            updateNpcLayer();
            updateSectorGridLayer();
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

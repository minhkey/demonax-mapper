#!/bin/sh

rm -rf .demonax-cache
rm -rf output && mkdir output

./target/release/demonax-mapper build \
    /home/cmd/repos/demonax-data/game \
    --data-path /home/cmd/repos/demonax-data \
    --sprite-path /home/cmd/repos/demonax-data/items \
    --monster-sprites /home/cmd/repos/demonax-data/outfits \
    --floors 0-15 \
    --min-zoom 0 \
    --max-zoom 5 \
    --output /home/cmd/repos/demonax-mapper/output

# Kill anything using port 8000 (if any)
PIDS=$(lsof -ti tcp:8000)
if [ -n "$PIDS" ]; then
    echo "Killing existing server(s) on port 8000: $PIDS"
    kill $PIDS
    sleep 1
fi

cd output

# Start new server
python3 -m http.server 8000

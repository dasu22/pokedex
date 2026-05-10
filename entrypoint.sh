#!/bin/sh
# On first boot the Fly.io volume is empty — seed it with the bundled data.
DATA_FILE="${DATA_FILE:-/data/pokemon_data.json}"

if [ ! -f "$DATA_FILE" ]; then
  echo "Seeding data volume from bundled default..."
  cp /app/pokemon_data_default.json "$DATA_FILE"
fi

exec ./pokedex

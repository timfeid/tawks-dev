#!/bin/bash

for file in "."/*.png; do
  base=$(basename "$file" .png)
  convert "$file" -trim +repage \
    -resize 192x192 -gravity center \
    -background none -extent 192x192 \
    -define png:color-type=6 \
    "$(dirname "$file")/normalized/$base.png"
  echo "Normalized: $base"
done

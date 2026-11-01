# Crawling zombie

`zombie.glb` uses Quaternius's Zombie_Male from **Ultimate Animated Character Pack (November 2019)**, released under **CC0 1.0**. Original author release: https://opengameart.org/content/animated-characters-pack . The source archive is the same original pack used by the purchased character catalog. No downloaded third-party textures or additional network requests are needed.

The source skeleton's Idle pose is the basis for an authored prone `Crawl` animation: opposing hand/knee reaches, bent elbows and a raised head. The original IK is baked into ordinary joint animation. Meshes, weights, original skin and clothing are preserved. `export-source.py` documents conversion from the original `Zombie_Male.blend` using Blender. Run `blender -b --python export-source.py -- Zombie_Male.blend zombie.glb`.

This is a board hazard, not a player costume. One rig and animation mixer persist across authoritative snapshots. Approximate world size and crawl speed are intentionally small and slow; gameplay movement and contact are server controlled.

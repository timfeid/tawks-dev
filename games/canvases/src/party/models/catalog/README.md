# Tawkathon character catalog

Six distinct Quaternius character models, each sold in the game for 10 tickets.
Their geometry, materials, skeletons and authored animations are distributed
under **CC0 1.0 Universal**. These assets keep their own appearance; fitted
clothing for the round Tawkathon character is not attached to these models.

## Provenance

| File | Original authored source | Primary license/download page |
| --- | --- | --- |
| `robot.glb` | `Animated Robot - Oct 2018/Blend/Robot.blend` | [Animated LowPoly Robot, posted by quaternius](https://opengameart.org/node/89467) |
| `ninja.glb` | `Blends/Ninja_Male.blend` | [Animated Characters Pack, posted by quaternius](https://opengameart.org/content/animated-characters-pack) |
| `cowboy.glb` | `Blends/Cowboy_Male.blend` | same author pack |
| `goblin.glb` | `Blends/Goblin_Male.blend` | same author pack |
| `knight.glb` | `Blends/Knight_Male.blend` | same author pack |
| `pirate.glb` | `Blends/Pirate_Male.blend` | same author pack |

The five-character archive is the November 2019 Ultimate Animated Character
Pack, released on the author's OpenGameArt page on June 18, 2021, with its
explicit CC0 designation. The original archive URL is:

<https://opengameart.org/sites/default/files/ultimate_animated_character_pack_by_quaternius.zip>

SHA-256: `0295062cbc72da52ee804f39ac38694901a40458011f9b2638fc7ef13ed509da`.

The Robot archive SHA-256 is
`5128d2cf906835895b931f6c1e764296ba2e01d3a30d809035fc6c1ee8a1c1ee`.
Its original author license notice is preserved in `LICENSE-Quaternius.txt`.
These archived CC0 releases were used, rather than newer packs published under
the Quaternius Asset License. Author/source information remains in the
canonical `../catalog.json` for each character. Verified October 28, 2026.

## Conversion

`export-source.py INPUT.blend OUTPUT.glb` reproduces the Blender 4.2 export
using the `bpy` Python module. For example:

```sh
python export-source.py Ninja_Male.blend ninja.glb
```

The conversion retains the original authored rig and animation. No UniRig,
Mixamo, motion emulation or externally hosted assets are involved. The Robot's
rigid bone-parented mesh panels are converted to conventional single-bone
skinned panels while retaining their rest transforms. Standard PBR materials
use the original authored diffuse colors. Unused stale image references and
non-character scene objects are omitted. Exported GLBs are self-contained,
with no external textures, compression or custom extensions.

Ninja, Cowboy, Goblin, Knight and Pirate retain Idle, Run, Victory, Defeat and
Death. Robot retains Idle, Run and its original Dance animation as Victory;
additional Robot clips were omitted to stay within the strict 512-accessor
import limit. Missing sad/fall mappings fall back to Idle using the existing
model runtime. All six have real Idle and Run animations and weighted rigs.

## Verification

Every file passed the same `inspectGlb` used for uploaded models. The canonical
manifest contains each resulting `ModelReport`, including exact byte counts,
vertices/triangles, clip mapping and `playable: true`. `validation.json` records
output hashes, node/bone/accessor limits and extensions. All six are under
470 KB individually, under 7,100 vertices, under 3,300 triangles, at most 58
nodes, at most 43 bones, no textures, and below 512 accessors/buffer views.

The actual browser `parseCustomModel` loader also loaded all six models and
advanced both authored Idle and Run clips. Bone transforms changed for each
clip, finite normalized geometry remained visible, and there were no browser
JavaScript errors. The isolated conversion/browser reproducibility files and
preview screenshot are kept outside the repository in
`/data/tawkathon-model-catalog/`.

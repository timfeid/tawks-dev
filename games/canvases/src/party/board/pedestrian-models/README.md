# Town pedestrians

Three distinct civilian meshes, rigs and animations by **Quaternius**, from the
November 2019 Ultimate Animated Character Pack, posted by the author with the
CC0 dedication on June 18, 2021:

https://opengameart.org/content/animated-characters-pack

Original archive:
https://opengameart.org/sites/default/files/ultimate_animated_character_pack_by_quaternius.zip

Archive SHA-256:
`0295062cbc72da52ee804f39ac38694901a40458011f9b2638fc7ef13ed509da`

- `casual-man.glb`: original `Blends/Casual2_Male.blend`.
- `casual-woman.glb`: original `Blends/Casual2_Female.blend`.
- `town-gentleman.glb`: original `Blends/OldClassy_Male.blend`.

All are **CC0 1.0 Universal**:
https://creativecommons.org/publicdomain/zero/1.0/
These archived CC0 assets are independent of the author's newer asset-license
changes. The original meshes, weighted skeletons and authored Death, Idle, Walk, Run
and RecieveHit (author's spelling) clips are retained. Materials use original
authored colors, exported as self-contained standard PBR GLBs with no textures
or external references. No generated rig or simulated button animation is used.

Converted with Blender 4.2, using the same export recipe as the character
catalog (with those five civilian clips selected). Source/conversion artifacts
remain in `/data/tawkathon-pedestrians/`. Output hashes and exact rig/clip counts
are in `validation.json`; each file passed the app's strict `inspectGlb` import
validator as rigged and playable.

Six authoritative pedestrian identities reuse the three models with independent
skeleton clones and animation mixers. Model identity survives packet/object
updates. Render height is 1.5 units, visibly distinct from player costumes.
Wallets are small original meshes; their aimed marker contains no giant labels.
Only a wallet/card reacts to suction, while the body keeps its authoritative
route. Bills fly to the vacuum holder after the server's npc_robbed event.

Run-over civilians play the author’s Death clip once and hold its final prone
pose until the authoritative round reset. Late joins see the settled pose.
The finite wallet stays available at ground level, with no new collision cash.

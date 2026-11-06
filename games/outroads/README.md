# Outroads

Drive real streets anywhere in the world with your lobby. Pick a city, a small town or a spot near you, everyone spawns there, and you cruise around together on OpenStreetMap roads drawn in 3D.

- `client/` is the activity frame. It streams OpenStreetMap vector tiles from OpenFreeMap around the camera, turns them into meshes in a worker, and draws them with three.js.
  - `src/lib/terrain.ts` turns elevation pixels into a height grid per tile, read pixel for pixel so neighbouring tiles share their edges. Roads are draped over it, bridges run straight between their ends, buildings stand on the lowest ground under them, and cars follow the slope and take air off crests. Hills can be turned off in settings.
  - `src/lib/mvt.ts` reads vector tiles. `src/lib/tilemesh.ts` turns one tile into roads with centre lines, extruded buildings at their mapped heights. Small buildings with no mapped height become houses with gable roofs, and ones standing wall to wall become flat roofed rowhouses. Where OSM maps no houses along a street, plain generic filler houses can stand in (a setting), water, parks, land use, street lamps and trees, plus the road, name and wall segments that driving needs.
  - `src/lib/world.ts` loads tiles within a radius and draws buildings in sixteen chunks per tile. Past a detail distance only buildings taller than 18 m stay drawn, and past the view distance nothing does. Lamps and trees drop out with the detail.
  - `src/lib/physics.ts` is the arcade handling. Speed is capped lower off road, the handbrake lets the tail out, and cars bump off buildings and each other. Snap to road assist eases the car along the road it is on.
  - `src/lib/game.ts` is one player's drive: the camera modes (chase, far, hood, photo), remote cars drawn 120 ms in the past between poses, headlights, honks and the minimap.
- `src/server.rs` is the drive socket on the party port, `/party/outroads/ws`. It keeps one room per lobby in memory and relays car poses ten times a second. It also answers place searches through `src/geocode.rs`.
- `src/room.rs` holds the spawn point, the host (the lobby owner while they are in it, otherwise whoever has been driving longest), the ballot, the hour of day and each driver's colour. The host moves everyone. Anyone can put a place to a vote, and more than half the drivers moves everyone.
- `src/runtime.rs` registers the lobby game.

## Data and credits

| What | From | Terms |
| --- | --- | --- |
| Roads, buildings, water, land use | OpenFreeMap vector tiles (OpenMapTiles schema), fetched by the client | Free, no key. Data is © OpenStreetMap contributors under the ODbL. The schema is © OpenMapTiles. |
| Place search | Photon by komoot, through the server | Free, fair use. The server sends one request a second at most, caches answers for a day, and sends its own User-Agent. |
| Hills | Terrain Tiles on AWS Open Data (Terrarium PNG, zoom 13), passed through the game server | Free, no key. The credits list the sources' required attribution (USGS 3DEP, SRTM, GMTED2010, ETOPO1, EU-DEM and others). |
| Satellite ground (off by default) | Sentinel-2 cloudless 2016 by EOX, layer `s2cloudless_3857` | CC BY 4.0 for the 2016 layer only. Later years are not free for commercial use. |

The credits stay on screen while you drive.

Tiles are cached in IndexedDB, capped at 120 MB and trimmed oldest first. Activity frames have no origin, so browsers refuse storage there and the cache lives in memory for the session.

## Privacy

Searches go through the server, so Photon never sees players. "Near me" asks the browser for a location, rounds it to about 100 m, and sends nothing until the player chooses to share it as the spawn point. The spawn point is public to the lobby, and car positions are in meters from it. Inside a tawks frame the browser blocks location, so players search for their town instead.

## Running it

It runs inside the tawks games service, which mounts `OutroadsServer::router()` on the party port and calls `runtime::register` with its bot.

| Setting | What it does |
| --- | --- |
| `OUTROADS_ACTIVITY_URL` | Where the built client is served. Without it the game stays out of the lobby menu. |
| `OUTROADS_SECRET` | Signs seat passes. Defaults to one derived from `PARTY_SECRET`. |
| `OUTROADS_TERRAIN_URL` | A Terrarium tile source. Defaults to the AWS Open Data bucket. The server serves it at `/party/outroads/terrain/{z}/{x}/{y}.png` with CORS, because the bucket sends none and game frames have no origin. |
| `OUTROADS_GEOCODER_URL` | A Photon compatible search endpoint. Defaults to `https://photon.komoot.io/api/`. |

To work on the client without tawks:

```sh
cargo run -p outroads --example dev
pnpm --filter @tawks/outroads-activity dev
```

and open the printed links, one per driver. Add `&stats` (or `&debug`) to show frame stats, and `&touch` to show the touch controls on a desktop.

## Later

The room keeps a spawn point in latitude and longitude, and positions in meters from it, so races between two addresses, delivery runs and "guess where we are" rounds can live in the room as modes, each with its own state next to the ballot.

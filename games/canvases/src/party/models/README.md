# Player model imports

The importer accepts self-contained glTF 2 binary (`.glb`) files. Whole models
keep their own materials and appearance; the base character's fitted store
clothes do not transfer to an arbitrary mesh.

Static models can be saved and previewed. Equipping a model requires an actual
skinned mesh and two distinct animation clips mapped to idle and locomotion.
The importer detects common clip names and lets the player map other names.
Optional cheer, sad and fall clips default to idle when absent. This path does
not call an external service, generate a rig, retarget a skeleton, or manufacture
missing animation clips.

`custom-model-rig.ts` supplies the same `CharacterRig` interface as the built-in
character builder. It normalizes the body to a 1.8-unit height (and bounds its
width), removes horizontal root motion, switches animation actions with a short
fade, and disposes textures, materials, geometry, skeletons and mixer state when
the player changes models or the view is destroyed. Server movement and game
hitboxes remain authoritative.

## Host integration

Mount `../model-import.svelte` with `api: ModelImportApi`. The Tawks adapter is
`apps/tawks/src/lib/party-models.ts`, exporting `partyModels` with list, get,
upload and remove methods using the configured API base URL and account token.
`onimport(model)` reports a saved model; `onequip(model)` is offered only for a
playable model and delegates ownership/purchase checks and costume persistence
to the host wardrobe. `onclose` closes the surrounding modal.

The API routes are:

- `GET /party-models`: the signed-in person's library.
- `POST /party-models`: multipart `file`, `name`, and optional JSON `clip_map`.
- `GET /party-models/:id`: appearance metadata for another authenticated player
  (including a guest session) in the room to resolve; it does not expose the owner's library.
- `POST /party-models/:id/delete`: owner-only library removal and storage cleanup.

Migration `0129_party_imported_models.sql` records ownership, authoritative
validation results, storage status and deletion. Wardrobe application must check
`user_id`, `status = 'ready'`, `deleted_at IS NULL` and `report.playable`; upload
metadata or browser flags alone never grant permission to equip. Incomplete
uploads count against the quota, and the owner's row lock serializes concurrent
uploads. Models are stored in the existing Tawks media bucket under random model
IDs, with a full CDN URL returned for desktop and browser clients.

## Rendering limits

Validation runs before local parsing and again on the API before storage:

- 10 MB per GLB; eight active imports per account.
- 60,000 instantiated vertices and 30,000 triangles; 64 meshes / 96 mesh parts.
- 128 hierarchy nodes; eight skinned parts, at most 96 joints per skeleton.
- Eight embedded PNG/JPEG images, maximum 2048 × 2048 each and eight megapixels
  combined; 32 materials.
- 24 uniquely named animation clips, maximum 120 seconds and 100,000 keyframes.
- One scene; bounded dense accessors and valid binary references.

External/data URIs, custom/compression extensions, sparse accessors, morph-target
animation, non-finite coordinates, oversized textures, cyclic hierarchies and
invalid bone/triangle indices are rejected. Standard PBR plus unlit, IOR,
specular, clearcoat, emissive-strength and texture-transform extensions are
supported. The loader refuses network resources inside a model even after
validation. Imported files are user-provided; no third-party models are bundled
with this implementation.

Run backend regression tests with:

```sh
cargo test -p tawks-api --lib party_models::validation
```

# Tawkathon costume assets

The matching round character and its costume geometry are generated in
`../characters.ts`. These are original meshes built from primitives; they do
not include EA/PuckLab game-derived character geometry. New costume mesh
shapes are provided under CC0 1.0 (https://creativecommons.org/publicdomain/zero/1.0/).
The surrounding application code retains the repository's MIT license.

Existing map props are Kenney CC0 assets; see ../board/models/README.md.
Imported player models retain their embedded appearance and their uploader's
rights; standard clothing is intentionally disabled on custom models.

The shared catalog is read by the game bot, store validation, and wardrobe UI.
Every accessory currently costs 10 tickets, permanently unlocks an equipment
slot option, and has no gameplay effects. Purchasing the same accessory twice
returns the existing entitlement without another ticket debit.

# Encore

Play a Clone Hero song library together in a tawks lobby: guitar, bass, rhythm, keys and drums, each player on their own controller.

- `src/library.rs` scans song folders (`song.ini`, `notes.mid` or `notes.chart`, audio stems).
- `src/formats.rs` and `src/chart.rs` read both chart formats into one note list in milliseconds.
- `src/room.rs` is a lobby's band: parts, ready checks and a shared start time.
- `src/server.rs` serves the song list, charts and stems to players holding a pass, plus the room socket. It runs on the party port.
- `src/access.rs` limits the server's own song folder to the accounts in `ENCORE_LIBRARY_USERS` (comma separated). Empty means nobody, so a library of songs you have no license for never reaches the public.
- `src/uploads.rs` checks songs players upload (`ENCORE_UPLOADS=1`): a chart plus Ogg, Opus or MP3 audio, size and length limits, and the "I own this or have the right to use it" box. Tawks stores them in protected storage; an upload only shows up in rooms its uploader is in.
- Only songs tawks marks official earn milestones or other rewards. Library songs and uploads earn nothing.
- `src/scores.rs` keeps every part played through (at least 90% of its notes judged, no auto kick) in the tawks database, with each player's best per chart. The frame reads personal bests and a chart's top 50 for friends, or for everyone on official songs, through `/party/encore/scores`.
- `client/` is the activity frame: Web Audio playback, controller input and note judging, all on the player's own clock.

Every player judges their own notes against their own audio, so network lag only delays the scores others see. The room agrees on one start time, and each client estimates the server clock from pings to start on it.

Run `cargo run -p encore --example scan -- /path/to/songs` to check how a library reads.

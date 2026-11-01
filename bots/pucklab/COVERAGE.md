# PuckLab domain and gameplay integration inventory

This describes actual source behavior in the prepared PuckLab service and
Tawks account-link branch. A visible tab or client data report does not prove
an online game ran.

| Surface | Existing authoritative domain | Current bot coverage / next work |
| --- | --- | --- |
| Lab club/cards/packs/market | `IceLabStore`, `/v1/icelab/state`, `/action`, versioned `/v1/lineup`, reward rules and account wallet | Not embedded. PuckLab Online shows Lab Rivals standings and your own record read-only and links to PuckLab, where clubs, packs, the market and lineups are managed. |
| Jerseys/roster drafts | `jersey_save/design`, `library_players/edit/uniform/set_logo/publish/discard`; same-permission `/v1/editors/*` adapters | Not embedded. Edited in PuckLab. The bot still mirrors each Lab club's home uniform onto its owner's Tawkathon shirt (`src/jersey.rs`). |
| HUT opponents | Original legal-build/card freezing plus the disabled native-service targeted Tawks exchange; accepted fixture freezes both real club lineups/builds | This branch adds exact-provider/two-accepted-person validation with both current Hockey memberships and installations. Capability metadata remains hard-disabled for launch/provider registration. Existing queue controls remain absent; no random queue call substitutes for a targeted opponent. Pregame cancellation is available in the native overlay only before its irreversible launch claim. |
| HUT game preparation | `legacy_prepare` → `LabGames.prepare` → `MatchLedger.freeze`; native export separately compiles roster/lines/jerseys/settings | Not embedded. The bot no longer offers game details or preparation; this needs the trusted gameplay contract. |
| Native loading reports | Match-scoped Legacy admission, prepared/loaded receipts bound to session, snapshot, bundle and attempt | Not embedded. No bot view reads match reports. |
| Installed-game launch | Tawks `RegisterGame`/`GameSession`/`GameLaunch`; current Legacy invite launch protocol. PuckLab Godot separately handles account identity, frozen bundle and private native admission | Native overlay now durably binds an eligible targeted Tawks session to a frozen fixture. A separate typed desktop/account-menu dispatcher now supports an installed development component; production packaging and release verification remain required; the old Hockey launcher is unchanged. A generic Tawks launch ticket is not a PuckLab match grant, and bot proofs never become normal login credentials. No launch button is advertised. |
| Shared chat/challenges | Tawks GameInviteService already owns challenge, host approval, begin, notification and chat-card updates | PuckLab Online has a Chat tab on the channel's own chat (`hockey-pucklab`). Challenges should use Tawks GameInviteService posted into that chat, with the respondent identity and chat membership, not a shared workflow's owner. Need persisted cross-system session binding and a safe match projection. The bot never sends an external message. |
| Live telemetry/results | `MatchParticipation` and `MatchFinal` bind frozen humans, real-time participation and validated finals; production authority defaults to `None`. Stand-in PuckLab result submission is currently disabled | No public account endpoint may create participation/final evidence or rewards. The bot shows no live or final match data. Next engine work needs authenticated actual gameplay facts and settlement/reconciliation; live display telemetry should remain separate from reward authority. |
| My Skater | `/v1/skater` details, face/portrait endpoints, `CareerSetupMixin`; name/position/number validation, face ownership attestation and PNG limits | Not embedded. Edited on PuckLab's own skater pages, which PuckLab Online links to. |
| Personal Career | `/v1/career/state/action`, run/journey/progression, offers, training, interviews, recovery and retirement | Not embedded. `career` feature is off in pinned rules. Preserve frozen-world ownership, action references, preview hashes and paid access. Simulation actions are distinct from human gameplay and cannot substitute for engine results. |
| Shared Clubs | `/v1/clubs/state/action`, memberships/roles, readiness, season/fixture preparation, competition and recovery | Not embedded. `clubs` feature is off. Preserve captains/owners, ready skater bindings, shared schedules and paid-resource gates. Native Legacy preparation currently accepts Lab modes, not Clubs skater control. |
| Fantasy leagues | `/v1/fantasy/state/action`, league ownership/membership, draft, trades/free agents, schedule/rules, league lineups, challenges | Not embedded. Reuse these domains; do not repurpose a Fantasy challenge as a Lab HUT match or copy its economy. Server role/rules/season locks remain authoritative. |
| Ranked/online exhibition | `/v1/ranked/state/action`, `/v1/online/state/action` | Not embedded. Their current `/play` paths are server simulation and intentionally unavailable through the bot. Legacy PuckLab private admission currently accepts `lab_rivals`, not these modes. |

The safe next gameplay sequence is: Tawks-authorized opponent/host acceptance;
PuckLab participant binding and legal frozen HUT fixture; native bundle and
same-account identity/admission; actual loaded engine/control proof; genuine
participation and final evidence; authoritative settlement; refreshed private
views and safe shared chat projection. None of the early steps authorizes a
later one automatically.


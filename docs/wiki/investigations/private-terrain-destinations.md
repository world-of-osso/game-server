# Private live-run terrain destinations

Verified 2026-10-11 against Retail build `dcfc90fffd79ba00406ae46f5f657592`. The cooldown-assist and SmartAI falls originate in private setup coordinates, not missing ground or client streaming. This repeats the [Stranglethorn audit failure](stranglethorn-ground.md).

## Evidence

Private UDP5881, fresh `fb_*` accounts and a SQLite backup; no canonical world.db writes. Baseline engine `95ab4449c`, server `5dfebc696`, protocol `6ba2f5cdd`.

| Case | Map / WoW X,Y | Supplied Z | Retail surface |
|---|---|---:|---:|
| cooldown-assist | 0 / -8985,-179 | 76.9624 | server79.55275; client79.55338 |
| smartai-erland | 0 / 1407,1090 | 54 | server/client60.478443 |

Baseline per-second instrumentation records replicated authority below the surface while the client is still Loading and has no parsed tile. Once parsed, client terrain returns the surface above, but the already-underground player continues falling. Tiles complete without failures. Thus waiting for client streaming cannot repair these destinations. The server/client Elwynn sample discrepancy is below0.001 yd, not the2.59 yd error in the supplied destination.

`shared::ground::select_ground` accepts only candidates within `STEP_UP_HEIGHT=1.6` above the feet. Both supplied heights are below that reach. Expanding reach or snapping players upward would change collision/ceiling semantics and conceal incorrect data; neither was changed. No claim about undocumented Retail recovery behavior is required.

## Root fix

Private fixtures now have one build-qualified source, [private-live-positions.json](../../../scripts/private-live-positions.json), with unchanged map/X/Y and corrected authored terrain Z. [place-private-character.py](../../../scripts/place-private-character.py) applies that source via explicit admin teleport; it requires a private admin socket, an `fb_*` account and matching character ownership. It does not start a server, mutate world.db, lift a live falling player, or change ordinary teleport/gravity behavior.

Run before client login, inside the agent slice:

```text
GAME_SERVER_ADMIN_SOCKET=/tmp/game-server-admin-5881.sock python3 scripts/place-private-character.py cooldown-assist fb_terrain_case Terrainproof --admin /path/to/game-server-admin
```

The raw AzerothCore Erland/NPC data in the earlier private overlay remains historical data, not corrected Retail content. This fixes the named private player destinations only; it does not silently rewrite NPCs, production starts, saved characters or arbitrary admin coordinates.

## Regression boundary

`ground::private_runs_tests` consumes the same manifest, parses real WDT775971/ADT/WMO assets and executes production authority gravity for200 ticks per destination, checking surface position and unchanged health. With the original Z values it fails on tick0 (`76.9624` versus actual server surface`79.55275`). A separate test retains `Unsupported` for both old underground coordinates, proving no collision clamp was introduced.

Four inspected1280×720 PNGs prove the original map/X/Y in Modern and Forever, grounded for15 seconds after loading: `elwynn-modern`, `elwynn-forever`, `silverpine-modern2`, `silverpine-forever`. Both authority and prediction stay on the measured surface, health292/293, all9 tiles parsed, zero failures. Silverpine uses the original Undead faction; an earlier Human probe stayed grounded but died to nearby Horde NPCs and is not acceptance evidence.

The maintained [native live fixture](../../../../game-engine/godot/tests/private_terrain_destinations_live.gd) captures per-second authority/prediction/terrain, saves final JSON/PNG and fails if either height is off-surface or the player is dead. It rejects any server argument other than private5881. Final-check results and exact revisions remain in the task handoff/proof ledger.

## Sources

- Engine `data/diagnostics/terrain-fall-2026-10-11/`: baseline logs/PNGs, copied SQLite and private redbs.
- Original engine `data/diagnostics/cooldown-assist-2026-10-11/run10-modern/` snapshots; SmartAI `live/retail.redb` preserves X1407/Y1090, and its handoff records original Z54.
- [Server regression](../../../crates/server/src/ground_private_runs_tests.rs), [ground query](../../../crates/server/src/ground.rs).

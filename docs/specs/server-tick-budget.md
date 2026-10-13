# Server tick budget

Keep login, catalog publication and admission from stalling the server's 20 Hz simulation. Sources: `frame_stats.rs`, `system_timings.rs`, `achievement.rs`, `admission_geometry.rs`. [Investigation and measurements](../wiki/investigations/server-tick-lag.md).

## What it must do

- [ ] Keep steady simulation within the existing 50 ms frame budget; login/catalog/admission work must not occupy a frame beyond one budget period. Preserve frame-overrun reporting and tick rate.
- [ ] Attribute slow frames to individual systems, serialized message types/bytes, and tick-thread database operations.
- [x] Evaluate simultaneously observed achievement facts without rescanning grants for each fact; preserve final counters, earned IDs and every toast.
- [x] Publish one final achievement snapshot and one Adventure Guide refresh per connection per frame; retain every toast and persisted contribution, with no idle replay.
- [x] Health-percent aura observation must not scan idle world populations; preserve threshold crossing, removal, reapplication and apply-below-threshold behavior.
- [x] Keep admission pending while surrounding ground, M2 floors, LOS and navigation are loaded off the tick thread. No successful entry or player activation before preparation completes.
- [x] Missing tile files count as prepared/no geometry; read or parse errors log explicitly and fail admission. No timeout admits unready geometry.
- [x] Connection bootstrap must not enumerate every hidden world entity; visibility changes, unlink/re-entry and component filters must retain the same network results.
- [x] Password verification runs off the tick thread; successful completion rechecks bans and credential rotation before minting a token and authenticating.
- [x] Grant evaluation avoids walking unrelated empty criterion trees, retaining zero-threshold and earned-achievement dependency semantics.
- [ ] Toy presentation preflight is shared across owner snapshots and refreshed when source catalogs change; ownership/favourites remain account-specific.
- [ ] Sleeping-unit detection geometry is initialized once and refreshed on activation, not rescanned every tick.

## How it works

- [Tick-lag investigation](../wiki/investigations/server-tick-lag.md)
- [Ground assets](ground-assets.md)
- [Achievement publication](dungeon-achievements.md)

## Implementation inventory

- `crates/server/src/frame_stats.rs`, `system_timings.rs` — frame and system diagnostics.
- `crates/server/src/achievement.rs`, `achievement_catalog.rs` — fact evaluation and coalesced publication.
- `crates/server/src/auras/health_pct_trigger.rs` — indexed health watchers.
- `crates/server/src/admission_geometry.rs`, `auth_character.rs` — pending admission and geometry completion.
- `crates/server/src/ground/preparation.rs`, `line_of_sight.rs`, `navigation.rs` — worker-prepared cache installation.
- `crates/navigation/src/query.rs` — staged navigation tile reads and installation.
- Shared-protocol `src/los/map.rs`, `src/ground.rs` — prepared LOS cache merge and immutable WMO placement sharing.

## Tests asserting this spec

- `crates/server/src/achievement_perf_tests.rs` — concrete imported spell facts, persistence, final snapshot and toast delivery.
- `crates/server/src/auras/health_pct_trigger.rs`, `auras/tests.rs` — 250k-unit catch-up budget and authentic aura lifecycle.
- `crates/server/src/admission_geometry_tests.rs` — actual blocked WDT FIFO, absent assets and corrupt asset admission.
- Shared-protocol `src/los/map_tests.rs` — prepared tiles stay usable without rereading changed disk files; missing/corrupt distinction.

## Known gaps (current cycle)

- [ ] Paired live before/after frame-budget proof on private UDP5831.
- [ ] Remaining synchronous account/catalog hot spots must be measured after geometry preparation.

## Out of scope

Production mutation, tick-rate/budget changes, timeout-based admission and removal/suppression of performance logs.

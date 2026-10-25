# Windwalker stale fixture controls

Approved after Xuen's recovery; independent `ww-fixture-controls` from current origin/master09f6866fb05e1fa160bd33a64404cabb8129bc7f. Xuen36eeb2339/xr1111 and SCKc743e921e/sk1106 remain frozen. Classification was reported to the lead before any test/code change.

## Classification

**Zenith is a fixture-input failure, not a wrong gameplay premise.** The supplied plain-master3f receipt/log fails at `rotation.select` with `NATIVE_FIXTURE_TARGET_UNAVAILABLE: explicit lifetime/role missing`. Native default-self readiness and enemy target IDs already pass. `NativeFacts::target_fact` requires explicit `FixtureTarget` metadata for `time_to_die`/`is_boss`; `prepare_measurement` does not install that metadata. The control now calls the existing shared fixture-lifetime/role setup before source-row selection. Its actual enemy-context/self-aura assertions remain unchanged. No TC premise change or production fix is needed.

**Movement distance is legitimately configured; no error is swallowed.** `native_monk` calls `prepare_literal`→`spawn_reference`, which explicitly inserts `FixtureMovement { distance_to_move: 0.0 }`. The current provider returns this declared0; `movement.distance>6` is false, so the native literal selector correctly returns `Ok(None)`. `Driver.select` propagates provider/program errors with `?`. The old test incorrectly treated this explicit configuration as absent.

`simc_apl_native/movement_distance_tests.rs` already covers direct provider zero/nonzero, missing and dungeon behavior. Those tests remain unchanged. The replacement preserves distinct native literal-driver integration: configured0 rejects the row, configured7.25 admits native Tiger Palm, and removing only fixture configuration propagates the named missing-data error through the actual selector. No movement-event scheduling, gameplay values, readiness or production error handling changes.

## Proof scope

| Boundary | Existing failure | Current proof |
|---|---|---|
| Zenith target_if enemy context and actual native self aura | Supplied plain3f failure; missing role/lifetime | GREEN pending; follower identity |
| Movement configured0/nonzero through native literal selection | Supplied plain7b failure; stale named-gap assertion | GREEN pending; Player identity |
| Missing configuration error through native literal selector | Provider gap already covered; selector explicitly retains it | GREEN pending; Player identity |

Existing failure artifacts, not rerun merely to recreate RED:
- `game-engine/data/diagnostics/ret-parity-2026-10-23/hostile-primary-4996/zenith-master3f-once.{proof.json,log}`
- `game-engine/data/diagnostics/ret-parity-2026-10-23/hostile-primary-4996/movement-master7b-once.{proof.json,log}`

Current base09f includes shared buff-fact additions after7b; the two controls, fixture movement setup and relevant target/movement provider contracts are unchanged. Verification will run the native Zenith control with follower identity and all five native row-gating cases with Player identity, using frozen269 inputs/current read-only SQLite snapshots. The supplied provider missing/dungeon controls are retained, not weakened or duplicated as a replacement for native selector coverage. Native GREEN, format/readability/diff receipts and a separate lead gate remain pending. No worker push or spawned agents.

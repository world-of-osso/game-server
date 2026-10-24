# Literal Player dispatch proof

Scope: shared fixture readiness/delivery only. No Arms or other spec DPS qualification. Branch `arms-literal-player-dispatch`, based on published post-ar1028 master9d13ac19c; hp1026/master0b9dd12a5 merged before GREEN as e99b879e8.

## Behavior matrix

|Boundary|Observed RED at4f91688e8|GREEN at e99b879e8|
|---|---|---|
|Real Human Player revalidates known spells after readiness|Cleared spellbook still casts successfully|Normal request refuses `NotKnown`, no extra CastSuccess|
|Explicit secondary target receives the cast|CastSuccess names primary4294967225 rather than selected4294967224|Selected secondary receives actual cast; CombatTarget and both AA timers/victim unchanged|
|Readiness belongs to selected target|100m secondary borrows nearby primary admission|Selected far target unavailable despite ready primary|
|Expired selected identity cannot redirect|Prepared cast succeeds on primary after selected add despawns|Explicit request refuses stale target; no CastSuccess|
|Harmful precast resolves native selected enemy|Empty precombat candidate order loses actual enemy|Native CombatTarget supplies harmful precast recipient|
|AA retarget changes only weapon victim|No usable retarget action|Victim changes; swing timers and CombatTarget remain; active AA is not restarted onto primary|
|WW creature admission stays unchanged|PASS control: creature cast after cached readiness still permits its native spellbook policy|Same native follower control passes|

Tests use a real native Human1 Monk with unchanged pinned269 gear/trait input; that profile's race metadata is Troll. They assert actual actor identity, not a false Human profile precondition. Actual native events/state are asserted, not source text or internal call counts.

## Proof ledger

Command through engine `scripts/agent/agent-run arms-parity`:

```text
python3 scripts/build-host.py --build-host native --test -j 2 -p server --bin game-server literal_player_ -- --ignored --nocapture --test-threads=1
```

Admission: explicitly extended `lead_arms_14775`, ADD/HOLD/SHED enforced. Inputs/private DB and raw receipts live under engine `data/diagnostics/arms-parity-2026-10-23/`.

|Scope|Result|Validity|
|---|---|---|
|dad769b88 setup attempt|E0609: Position is named-field, not tuple|Compile-only, NOT behavioral RED; corrected to Position.x in4f91688e8|
|4f91688e8 `player-dispatch-behavior-red`|1PASS/6 actual assertion RED;20m47 compile/34.94s native runtime|Source-backed failure boundaries above, no compiler warnings|
|e99b879e8 `player-dispatch-hp1026-green`|7PASS/0fail/0ignored;18m08 compile/27.17s native runtime;0compiler warnings|Current relevant Rust scope after required hp1026 merge; later docs-only changes preserve proof|

Earlier GREEN sampler stopped before Cargo admission when hp1026 merge was requested; no pre-hp GREEN result is used. Private snapshot provenance remains `native-snapshot.proof.json`; exact269 inputs are recorded in `player-dispatch/inputs.proof.json`.

## Representation and exclusions

`Prepared` keys native readiness by `(action, explicit target)`; untargeted lookup has its own native-default entry, never an explicit-target fallback. Player `Execution::Spell` invokes ordinary `SpellCastIntent` handling with an explicit entity; NPC/follower execution retains `creature_cast`. Normal Player requests now reject a missing explicit identity instead of silently using primary. `retarget_auto_attack` mutates only `AutoAttack.victim` (SimC76f8118 `player.cpp10824–10860`).

Composite expansion, resource-limited/pool hooks, remaining target/dot/raid-event/snapshot facts and timed-add lifecycle are separate open work. Native per-spell numeric qualification,300s DPS, broad checks/readability and final integration are not claimed; lead owns independent integration verification.

# Vazruden the Herald / Nazan

Implement the Ramparts encounter from pinned TC `a352b1fa` `boss_vazruden_the_herald.cpp` and `instance_hellfire_ramparts.cpp`. Installed Retail `12.1.5.70448` owns encounter-credit identity; TC owns the completion condition. [Source audit](../wiki/systems/boss-scripts.md#ramparts-herald-identity-policy--2026-10-17).

## What it must do

### Identity and data

- [x] Authenticate and record binding identity under named policy `ENCOUNTER_CREDIT_FROM_RETAIL_DUNGEONENCOUNTER`: verify active70448 DungeonEncounter1892/map543/bit1, Journal529/instance248 and actor1106/display18944 against source Herald17307. Do not substitute older Journal data when missing or disagreeing; record named gap `HERALD_RETAIL_IDENTITY_GAP` instead.
- [ ] Retain source actor roles: Herald17307 becomes Nazan17536 on the same actor;30717 summons Vazruden17537. A shared display alone cannot select a primary or create extra credit bindings.
- [ ] Prepare only independent database copies from actual TDB1210 templates, difficulty rows, models and addons. Preserve authored fields, population and other encounters; no synthetic Normal row, legacy-stat substitution or admission override.
- [x] The `herald` spell producer preserves every field of the seven authored18430/20538/30693/30694/30696/30698/30699 spell/effect pairs, idempotently; missing input refuses atomically without unrelated-data changes.
- [ ] Preserve group357's actual ordinary initial population: Herald and two17517 Sentries, groupFlags0. Its forced reset must not require changing it into a manual-spawn group.

### Prepared actors

- [x] Project17537's actual base0 and Heroic2, and21252's actual base0/Normal1/Heroic2, with source models/addons. Do not create a Normal1 row for17537; preserve unrelated difficulties, Nazan, population, bindings and group357's flags.
- [ ] Resolve Nazan17536's sourceCT0 through its real map/area context before runtime use. Record `HERALD_NAZAN_MAP_TUNING_UNIMPLEMENTED` until implemented; neither legacy62 nor an uncontextualized level1 cache is acceptance.

### Lifecycle and phases

- [ ] Two actual17517 Sentry deaths in one copy start the Herald once through source action1730700; other copies and unrelated deaths do not contribute.
- [ ] Perform the source center movement, arrival-plus1s intro, landing/morph, grounded30717 summon, flight/hover/facing and zone engagement. Use actual movement completion, not fabricated arrival callbacks.
- [ ] Preserve flying schedules, last-flight exclusion and passive cast suppression. Descend on source actions or incoming damage below30%; retain minimum1 health until the source fall and subsequent2s transition finish.
- [ ] Preserve Vazruden's5301 appearance cast,5s aggro speech,5–10s Revenge attempts and incoming-below40 Call Nazan30693. Failed ordinary admission must not be bypassed; pinned TC itself notes the Revenge prerequisite problem.
- [ ] Run source ground-phase casts, fireball/cone liquid-fire children and object lifetimes with authored targets and effects; retain Heroic branches as unverified until tested, not silently replaced by Normal behavior.

### Completion and reset

- [ ] Credit1892/bit1 only when both Nazan and Vazruden have actually died, in either order, exactly once and only in their copy. Neither individual death, intro nor entry morph completes it.
- [ ] Activate the authored difficulty-specific chest on completion; preserve loot and persistence boundaries.
- [ ] Preserve source evade/failure,30s forced group357 reset and unloaded/reloaded instance behavior without stale Sentry counts or surviving summoned Vazruden duplicates.

## How it works

- [Source and identity audit](../wiki/systems/boss-scripts.md#ramparts-herald-identity-policy--2026-10-17).
- [Shared boss and instance contracts](boss-scripts.md).

## Implementation inventory

- `crates/server/src/scripted_ai/hellfire_ramparts.rs` — currently registers Gargolmar and Omor, not Herald/Vazruden.
- `crates/server/src/scripted_ai/instance_script.rs` — existing copy-local boss/data state; Herald source lifecycle is not implemented.
- `scripts/encounter_cpp_bindings.py` — `herald` exports primary17307/1892/bit1 SQL only after pinned TC and explicit70448 DungeonEncounter/Journal/actor CSV validation; records the named policy and input hashes.
- `scripts/encounter_serverside_spells.py` — `herald` copy-only profile selects the seven exact TDB spell/effect pairs.
- `scripts/encounter_summons.py` — `herald` projects17537/21252 base/models/addon and actual nonzero difficulty rows;17536 remains unresolved until map-context tuning works.

## Tests asserting this spec

- `scripts/tests/test_herald_templates.py` — actual-world-copy CLI projection of17537/21252, idempotence, exact authored gold and byte-identical identity refusal; GREEN1/1 at252a23c9 (12.947s), with unchanged Omor actual-copy control retained from479979bd.
- `scripts/tests/test_herald_binding_cli.py` — pinned-source/70448 binding CLI, primary-only preservation and named identity gaps; GREEN with unchanged binding controls and warning-free affected producer fixtures.
- `scripts/tests/fixtures/herald-binding/` — exact pinned TC files and extracted active70448 identity rows, with original source hashes.
- `scripts/tests/test_encounter_serverside_spells.py` — real CLI fixture for all seven authored spell/effect rows, idempotence, unrelated-data preservation and atomic refusal; producer suite GREEN6/6.
- `scripts/tests/fixtures/herald-serverside-spells.json` — exact TDB1210 records selected by the pinned script.

No Herald encounter behavioral acceptance exists yet. Source/70448 identity audits and data preparation are not runtime GREEN.

## Known gaps (current cycle)

- [ ] Establish source-data prerequisites and behavioral RED before implementing each bounded slice.
- [ ] Implement and verify real actor/movement/spell/death ordering, both-dead credit and reset.
- [ ] Native rendering/audio, obstacle paths, Heroic behavior, loot and persistence remain unproved.

## Out of scope

- Changing Gargolmar/Omor identity authority or existing verified mechanics.
- Reproducing TC's ignored out-of-range encounter row as a missing Retail credit identity: the named policy above deliberately separates identity from completion timing.
- Inventing semantics for30694, substitute spells, tuning, forced admission or unproven native-parity claims.

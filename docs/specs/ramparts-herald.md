# Vazruden the Herald / Nazan

Implement the Ramparts encounter from pinned TC `a352b1fa` `boss_vazruden_the_herald.cpp` and `instance_hellfire_ramparts.cpp`. Installed Retail `12.1.5.70448` owns encounter-credit identity; TC owns the completion condition. [Source audit](../wiki/systems/boss-scripts.md#ramparts-herald-identity-policy--2026-10-17).

## What it must do

### Identity and data

- [x] Authenticate and record binding identity under named policy `ENCOUNTER_CREDIT_FROM_RETAIL_DUNGEONENCOUNTER`: verify active70448 DungeonEncounter1892/map543/bit1, Journal529/instance248 and actor1106/display18944 against source Herald17307. Do not substitute older Journal data when missing or disagreeing; record named gap `HERALD_RETAIL_IDENTITY_GAP` instead.
- [x] Retain source actor roles: Herald17307 becomes Nazan17536 on the same actor;30717 summons Vazruden17537. A shared display alone cannot select a primary or create extra credit bindings.
- [ ] Prepare only independent database copies from actual TDB1210 templates, difficulty rows, models and addons. Preserve authored fields, population and other encounters; no synthetic Normal row, legacy-stat substitution or admission override.
- [x] The `herald` spell producer preserves every field of the seven authored18430/20538/30693/30694/30696/30698/30699 spell/effect pairs, idempotently; missing input refuses atomically without unrelated-data changes.
- [ ] Preserve group357's actual ordinary initial population: Herald and two17517 Sentries, groupFlags0. Its forced reset must not require changing it into a manual-spawn group.

### Prepared actors

- [x] A separate copy-only `herald-nazan` profile projects17536's actual base0/Heroic2 CT0 fields/models/addon, without changing the existing `herald` profile's preserve-Nazan contract. Preserve CT0 and its unresolved cache bounds1; runtime context, not an importer-assigned220/1065, chooses native levels. No synthetic Normal1, population, binding or authored-provenance changes.
- [x] Project17537's actual base0 and Heroic2, and21252's actual base0/Normal1/Heroic2, with source models/addons. Do not create a Normal1 row for17537; preserve unrelated difficulties, Nazan, population, bindings and group357's flags.
- [x] Resolve sourced17536CT0 at the ordinary-spawn boundary through real map/area context: Normal1→220, Heroic2→1065, source deltas/modifiers and derived level/HP/armor/scaled damage. `HERALD_NAZAN_MAP_TUNING_UNIMPLEMENTED` is retired only for that proved boundary; neither legacy62 nor an uncontextualized level1 cache is acceptance. Fixtures add no canonical Nazan placement.
- [x] Prove actual source Normal/Heroic template use and same-actor `UpdateEntry` through natural INTRO2. `HERALD_NAZAN_ENTRY_MORPH_UNIMPLEMENTED` is retired for this living transition, not ordinary respawn or the full encounter.

### Same-actor projection prerequisite

- [x] Initial spawning and living same-actor entry updates consume one template projection; preserve the caller-selected initial template, RNG order, spawn overrides, deferred addon application and ordinary spawn controls.
- [x] Retain ordinary origin GUID/original entry independently of current `Npc.template_id`. Updating to17536 keeps the entity, current position/map, live AI/event progress, home/waypoint cursor and threat/combat references; source contextual target stats reset health to the new maximum.
- [x] Preserve original spawn-addon priority: Herald content GUID20202693 supplies sheath0/path36215440 instead of Nazan template sheath1/path0. Nonzero mount/emote/path fields act; zero does not erase prior runtime state. Missing default equipment retains it; explicit equipment0 clears it. Addon auras are additive and existing instances are not refreshed.
- [x] Missing target/context/origin refuses before actor mutation with an actionable diagnostic. No legacy level/tuning substitution or actor recreation.

### Grounding-only target32 prerequisite

TC Object.cpp709–748 calls GetMapHeight with pre-clamp Z once, then assigns the returned height; it does not demand that a second query from the assigned Z be a fixed point. Real ray trace at the same XZ gives81.05518 for inputY89.05998 and81.05517 for inputY81.05518, each bit-stable when repeated with identical inputs. Therefore the independent oracle uses the actual finalXZ but the identical pre-clamp Y/ray, and compares bits exactly. No ULP tolerance, rounding, repeated clamping or WMO-reader changes. The natural INTRO2 fixture captures the actual standing caster position immediately before30717's due tick, verifies it is unchanged on that cast tick before first-flight movement, and uses that observed pre-clamp Y rather than a guessed constant.

- [x] Apply TC Object.cpp698–762 Z semantics after existing target32 XY selection, without reading a model/display/reach: valid source floor for represented dry non-hover non-flyers; flyers only rise from below; invalid/missing height preserves Z.
- [x] Attr9 FORCE uses its final ground-Z branch regardless of flight; preserve existing XY. Do not claim FORCE distance-backoff parity.
- [x] Retain actual688 no-height and all follower Felguard behavior without source-reach defaults, changed assertions or widened ignores. Exact floor oracle queries finalXZ using the primitive's same pre-clamp Y/ray, then compares height bits with no tolerance.
- [ ] Keep `TARGET32_CASTER_SUMMON_XY_UNIMPLEMENTED`, `TARGET32_FIRST_COLLISION_PARITY_UNIMPLEMENTED`, `TARGET32_HOVER_SWIM_STATE_UNIMPLEMENTED` and `TARGET32_FORCE_XY_BACKOFF_UNIMPLEMENTED` explicit. Selectors42,44,47,49,50,72,55 remain unchanged under `CASTER_RELATIVE_DEST_ALLOWED_Z_UNIMPLEMENTED`.

### Lifecycle and phases

- [x] Two actual17517 Sentry deaths in one copy start the Herald once through source action1730700; other copies and unrelated deaths do not contribute.
- [x] INTRO1 speaks once, becomes passive, stops the original patrol and physically moves to the authored center. Repeated actions do not replay the intro; patrol remains stopped before the source arrival+1s INTRO2 deadline.
- [x] First INTRO2 slice: actual center arrival→exactly1s→same-actor17536 projection, actual grounded30717/17537, gravity restoration, ordinary30698/30694, copy-local engagement and first real West2 arrival with30700/18430/Aggressive. LAND emits nothing under `NPC_ONESHOT_EMOTE_UNIMPLEMENTED`, with no pose/social substitute.
- [ ] On engage schedule every source event: Herald victim0/random10–15s/fly20–30s; Vazruden speech5s/Revenge5–10s. Preserve ordinary admission and Passive suppression/3200ms victim repeat; assert only effects actually due through first West arrival. Later schedules remain real but unasserted.
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

- `crates/server/src/scripted_ai/hellfire_ramparts.rs`, `hellfire_ramparts/herald.rs` — registers Gargolmar, Omor, bounded Herald INTRO2 and actual Vazruden appearance/engagement. Real NPC death ingress counts Sentry entries in copy-lifetime instance state and dispatches1730700 to the retained ordinary Herald origin. INTRO1 removes active patrol state, not addon path data, then uses real point movement. Three INTRO1 and both Normal/Heroic INTRO2/copy cases are GREEN in the current targeted proof; later lifecycle is excluded.
- `crates/server/src/scripted_ai/instance_script.rs` — copy-local boss/data state and transient Ramparts Sentry count; unload discards it. Forced reset and remaining Herald lifecycle are not implemented.
- `crates/server/src/zones/entry.rs` — `EntryProjection` reads/rolls represented target fields and applies them to a new or existing entity; `CreatureOrigin` retains ordinary source identity, `AddonPathId` is addon data rather than a running movement cursor. Bounded entry/spawn proof is GREEN; not a Herald AI registration or ordinary-respawn restoration.
- `scripts/encounter_cpp_bindings.py` — `herald` exports primary17307/1892/bit1 SQL only after pinned TC and explicit70448 DungeonEncounter/Journal/actor CSV validation; records the named policy and input hashes.
- `scripts/encounter_serverside_spells.py` — `herald` copy-only profile selects the seven exact TDB spell/effect pairs.
- `scripts/encounter_summons.py` — `herald` projects17537/21252 base/models/addon and actual nonzero difficulty rows; the separate `herald-nazan` profile selects17536 base0/Heroic2 CT0 with source models/addon, without changing `herald`. Real-copy producer/refusal/idempotence/preservation controls are GREEN; natural Normal/Heroic INTRO2 now proves living runtime morph separately.

## Tests asserting this spec

- `scripted_ai/tests/herald_intro2.rs` — genuine arrival+1s RED (pre-rewrite4fa9425b8) precedes implementation. Current Normal/Heroic natural arrival+.95/+1, retained entity/origin/AI/map, source17536 stats, real17537/property64,5301/engagement and first physical West arrival/hover/Aggressive/facing bits all pass; other-copy/duplicate and LAND no-substitute controls pass. Casts are observed through actual SpellGo, floor through an independent identical source ray. The two enabled cases pass85.33s at4766f52c0 after master6c8929589 merge. Earlier fixture compiler failures and the rejected post-clamp-Y fixed-point oracle are excluded from feature RED/GREEN claims.
- `spell_cast/target32_grounding_tests.rs` — native30717/688, actual Ramparts geometry, model-free casts, dry/flying/missing-height/FORCE branches and unchanged XY. Genuine3RED/1PASS6.01s at a38c17a65; four GREEN plus full17 follower-pet cases21/21PASS148.29s at901aa4f11. The combined retained43 cases pass150.27s atd970d17cb; only the pre-existing ignored navigation diagnostic is skipped. Test-only facing refinement is separately GREEN above; owned production is unchanged afterfabe7e29a. Six tests deleted with the reverted port are not claimed as executed. Source setup/compiler/protocol-drift failures remain excluded.

- `scripted_ai/tests/herald_intro.rs` — ordinary source Herald/Sentry GUIDs, real lethal transitions, split allocated copies, unrelated/corpse deaths and once-only center movement. Shared fixture defaults stay unchanged; this fixture loads canonical read-only patrol data before reconstructing its ordinary Herald. First b15f94882 run is setupFAIL only (missing fixture waypoints); corrected7844943bc reaches genuine2FAIL21.34s at the missing second-death INTRO1 boundary, then implementation0916fea91 passes2/2 in19.38s. Missing-Herald controls cover retained first death and absent-pair reset; all three INTRO1 cases pass in the current43-case retained batch.

- `zones_entry_tests.rs`, `zones_entry_controls_tests.rs` — source Normal/Heroic same-actor stats/origin/addon, real living-player threat/combat references, optional runtime state/equipment, real EventMap/dispatch probe and actual TDB30/100/176396 addon auras11959/3417/347931, including an exact finite5s deadline that a silent refresh would move. Current eight genuine REDs at1e10b0a51 (27.94s) precede implementation. Earlier missing-addon/clock/master-caller setup failures are excluded. Focused11/11 at5f25cf953 (18.56s), then final affected68/68 at9e83421de (233.18s,55 pre-existing ignored). Supplemental actual Morna/Drudge nonzero addon mount/emote1/1 at65e035a2a (10.14s), with no production changes after the affected batch. Strengthened finite-aura case1/1 at44cdfe55c (17.53s), unchanged production; required package check0/warning-free34.97s and fmt0 on that head. Independent landing gate pending. Neither source fixture setup nor these primitive cases certify encounter choreography/native behavior.
- `scripts/tests/test_nazan_templates.py` — actual read-only world snapshot and real `herald-nazan` CLI; exact represented CT0 base/Heroic fields/models/addon, idempotence, unrelated-data preservation and byte-identical drift refusal. Genuine real-copy missing-profile REDb4a524732 becomes new/Herald/Omor GREEN3/3 at0a3ab4b24 (27.783s), followed by Nazan-only GREEN1/1 at7fe38b45e (26.460s) after read/write naming cleanup; ResourceWarning escalated, warning-free. This is data-only, not morph acceptance.
- `crates/server/src/zones_context_tuning_tests.rs` — sourced17536 ordinary Normal/Heroic map stats, actual MCNK area ancestry, explicit-template/critter/known-zero/missing-context controls, authored-source stats and real PostStartup publication ordering. Seven enabled cases pass within affected54/54 at33dcb7b6 (177.24s); no canonical Nazan spawn or morph acceptance.

- `scripts/tests/test_herald_templates.py` — actual-world-copy CLI projection of17537/21252, idempotence, exact authored gold and byte-identical identity refusal; GREEN1/1 at252a23c9 (12.947s), with unchanged Omor actual-copy control retained from479979bd.
- `scripts/tests/test_herald_binding_cli.py` — pinned-source/70448 binding CLI, primary-only preservation and named identity gaps; GREEN with unchanged binding controls and warning-free affected producer fixtures.
- `scripts/tests/fixtures/herald-binding/` — exact pinned TC files and extracted active70448 identity rows, with original source hashes.
- `scripts/tests/test_encounter_serverside_spells.py` — real CLI fixture for all seven authored spell/effect rows, idempotence, unrelated-data preservation and atomic refusal; producer suite GREEN6/6.
- `scripts/tests/fixtures/herald-serverside-spells.json` — exact TDB1210 records selected by the pinned script.

Acceptance is bounded to natural INTRO1→INTRO2→first West arrival plus represented target32 Z semantics. Sixty distinct targeted cases pass across the21/43 partitions (four primitive cases overlap), with post-merge facing proof above. Independent LEAD check/fmt/integration gate remains pending. Source XY/collision/hover-swim/FORCE-backoff, later combat/descent/paired credit/reset and native behavior are not certified.

## Known gaps (current cycle)

- [ ] Establish source-data prerequisites and behavioral RED before implementing each bounded slice.
- [ ] Implement and verify real actor/movement/spell/death ordering, both-dead credit and reset.
- [ ] Native rendering/audio, obstacle paths, Heroic encounter behavior, loot and persistence remain unproved.
- [ ] `LANTHA_DISPLAY_122612_MODEL_INFO_ABSENT`: canonical and full local TDB1210 model-info omit Lantha230861/display122612. This BLOCKS returning the reach-dependent target32 TC XY port until a sourced reach row exists or the user explicitly decides otherwise; not an accepted production-runtime gap. The separately approved Z-only slice never reads model/reach and leaves XY unchanged. Do not invent reach or broaden ignores.
- [ ] `CREATURE_ENTRY_RESPAWN_RESTORE_UNIMPLEMENTED`: ordinary respawn does not yet restore the retained original entry. Living projection proof is not until-respawn parity.
- [ ] `SMARTAI_ACTION36_UPDATE_ENTRY_UNIMPLEMENTED`: SmartAI36 remains excluded; the shared projection is not an action36 port.
- [ ] Full TC attack-time/resistance/immunity/static-flag/movement-capability projection remains outside the currently represented template fields; no generic TC UpdateEntry parity claim.

## Out of scope

- Changing Gargolmar/Omor identity authority or existing verified mechanics.
- Reproducing TC's ignored out-of-range encounter row as a missing Retail credit identity: the named policy above deliberately separates identity from completion timing.
- Inventing semantics for30694, substitute spells, tuning, forced admission or unproven native-parity claims.

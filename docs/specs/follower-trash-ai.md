# Follower dungeon trash AI

## Contract

Every spawned trash entry in the current27 follower-dungeon maps is inventoried from the union of legacy, content and TDB spawn slots. Boss/encounter actors, inventory Journal model/map actor candidates, source-declared player-friendly factions and noncombat/service candidates remain visible with explicit exclusions. Uncertain encounter candidates are neither generic trash recipients nor donors; withholding them never certifies a primary binding. Counts derive from the current canonical DB, not the historical627/650 estimate.

Ability authority: Retail12.1.5.70448 Spell* rows, TDB1210 creature_template_spell/SmartAI/addons and TrinityCore a352b1fa scripts, then LittleWigs2777a9d trash cast events, then cached Wowhead NPC Abilities. Passive addon auras do not establish a cast kit. Warning-only aura/damage events never become casts. Wowhead requests use only the host-wide locked/cache fetcher, at most one request per10seconds. Player-class-tagged Wowhead rows require TDB/TC/Lua corroboration; otherwise they remain excluded evidence, not NPC bindings. Follower205 selects source-labelled Normal variants (base0/Normal1/Follower205 modes), retaining other IDs as evidence and never casting two difficulty versions of one ability.

If a trash creature has no sourced kit, `FOLLOWER_TRASH_SIMILAR_CREATURE_KIT` records a single-hop sourced donor from the same dungeon or another-difficulty template. Exact name, authenticated TDB display, nonzero family, shared name stem and shared role are descending similarity evidence. Excluded actors never donate kits. Donor entry, source and basis remain inspectable. Bear-family inference additionally recognizes the source-linked bearmount.m2 FileData123058 and bear2.m2 FileData1266661; this is a named project inference, never an invented TDB family value. No matching donor remains `none`; no spell identity is invented.

## Execution

Reuse the existing native SmartAI and spell-casting pipeline. Authentic TDB rows remain unchanged in `tdb_smart_scripts`; project-generated rows belong to `content_smart_scripts`, loaded by the same native interpreter. Existing authored SmartAI/ported TC behavior takes precedence. Literal TC Engage/event/cast/repeat lanes are inventoried, not guessed. The existing TC-script exception ports `npc_arcane_tender` directly: its source ScriptName, appearance/home Stasis Ritual, Reset and attempt-relative12s/22s event clocks remain native rather than approximated by project SmartAI rows. Wild Eruption effect0 follows the pinned per-hit destination hook to375650; its DB2 missile creates375647 through the native effect pipeline. Native375647 creates area-trigger properties25515/template29888; the source CAST/ENEMY action applies375649, whose native periodic clock owns damage. Neither375647 nor missile375650 is itself a damage spell. No damage or placement is invented. Destination-triggered spells that require an enemy object resolve the NPC's actual current victim as pinned TC `InitExplicitTargets` does; no synthetic target, DB2 rewrite or fabricated destination is introduced. Project rows carry `FOLLOWER_TRASH_*` provenance, including donor policy where applicable.

Native Spell* records own cast times, ranges, cooldown/category admission and spell effects. Where source supplies no combat cadence and DB2 has no recovery time, the sole project default is `FOLLOWER_TRASH_UNSOURCED_CADENCE_MS = 10000`. This is project behavior, not a Retail timing claim. Source-authored event clocks remain source-authored, not reconstructed from warning bars.

## Source area-trigger admission

Effect179 destination fields share the source-backed field executor. Admission is feature-driven across every native create-properties ID, never an ID allowlist: supported shape, curve, actions and flags all must be satisfied. Unsupported or missing definitions are skipped with recorded reasons; emit admitted/skipped counts. Movement admission checks source spline-point and orbit metadata independently of MoveCurveId; missing metadata never establishes stationary behavior. The canonical battleground importer includes verbatim `tdb_areatrigger_create_properties_spline_point` rows. Existing26505 and25515 are regression examples, not registrations. Preserve authored radius/scale, duration, entry/exit actions and periodic aura clocks; never substitute DB2 spell radius for missing source geometry. Wider-admission control failures must be reported before narrowing scope.

## Named follow-up: Leymor movement forces

`AT_LEYMOR_ARCANE_ERUPTION_MOVEMENT_FORCE_PENDING` withholds properties25390 while the exact pinned TC `at_leymor_arcane_eruption` force script is unported; three existing field-dependent controls remain explicitly ignored until that follow-up. Never restore guessed-radius lifecycle fallback. Generic Unit forces must apply/remove by source GUID with origin, magnitude and Gravity type, preserving independent sources.

`MOVEMENT_FORCE_GRAVITY_RADIAL_HEADLESS` is an approved project policy, not Retail-sourced physics: negative magnitude pulls server-driven movers horizontally toward the origin at its absolute yd/s; positive pushes away. Source vectors add each tick to intended velocity, clamp without overshooting a pull origin and use navmesh/collision without teleporting or leaving the mesh. Player application remains `MOVEMENT_FORCE_CLIENT_APPLICATION_BLOCKED` pending protocol MoveApply/MoveRemove and engine support. Store enter/exit/exact-source removal and independent-source stacking need separate RED→GREEN proof.

Forecast-only tests use authentic unscripted packs. Newly scripted trash requires an explicit pull decision; fixture updates must never weaken production forecast or creature AI to regain numerical estimates.

## Publication and proof

Implementer never writes canonical world.db. Deliver stable-key SQL and a receipt tested by replay on a fresh canonical SQLite backup; repeated application preserves authored source rows and unrelated content. Lead publishes.

Stockade and Azure are the first native proof batch. A sourced trash creature must fail its native cast test before implementation, then emit a real native cast completion after projection. Controls cover creature_ai, scripted_ai, dungeon_simulator and followers under the shared gate environment, at most6test threads, plus check/fmt. Census/source coverage is not native mechanic certification; missing effects, unsupported selectors and unresolved TC-only actors remain named gaps.

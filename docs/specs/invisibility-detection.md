# Typed invisibility and quest-area auras

Authored invisibility and quest-area eligibility must determine which units each observer can see. Source: TrinityCore a352b1fa Object.cpp `CanDetectInvisibilityOf`, SpellAuraEffects.cpp `HandleModInvisibility`/`HandleModInvisibilityDetect`, SpellMgr.cpp `SpellArea::IsFitToRequirements`, and TDB1210.26091 `spell_area`. [Implementation](../wiki/systems/auras.md#typed-invisibility-and-spell-area-eligibility-2026-10-11).

## What it must do

### Detection
- [x] Aura18 carries its authored MiscValue0 type and summed integer amount, including stacks. Aura19 carries independent detection types/values; zero still marks a type present.
- [x] Detect only when every hidden type has matching detection at least as strong. For unit targets, invisible observers also require reciprocal type flags, except aura335. Game objects are exempt from reciprocity (TC Object.cpp:1003), not from detection of their own hidden types.
- [x] Stationary detector changes, detector removal and late arrivals update per-observer replication. Self remains visible under the existing interest contract.
- [x] Boss vanish still hides its target until the aura ends; hostile/friendly spell relations follow observer detection.
- [ ] Melee and creature acquisition use the same typed predicate; dedicated detector-vs-swing/acquisition request proof is not recorded.

### Spell-area lifecycle
- [x] Requirements preserve area/zone, quest-status masks, signed aura prerequisites, race mask and gender. Autocast/autoremove and ignore-quest-change flags control lifecycle; aura-only changes are not quest-only changes.
- [x] Quest none/incomplete/complete/rewarded states drive authored detection; Jaga29401 becomes visible while eligible and disappears after real reward.
- [x] Import authored TDB rows through the importer without replacing unrelated world tables.
- [ ] Spell-area autocasts must not be cancelable by the player; implemented, dedicated CancelAura request proof not recorded.

### Native acceptance
- [x] Jaga29401 is accepted, five authored balloons are looted for item credit, and the quest is genuinely turned in in Modern and Forever. Both skins assert post-reward giver removal and have inspected PNGs from private UDP5496.
- [ ] Vin29412 and Rendan28029 have an authored eligibility route and real native completion proof; source-data gaps remain, not successful completions.

## How it works
- [Aura runtime and proof](../wiki/systems/auras.md#typed-invisibility-and-spell-area-eligibility-2026-10-11)
- [Safe import and current source gaps](../invisibility-import.md)

## Implementation inventory
- `crates/server/src/auras/{invisibility.rs,unit_state.rs,handlers.rs,mod.rs}` — typed aura values and cancellation source.
- `crates/server/src/{interest.rs,combat.rs,creature_sight.rs,spell_cast/targets.rs}` — observer-specific replication and targeting.
- `crates/server/src/{spell_area.rs,quest.rs,main.rs}` — quest/area/aura eligibility reconciliation and runtime wiring.
- `scripts/{spell_area_schema.sql,mysql_dump.py,import-azerothcore.sh}` — safe incremental and full-import entry points.

## Tests asserting this spec
- `crates/server/src/auras/invisibility.rs` — real aura/stack sums, threshold/type presence, reciprocal flags/aura335.
- `crates/server/src/interest_tests.rs` — real89302/82343/100616/82336 stationary/new observer transitions.
- `crates/server/src/maps/tests.rs` — real32727/type4 preparation preserves authored Cage gate visibility after worldport, before movement/opening.
- `crates/server/src/spell_area.rs` — authored100616 lifecycle, real Jaga area5171, source flags and signed/race/gender requirements.
- `crates/server/src/scripted_ai/tests.rs` — Moloch vanish detection; `scripts/tests/test_spell_area_import.py` — unsigned masks and unrelated-table preservation.
- Engine `godot/tests/quest_objectives_live.gd` — both-skin native loot, credit, reward and post-reward hiding; evidence in engine `data/diagnostics/invis-detection-2026-10-11/live-proof-ledger.json`.

## Known gaps (current cycle)
- [ ] No authored spell-area/quest start/reward/SmartAI route to Rendan's82336 detector in inspected source. Vin has no corresponding Stormwind/29412 spell-area rule. Do not guess rows, strip invisibility or grant detectors/credit to claim completion.
- [ ] Current Core Fragment input is normal difficulty1 and passes server copy-spawn/interest proof; native dungeon loot/turn-in remains unverified. Map530 remains explicitly unhosted.
- [ ] Pet/possessor detector delegation, failed-quest state representation and TC battlefield-specific spell-area conditions are not verified by this NPC slice.

## Out of scope
Canonical world.db writes, UDP5000, master merge and full suites are prohibited for this branch. Generic scripted phasing is not part of the requested typed-invisibility/quest-area change.

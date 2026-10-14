# Alterac captain combat

AV25's ordinary combat ended with ten allied deaths before Captain Galvangar died. Source and retained receipts explain the damage budget and expose an incomplete combat driver; they do not establish a gameplay defect responsible for the wipe. Verified: 2026-10-14.

## Observed boundary

Runtime server24068495, leased protocol7edae0b: all nineteen peer roles were accepted, twenty players entered normally, and combat lasted88.58s. Captain11947 had24,759,352 maximum HP and lowest observed24,519,936 HP:239,416 net HP removed (0.967%). This is net health change, not total dealt damage or a DPS-meter measurement. Fifty-seven Flash Heal completions were recorded; all ten allies died. No health, score, equipment, spell or outcome injection occurred during combat.

The driver commands peer autoattacks, one Shield Wall871 per tank, and repeated Flash Heal2061. It issues no damaging spell rotation and no native-owner attack command. Accepted Protection peers actually know Shield Slam23922, Thunder Clap6343, Revenge6572, Demoralizing Shout1160 and Ignore Pain190456; Holy peers know2050/47788/64843 as well as2061. Knowing a spell does not prove its successful use. The unused abilities must not be treated as active tank/healer throughput.

## Source stats, not a health override

The private runtime `content_creature_template`11947 has ContentTuning663, HealthScalingExpansion11, HealthModifier175, DamageModifier35, BaseAttackTime2000 and BaseVariance1. Its raw legacy `creature_template` row instead has level61 and HealthModifier25; replacing current content with that row would mix versions, not fix combat.

At level90 the selected ExpectedStat row475 (ExpansionID-2, used because11 has no level90 row) supplies CreatureHealth141482.016 and CreatureAutoAttackDps6855.70312. There are no tuning663 ExpectedStat modifiers; warrior class modifier4 is1 for these stats. Existing server f32 arithmetic produces24,759,352 HP, matching the receipt. Its ordinary two-second swing range is approximately479,899–719,849 **before** armor, avoidance, absorbs and active reductions. Allies' observed maximum HP spans482,140–699,120. Thus the template can threaten these actors with ordinary melee alone; that comparison is not per-hit attribution.

Pinned TC `Creature.cpp`3173–3196 derives health and base DPS from ExpectedStat;1650–1651 sets the maximum weapon roll to1.5 times the minimum. `StatSystem.cpp`1138–1147 applies the template damage modifier and attack-speed multiplier. Current server `level_scaling.rs` and `combat.rs::creature_weapon_damage` implement that baseline; creature attack power is explicitly not modeled, so this is not a complete TC damage-equivalence certificate.

## Script gap and causal limits

TC `boss_galvangar.cpp` schedules15284/19134/15589/13736/16856 and evades beyond50yd. Current `scripted_ai::create_ai`/`insert_script_name` do not register `boss_galvangar`. That is a source-parity gap, **not an established cause of excess damage**. No scheduled-script damage should be invented to explain the retained run.

Retained AttackStart identities name only the captain attacking allies; those are target-change receipts, not hit counts or complete damage attribution. No combat-damage ledger or server-tick attribution establishes the precise fatal sequence. Memory-full PSI avg10 reached2.51%, so this run is not timing acceptance either.

Before another victory attempt, exercise the actors' legitimately known damaging/mitigation/healing abilities through normal requests and retain successful-cast and damage receipts. Do not lower NPC stats, grant spells, resurrect inside the match, extend deadlines or force a result. Porting the captain's missing script remains separate from proving why this driver wiped.

## Sources

- [Lifecycle contract](../../specs/battleground-lifecycle.md#alterac-valley--active-implementation).
- [Creature scaling](../../../crates/server/src/level_scaling.rs), [template loading](../../../crates/server/src/world_data.rs), [melee](../../../crates/server/src/combat.rs), [AI registry](../../../crates/server/src/scripted_ai/mod.rs).
- TrinityCore a352b1fa9017138cc2f1dcfc0a51afaa17391ecf: `src/server/scripts/Battlegrounds/AlteracValley/boss_galvangar.cpp`, `src/server/game/Entities/Creature/Creature.cpp`, `src/server/game/Entities/Unit/StatSystem.cpp`; retained byte hashes in `alterac25-source-stat-budget.json`.
- Engine diagnostic directory `data/diagnostics/bg-coverage-2026-10-12/`: `alterac25-bounded-combat-proof.json`, `alterac25-source-combat-audit.json`, `alterac25-observed-npc-attackers.json`, `modern-alterac25-role-combat-11947.jsonl`, `modern-alterac25-role-peer*.json`, `run-alterac-roles-match.py` and `alterac25-source-stat-budget.json`.

## See Also

- [[boss-scripts]] — registered scripted AI boundaries.
- [[game-objects]] — separate IoC pickup dispatcher proof.

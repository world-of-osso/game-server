# Pet battle trainer data

Verified 2026-10-10. Trainer creatures and quest objectives exist locally, but no authentic opponent team was recovered. NPC tamer battles remain unimplemented; substituting wild pets or inventing breeds, levels, abilities or rewards would violate the task's data boundary.

## Local source census

- Retail CASC build `12.1.0.69933`, build key `dcfc90fffd79ba00406ae46f5f657592`: extracted `BattlePetNPCTeamMember` FDID **841648** successfully. WDC5 header declares **0 records, 0 sections**, layout `782EE721`. This is an empty table, not an encrypted-record decode failure.
- Community listfile identifies `dbfilesclient/battlepetnpcteammember.scn` as FDID **4007284**. Local resolver reports that path absent from the root; no payload recovered.
- Local `TDB_full_1210.26091_2026_09_09.7z` world dump contains only `battle_pet_breeds` and `battle_pet_quality` pet-battle tables. These associate species with breeds/quality, not trainers with ordered teams. Hotfix dump contains abilities, species, breed quality/state and species state/localizations; no NPC team table.
- World dump has Julia Stevens **64330** and Dagra the Fierce **66135**, both with empty ScriptName. Neither has `smart_scripts` rows in this dump. Creature identity alone does not establish opponent pets.
- Existing read-only `world.db` TDB reference tables contain **239 Type=11 objectives across 186 quests**. Example: objectives **268521/268913** target Julia for quests **31316/31693**; **269049/269055** target Dagra for **31813/31819**. One-time quests 31316/31813 have `RewardItem1=89125`, `RewardAmount1=1`; quests 31693/31819 have no first item reward. Do not copy one-time rewards onto repeatable battles. RewardMoney/XP difficulty fields are scaling inputs, not currency amounts.

Durable full census, source row samples, objective/reward records and extracted DB2 SHA-256: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/pbtrainers-20261010/local-data-census.json`. Raw sources remain under game-server `data/reference/tdb1210/`; no canonical database mutation or data commit.

## Integration boundary

The [turn library](../systems/pet-battle-engine.md) already represents three pets per team. Existing wild snapshots transmit vector teams, active slots, health, abilities, `can_trap` and terminal outcomes. No mandatory new battle wire field has been demonstrated for the requested basic PvE encounter. Gossip can provide the entry point server-side, but currently has only submenu/role/close actions.

Do not route trainers through the unchanged wild encounter wrapper: `Encounter::start` synthesizes one Common/breed3 enemy, `apply_rewards` uses a wild XP multiplier and first enemy level, and a win despawns the target creature. These are not authenticated trainer policies. Trainer teams need authored ordered pets, levels, breed/quality, ability selections and any encounter-specific states before runtime admission.

Quest runtime currently loads Wrath `RequiredNpcOrGo` objectives as Monster/GameObject in `quest_data.rs`; `ObjectiveKind` has no pet-tamer kind. `quest_rules::credit_kill` credits Monster objectives only. A trainer victory must not masquerade as an NPC kill. Local TDB Type=11 records must remain distinct when imported/loaded. Daily eligibility, objective credit, reward turn-in and persistence need end-to-end tests once an authentic encounter can be started; quest-row presence alone proves none of those behaviors.

## Result and next step

No runtime changes, tables or importer added. No gameplay/behavioral PASS claimed. Recover at least one ordered trainer team from an allowed local source before implementation; retain unavailable encounters as explicit gaps. Then add its producer/importer and isolated import test, never write canonical world.db from an agent. Cover real gossip start, team construction, all-enemy victory, loss/forfeit/disconnect, no trap/no trainer despawn, persisted rewards and the authored daily objective/reset boundary. Engine/protocol work, if any, belongs in the handoff rather than another slot.

## Sources

- Local TDB1210 archive world/hotfix dumps and read-only `tdb_quest_objectives` / `tdb_quest_template`.
- Local community-listfile FDID mappings and actual extracted WDC5 header; census artifact above.
- `crates/server/src/wild_pet_battles.rs`, `npc_interaction/gossip.rs`, `quest_data.rs`, `quest_rules.rs` at base `125a0bc`.

## See Also

- [Pet battle engine](../systems/pet-battle-engine.md) — existing deterministic turn and wild runtime boundaries.

# Gathering

Verified: 2026-10-11. Gathering is present on master; native Classic baseline proof is scoped below. The [contract](../../specs/gathering.md) owns requirements and lead-owned import instructions. Existing profession, cast, loot and spawn-pool systems carry this loop; no parallel crafting subsystem.

## Representation and wiring

- `GatherableResource` has one field, `parent_skill`, on authored GO50 resources and fully looted skinnable corpses. Existing `UseGameObject` routes to `gathering::use_resource` before service interaction.
- Gathering metadata loads through `world_data::startup_load_world_data`. Node templates use authored locks/loot/trivial thresholds/depletion limits. Skin mappings use creature difficulty `SkinLootID`, `ContentTuning.ExpansionID` and `SkillLine.ParentTierIndex-4`. Skinning casts parent ability8613, not learning-only child265855.
- Cast target validation admits the authored GO/dead corpse while retaining normal caster/cost/movement/cooldown handling. Completion revalidates before loot/skill mutation; unclaimed player item-loot blocks another gather.
- Existing `LootTables` reference/group/chance/count processing rolls GO and skin templates. Existing personal item-loot window/store handles collection. `Learner::gather_skill_up` follows pinned TC thresholds/default chances and existing changed-state persistence/snapshots.
- GO pool selections initialize with creature pools; pooled node respawn re-picks the authored group, including child-pool changes. Depletion retains the shared node for Data6 delay; positive authored spawn timers schedule respawn. Missing ability/timer mappings report gaps, not fabricated resources.

## Instance-entry pool ordering

Wailing Caverns43 entry on private UDP5896 reproduced a panic at `gathering/mod.rs:104` while processing real `WorldPortAck`. `spawn_map_content` queued game-object gathering nodes before creature spawning queued that copy's `PoolSpawns` state. Continental startup already used the opposite order. A follow-up reproduced the same panic during `SelectCharacter` for a character saved inside43: login had its own reversed producer order. Login and world-port now share `queue_map_content`, queuing creature/pool initialization before game objects. Pool selection still initializes when a map has no NPC population index. The node invariant remains strict, with no assertion softening or alternate selection path. The real-data transfer regression creates two independent copies; the persisted-character login regression reopens redb into fresh server apps, with and without an NPC index. Both compare actual spawned node GUIDs with the copy's authored pool selection. Teleport/summon arrivals and reset/recreated copies use world-port; battleground entry uses its same content producer. Continental startup explicitly orders game objects after creature/pool initialization.

## Corpse loot readiness

Native `forever-skin-a23` reproduced a dead, correctly targeted wolf refusing Skinning because normal loot remained, while its prematurely published resource marker prevented the normal loot interaction. Death and deferred loot generation are separate boundaries: absence of `CorpseLoot` does not mean an empty completed roll. The loot producer now retains an empty `CorpseLoot` for zero-drop rolls, and the Skinning publisher requires that completed roll before checking empty slots. After integrating group loot, one authoritative empty-loot predicate also rejects pending rolls and untaken shared slots. The regression exercises unrolled, pending and completed-empty states; the loot decay test retains observable half-decay and refusal of an empty loot window.

## Proof boundary

| Capability | Proof |
|---|---|
| Herb/ore models load; real casts, loot and skill progression | Behavioral real-DB tests |
| Missing skill, movement interruption, pending loot, repeat collection | Behavioral tests |
| Authored depletion/respawn position and pool selection/limits | Virtual-time and authored-pool behavioral tests |
| Skill persists after redb close/reopen | Real persistence boundary test |
| Both-skin native tracking/gathering/kill/skill-up PNGs | [Integrated native proof ledger](../../../../game-engine/docs/wiki/systems/gathering.md#inspected-native-baseline), six passing real-realm cases |

The original ten gathering tests passed at fcf9eea; after integration, twelve gathering tests and six game-object tests pass (one pre-existing imported-copy test remains ignored). Both checks/format and native baseline proof are recorded in the linked engine ledger. Importer two tests passed. Logs are in canonical engine `data/diagnostics/gathering-nodes-2026-10-11/`. Lead imported1996 lock records/3149 mapped skin rows and repaired60 GO50 loot namespaces. Eight skin item gaps (25699/52977 absent from ItemSparse) remain. Node176116/176142/176143 lack local gather abilities; PlayerCondition handling and nonpositive respawn timers remain explicit limits. Independent verification remains parent-owned; the earlier Claude OAuth failure is historical, not a passing gate.

## Sources

- [Metadata importer](../../../scripts/gathering_metadata.py) — local Lock FDID1343608/layoutDA418152, TDB SQL parser and existing loot mapper.
- [Gathering runtime](../../../crates/server/src/gathering/mod.rs) and [data](../../../crates/server/src/gathering/data.rs) — authority, casts, loot and lifecycle.
- [Progress rules](../../../crates/server/src/profession/gathering.rs) — TC a352b1fa UpdateGatherSkill/EffectSkinning and worldserver defaults; source copies retained with diagnostics.
- [Behavioral tests](../../../crates/server/src/gathering/tests.rs) — concrete source records and persistence/time boundaries.

## See Also

- [[professions]] — learning and snapshots.
- [[game-objects]] — model boxes and scene spawning.
- [Loot contract](../../specs/loot-frame.md) — shared rolling and inventory collection.
- [Spawn pools](../../../crates/server/src/spawn_pools.rs) — authored selection lifecycle.

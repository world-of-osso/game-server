# Talents (Trait Trees)

Retail talents are DB2 trait trees, one per class. `spell_info::trait_data` loads each class tree into `ClassInfo.traits`. `trait_config` validates a config for a (class, spec, level); it is a port of TrinityCore `TraitMgr` (`ValidateConfig`, `MeetsTraitCondition`, `GetGrantedTraitEntriesForConfig`). [[class-progression]] keeps the active spec's config in `TraitLoadout`, persists purchases per spec and adds talent spells to `KnownSpells`. The data is Retail 12.1.0.69933 (`scripts/db2.py`).

## Data model

| Piece | DB2 source | Loaded as |
|---|---|---|
| Tree per class | `skill_line_x_trait_tree` (class skill line → `TraitTreeID`; Paladin 800 → 790) | `TraitTree.tree_id` |
| Nodes | `trait_node` (`PosX`/`PosY` for the UI, `Type`, `Flags`, `TraitSubTreeID`) | `TraitNode` sorted by ID |
| Node types | `Type` 0 Single, 1 Tiered (apex), 2 Selection (choice), 3 SubTreeSelection (hero tree choice) | `TraitNodeType` |
| Entries | `trait_node_x_trait_node_entry` (`_Index` order) → `trait_node_entry` (`MaxRanks`, `NodeEntryType`, `TraitSubTreeID`) → `trait_definition.SpellID` | `TraitNodeEntry` (+ `passive` from `spell_misc.Attributes_0 & 0x40`) |
| Prerequisites | `trait_edge` Left → Right. Class trees use only `Type` 2 SufficientForAvailability; `Type` 3 RequiredForAvailability is supported, and any other type fails loading | `TraitNode.parents` |
| Groups | `trait_node_group`, `trait_node_group_x_trait_node` | `TraitNodeGroup`, `TraitNode.groups` |
| Conditions | `trait_cond` linked by `trait_node_x_trait_cond`, `trait_node_group_x_trait_cond`, `trait_node_entry_x_trait_cond`; `SpecSetID` resolved via `spec_set_member` | `TraitCond` |
| Costs | `trait_cost` linked by `trait_node_group_x_trait_cost`, `trait_node_x_trait_cost`, `trait_node_entry_x_trait_cost` | `TraitCost` |
| Currencies | `trait_tree_x_trait_currency` (`_Index` order) + `trait_currency_source` | `TraitCurrency` with `sources` |
| Hero trees | `trait_sub_tree` (Paladin: 48 Templar, 49 Lightsmith, 50 Herald of the Sun) | `TraitSubTree` |

The last seven link tables were added to the DB2 import for this system.

Condition types (`TraitCond.CondType`): 0 Available, 1 Visible, 2 Granted, 3 Increased, 4 DisplayError (UI text only), 5 RanksAllowed. The flags are 0x1 IsGate and 0x4 IsSufficient. 0x2 IsAlwaysMet is ignored, as in TrinityCore.

### Paladin tree 790, observed

- Node groups carry spec rules:
  - Visible conditions with IsSufficient: group 8477 is SpecSet 28 = {66 Protection}, 8484 is SpecSet 27 = {65 Holy}, 8489 is SpecSet 29 = {70 Retribution}.
  - Granted conditions with GrantedRanks 1: group 8479 for Holy, 8486 for Retribution, 8488 for Protection.
- Gates are Available conditions with IsGate on the gated group:
  - Group 8478 needs 8 spec points (cond 20152, currency 2800).
  - Group 8481 needs 20 spec points (cond 48134).
  - Group 8483 needs 8 class points and 8482 needs 23 class points (conds 20155, 20153; currency 2801).
- Costs are group costs: group 8480 costs 1 of 2801 (class nodes) and 8487 costs 1 of 2800 (spec nodes). The hero groups cost their tree's own currency: 10538 costs 2986 (Templar), 10539 costs 2987 (Lightsmith), 10537 costs 2988 (Herald).
- Hero trees:
  - The three SubTreeSelection nodes 99837/99838/99839 are in group 10649, whose Visible condition 27046 needs level 71. Each node is also Visible for one spec: 99837 (Templar 48 / Herald 50) is for Retribution.
  - Group 10536 grants the hero keystones from level 71 (cond 26710).
- Apex nodes 110417–110419 are Tiered, with entries of 1, 2 and 1 ranks. Available cond 49368 needs level 81. RanksAllowed conds need level 81 for rank ≥1, 84 for ≥3 and 90 for ≥4.

## Points per level

`TraitCurrencySource` rows are summed for each tree currency. A row counts when its `PlayerLevel` ≤ level, it has no quest and no achievement, and its `TraitNodeEntryID` (if any) is in the config. Every source row for these currencies has `Amount` 1, except the quest rows, which have `Amount` 0. All ten class trees use the same five currencies, and all of them are `Type` 2 TraitSourced.

| Currency | Flags | Levels granting 1 point | At 80 | At 90 |
|---|---|---|---|---|
| 2801 class | 0x4 class icon | even 10–70, then 82, 85, 88 (rows 29922…) | 31 | 34 |
| 2800 spec | 0x8 spec icon | odd 11–69, then 81, 84, 87, 90 (rows 29892…) | 30 | 34 |
| 2986 / 2987 / 2988 hero | 0 | 71–80, then 83, 86, 89 (rows 34498…34527) | 10 each | 13 each |

Each hero currency is spent only by its own sub-tree's nodes, so a player effectively gets the points of one hero tree. The test `points_per_level_follow_currency_sources` checks levels 9, 11, 80 and 90.

## Validation (`build_config`)

A config is the purchased `(node, entry, ranks)` list merged with the granted entries. Validation fails on the first error and returns a concrete reason:

1. **Granted entries** are computed from an empty config. Class trees' Granted conditions only test spec set (36 of 47) and level (15 of 47). They have no currency or node references (checked over all ten trees). Granted ranks are capped at `MaxRanks` and cost nothing.
2. **Membership:**
   - An unknown node gives `node N is not in trait tree T`.
   - An entry that is not on the node gives `entry E does not belong to node N`.
   - A duplicate purchase gives `... is listed twice`.
3. **Ranks:** purchased plus granted ≤ `MaxRanks`.
4. **Choice nodes** (Selection and SubTreeSelection) need exactly one entry: `choice node N must have exactly one entry, got K`.
5. **Conditions:** Visible, then Available, then RanksAllowed are checked over the node's, each group's and the entry's condition lists.
   - A list passes when any met condition has IsSufficient, or when none of its conditions fail.
   - A condition fails on quest or achievement requirements (there are no such systems yet, and class trees have none). It also fails when the spec is not in the SpecSet, or the level is below `RequiredLevel`.
   - **Gates** (a currency condition that names a group, node or entry): currency is spent in buckets keyed by the spending node's own highest gate requirement. The condition needs the sum of buckets with a gate below `SpentAmountRequired` to reach it. In other words, the points must be spent in rows above the gate.
   - A condition without a currency but naming a group, node or entry compares bought ranks there.
   - RanksAllowed applies only once the node or group holds ≥ `GrantedRanks` bought ranks.
6. **Parents:** a node with parents needs at least one fully ranked parent. A Selection parent counts as full when any one of its entries is maxed; any other parent needs every entry maxed. A RequiredForAvailability parent must itself be full. The error is `node N requires one of nodes [..] fully ranked`.
7. **Hero trees:** at most one SubTreeSelection pick. Bought ranks in a sub-tree node need that sub-tree to be the selected one: `node N belongs to hero talent tree S, which is not selected`. A granted keystone of an unselected tree stays in the config, but its spell does not apply (TrinityCore `CanApplyTraitNode`).
8. **Budget:** spent per currency is the group, entry and node costs times bought ranks. It must not exceed the owned amount: `config spends X points of currency C, level L has Y`.

The config does not need to spend all its points.

## Starter Build

Contract: [Starter Build](../../specs/talent-starter-build.md). `trait_data` loads populated `trait_tree_loadout` rows per tree/spec, with entries in `OrderIndex` order. Pinned TrinityCore a352b1fa `TraitMgr.cpp:1038–1096` supplies grant initialization, entry-zero resolution, cumulative `NumPoints`, and ordered affordable rows without blocking the other currency. Each candidate also passes normal trait-purchase eligibility before being adopted: installed Arms loadout rows reach apex110407 at OrderIndex56 while spec currency is still affordable, before its level81 availability condition49327. Its group9029 costs one spec point per rank. Currency-only construction therefore failed at level70; eligibility defers this row until its authored level instead of invalidating the whole Starter build. Ordinary level/rank/gate/parent/budget rules remain authoritative. No hero choices are invented when authored rows omit them.

The built-in selection uses `STARTER_BUILD_LOADOUT_ID` (`u32::MAX`, outside allocatable named IDs); purchased ranks live in the existing unnamed-default field, preserving redb encoding. It is not inserted in named config rows. Snapshots explicitly advertise `starter_available`. Login, spec activation and level changes rebuild an active Starter allocation against current budgets. Accepted manual Apply changes selection to default; named create/activate selects that named config. Failed requests do not persist mutations.

Source receipt: engine `data/diagnostics/talent-import-export-2026-10-15/source/receipt.json`; a352b1fa `src/server/game/Spells/TraitMgr.cpp` SHA256 `b1477eb159576a833ccc0779903585a2399d4f454e86155b7883da95209fa3e9`. Installed Retail `Blizzard_ClassTalentsFrame.lua:475–531,793–803,1084–1117,1171–1217` grounds availability, built-in/noneditable selection and deviation handling. Native presentation contract remains in the [engine talent-loadout spec](../../../../game-engine/docs/specs/talent-loadouts.md).

Verified2026-10-15: [scoped acceptance](../../specs/talent-starter-build.md#proof--2026-10-15) covers installed-class level legality, exact Arms apex gates, named isolation/rejection/spec persistence and actual private both-skin level-up/relog/deactivation. No reference-data or default-realm edits.

## Persistence and messages

- **Storage (2026-10-10):** `character_trait_loadouts_v1` stores per-spec unnamed defaults, selected IDs and named purchase lists, plus a character-wide monotonically increasing ID counter. Only purchased ranks are stored. On first load, legacy `character_traits` single allocations migrate to each spec's default and persist in the new table; subsequent reads never consult the old row. Both rows are removed with the character.
- **Named loadouts:** owner `TraitLoadoutRequest` creates from a validated full allocation, renames, deletes or activates a config of the active specialization. Apply updates the selected config. Delete removes the saved copy while retaining its current allocation as the unnamed default if selected. Wrong-spec/unknown-ID/invalid-rank mutations change nothing. Activation validates the saved allocation at the current level. `TraitLoadoutsSnapshot` publishes selected ID (zero = default), IDs and names after login, spec/level allocation changes and accepted mutations. Commit/lifecycle typed buffers are sorted by TalentChannel message ID before processing. Rename/delete publish only result/list metadata, never an allocation snapshot that would discard client pending edits. Create/activate/Apply publish the replacement allocation. A real-message regression covers rename and active/inactive deletion without talent/spell mutation.
- **Retail grounding:** local Mainline `Blizzard_ClassTalentsFrame.lua:493-533,577-621` supplies creation/current-allocation copying and auto-apply selection; `Blizzard_ClassTalentLoadoutEditDialog.lua:35-74` supplies rename and confirmed deletion; NameControl XML limits names to30 characters. Native import/export remain gated by the separately documented client serialization-version prerequisite; no guessed format version is emitted.
- **Protocol ranks:** `TraitEntrySelection.rank` is the total (granted plus bought) in both directions. On commit, the server subtracts the granted ranks.
- **`CommitTraitConfig`:**
  - It must name the active spec and validate at the current level.
  - On success the server persists the config and replies `TraitCommitResult { ok: true }`, then `SpellsUnlearned`/`SpellsLearned` deltas, then `TraitConfigSnapshot`.
  - On failure the reply is `ok: false` with the reason, and nothing changes.
- **`TraitConfigSnapshot`:** `spec_id`, `tree_id`, entries including granted ones, and `unspent` per tree currency in `TraitTreeXTraitCurrency` order. It is sent:
  - on enter world (when the class has spec data)
  - on spec change, which loads that spec's saved purchases
  - on a level change that alters the config or budget
  - after every accepted commit
- **Level changes:** a level change revalidates the saved purchases. If they become invalid (for example after an admin level drop), the spec's config resets to granted-only and the empty purchase list is saved.

## Known spells and integration points

- `KnownSpells` = class learnables, then spec spells, then mastery, then talent spells (without duplicates).
- A talent spell is the `TraitDefinition.SpellID` of every entry with ranks whose node is outside hero trees or in the selected one. Active and passive spells both count.
- A Retribution level 80 empty config already knows its granted Hammer of Wrath 1241288, Auras of the Resolute 385633 and Greater Judgment 231663.
- `TraitLoadout::active_talent_spells()` yields `KnownSpell { spell_id, passive }`. It is the hook for the aura runtime to apply passive talent auras.
- `TraitDefinition.OverridesSpellID` of an applied talent removes the baseline spell from `KnownSpells` (`TraitLoadout.overrides`), and bar buttons follow ([[class-progression]]). Paladin examples: Final Verdict 383328 over Templar's Verdict 85256, Hammer of the Righteous 53595, Blessed Hammer 204019, Holy Shock 20473 and Crusading Strikes 404542 over Crusader Strike 35395.
- Not handled yet:
  - `specialization_spells.OverridesSpellID` (for example Holy Judgment 275773 over Judgment 20271): both spells stay known.
  - `TraitDefinitionEffectPoints` is not imported.

## Retired

- The hardcoded 28-talent Paladin `talent.rs` is gone, along with its `QueryTalents` / `ApplyTalentChoice` / `ResetTalents` handlers.
- The protocol messages stay in shared-protocol because the game-engine talent frame (`src/talent.rs`, `game/networking/messages.rs`) still compiles against them. The server no longer answers them.
- `InspectSnapshot.talents` (legacy `TalentSnapshot`) is sent empty until the protocol has a trait inspect message.
- The server no longer inserts `SelectedTalents`.

## Sources

- [trait_data.rs](../../../crates/server/src/spell_info/trait_data.rs) — model and loader
- [trait_config.rs](../../../crates/server/src/trait_config.rs) — granted entries, budgets, validation
- [trait_config_tests.rs](../../../crates/server/src/trait_config_tests.rs) — Paladin tree 790 real-DB tests (node IDs cited)
- [class_progression.rs](../../../crates/server/src/class_progression.rs) — `TraitLoadout`, commit/snapshot systems
- [persistence_progression.rs](../../../crates/server/src/persistence_progression.rs) — `character_traits`
- TrinityCore `src/server/game/Spells/TraitMgr.cpp` and `DataStores/DBCEnums.h` (master) — rule reference

## See Also

- [[class-progression]] — spec resolution and known spells that include talent spells
- [[spell-info]] — `ClassSpells` / `ClassInfo.traits`
- [[persistence]] — redb tables
- [[world-data]] — DB2 import

# Native dungeon and achievement client contract

Server/protocol contract for the native client; the original phase-2 insertion notes below are historical. [Spec](specs/dungeon-achievements.md), [runtime/provenance](wiki/systems/dungeon-achievements.md).

## Dungeon objective block

`DungeonProgress` is a complete replacement snapshot on reliable ordered `InstanceChannel`. Identity: `map_id`, `instance_id`, `difficulty_id`. `encounters` is in DungeonEncounter `OrderIndex`, then ID order. Each row carries `encounter_id`, localized `name`, `defeated`, `optional: Option<bool>` and raw DB2 `flags`. `None` means optionality is unknown, not mandatory. The pinned DB2 has `Flags`, not a separately named optional column; no unverified bit is interpreted as optional. enUS names come from the local import. No per-session locale selection is implemented.

Entry/reconnect broadcasts the current snapshot to players in that copy. Each successful scripted encounter defeat broadcasts after the saved encounter mask changes. No duplicate-DONE update. Entering a continent sends an empty encounter list, clearing the block; manual reset requires an empty copy, and its next entry sends the fresh list.

Client insertion points (not edited here): add `DungeonProgress`, `AchievementCatalogPage` and existing `AchievementStateUpdate` subscriptions to `godot/network/src/lib.rs:184` (`NetworkBridge::connect`). Add account-state classification/dispatch alongside quests in `godot/rust/src/account.rs:1072-1083`, mirror snapshots into account UI state, and have `godot/rust/src/objective_tracker.rs:60` (`objective_tracker_view`) read that state. Extend `godot/ui-model/src/ui/screens/objective_tracker_component.rs:84` (`ObjectiveTrackerState`) with a separate dungeon block, leaving watched quest order unchanged. Clear on world transfer/disconnect, reject snapshots for a different active copy, then replace by identity. Render names in received order, check defeated rows, and label optional only for `Some(true)`.

Retail source root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:
- `Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua:202` reads stage metadata; `:379-416` reads `C_ScenarioInfo.GetCriteriaInfo`, description, quantities and completion, rendering checks/nubs.
- `Blizzard_APIDocumentationGenerated/ScenarioInfoDocumentation.lua:11-25` declares indexed criteria lookup; `:263-281` describes criterion fields. This is a dungeon block modeled after Retail criteria rendering, not an implementation of every scenario API.

## Achievement frame

`QueryAchievementCatalog` uses reliable ordered `AchievementChannel`. Request on opening/browsing, full-catalog search, selected own-state links or restored/tracked IDs; no bulk achievement catalog is sent on login.

| Request variant | Response variant | Fields |
|---|---|---|
| `Summary { scope }` | `Summary` | authoritative points/counts, category ancestry totals, newest three earned rows |
| `Achievement { achievement_id }` | `Achievement` | correlated optional own-state row; never comparison data |
| `Statistics { category_id, after_id }` | `Statistics` | counter rows with optional raw `statistic_value`, `next_id` |
| `Search { text, scope, after_id }` | `Search` | correlated term/scope, matching rows, `next_id` |
| `Categories { after_id }` | `Categories` | `categories`, `next_id` |
| `Category { category_id, after_id }` | `Category` | echoed category, `achievements`, `next_id` |
| `Criteria { achievement_id, after_id }` | `Criteria` | echoed achievement, `criteria`, `next_id` |

Start `after_id=0`; use the returned `next_id` unchanged for the next page. `None` ends pagination. IDs are exclusive ascending cursors, not offsets. Unknown category/achievement and exhausted cursors return empty terminal pages. Category rows: `category_id`, signed `parent_id` (-1 root), `order_index`, `name`; collect pages and display by authored order under the parent.

Achievement rows: `achievement_id`, `name`, `description`, `points`, `icon_fdid`, raw `flags`, `earned`, `earned_at` (optional Unix seconds UTC), `progress_supported`, first `criteria` page, `next_criteria_id` and optional `statistic_value`. `earned_at=None` is a genuinely unknown historical date, not zero/today. Newly earned dates persist with the character. `progress_supported=false` means catalog metadata is available but phase-1 evaluation cannot grant that achievement; do not treat it as completed or claim Retail evaluation parity.

All/Account/Character scope uses authored flag0x20000. Summary and ordinary category browsing exclude counters, hidden/guild rows and hidden tracking copies; search includes statistics but excludes non-player rows. Statistics quantities are unclamped supported counters; None remains unevaluated, not fabricated zero. Search matches title/description over the full faction/scope-eligible catalog, not loaded pages. The shared native limit is256 characters; replies preserve valid Unicode terms so the client can correlate them. Selecting a result queries authoritative own-state details.

Earned payloads include authentic `Achievement.IconFileID`; the client must not guess an icon from an unrelated asset. Watch persistence stores IDs only; restored/watch refresh requests use `Achievement`, including while the window is closed. No authoritative comparison endpoint exists; InspectSnapshot exposes equipment/talents only.

Criterion rows: `tree_id` (cursor/row identity), `order_index`, `criteria_id`, localized `description`, `current`, `required` (u64), `completed`, `progress_supported`. Request additional criteria with `next_criteria_id`. Keep unsupported lines visibly unevaluated; no synthetic names/counters. Existing `AchievementStateUpdate` remains the live toast/progress signal; re-request the visible category when it changes so its dates and criterion lines stay current. The native client uses a dedicated achievement runtime/frame, not quest-log data. That frame should read the account's cached category/achievement/criteria pages, project them into its SharedContext screen state, and send requests through `NetworkBridge::send` (`godot/network/src/lib.rs:303`) on AchievementChannel. Current client behavior/proof is owned by `game-engine/docs/wiki/systems/dungeon-achievements.md`. Client and server must build against the same updated protocol; the registry fingerprint changes.

Retail sources:
- `Blizzard_AchievementUI/Mainline/Blizzard_AchievementUI.lua:572` builds category names/parents; `:1814` reads achievement name/points/date/description/icon; `:1952` reads criterion text/completion/current/required; `:2728` requests a category/index row.
- `Blizzard_APIDocumentationGenerated/AchievementInfoDocumentation.lua:5` names `C_AchievementInfo`; `:64-78` defines validity lookup. Category/achievement/criterion list APIs used by this pinned UI are globals (`GetCategoryInfo`, `GetAchievementInfo`, `GetAchievementCriteriaInfo`), not falsely attributed to that namespace.

## Criteria coverage — 2026-10-11

Pinned census: `scripts/achievement_census.py --db2 <69933-dir> --modifiers <ModifierTree.csv> --world-db <world.db> --output <report.json>`. SQLite is opened read-only. Every Achievement row records descendant CriteriaTree/Criteria IDs, criterion types, referenced ModifierTree IDs/types and explicit blockers. Counts are structural all-faction unions, not playable-content or full Retail evaluation certification.

Type12 is CurrencyGained (not zone quests). TrinityCore a352b1fa CriteriaHandler.cpp599-600 accumulates awarded quantity; capped requests credit only the actual wallet increase, after the currency persistence boundary. Spending, failed awards and already-owned login balances produce no gain credit. Account contributions add earned quantities across characters; statistics retain raw quantities. Currency registry/content limitations remain independent of structural criteria support. Type11 is CompleteQuestsInZone.

Baseline3388 trackable achievements (2338 account/1050 character),105 statistics. Batch12 census3489 (+101),143 statistics. Batch46 adds positive signed reputation load/change credit, with traditional Retail thresholds and authentic435-faction race/class baselines in the shared protocol. Old reputation saves migrate once to `reputations_retail_v2` by preserving recoverable per-race deltas; old clamped history is unrecoverable. Preserve earned IDs/dates; purge previously unsupported unsigned type46 quantities and rebuild from signed state. Unmigrated sibling-character quantities never enter account aggregation. Conditional/modifier/timer/world-state restrictions remain unevaluated.

Type36 AcquireItem follows TC Player.cpp11336–11340 StoreNewItem: committed new loot/group-loot awards, quest provided/reward items, crafting creates/disenchant material loot and standard/economic merchant purchases accumulate actual stack quantities. Transfers of an existing GUID (bank, mail, trade, buyback) are not new-item creation and do not receive this credit. Admin setup grants are not gameplay proof. New-item gameplay paths publish to the existing inventory update queue after their commits; achievement processing persists character/account progress on the following tick.

Type158 WinPetBattle admits the unconditional Asset0 criterion19724 only. Pinned trees define tenth win6594, fiftieth6593,250th6462,1000th6591 and5000th6592. Actual wild/PvP terminal winners receive one credit; losers, draws and forfeiting players do not. Conditional/streak criteria21526/21527 stay unevaluated. TC a352b1fa DBCEnums.h707 names the type but CriteriaHandler marks it NYI; quantity semantics here derive from authenticated DB2 descriptions/amounts and authoritative native battle outcomes, not a claimed TC implementation.

Type216 MythicPlusCompleted admits compiled authentic all-of ModifierTree predicates only: keystone minimum247, in-time248, challenge-map249 and exact player level39. Regenerate with `scripts/import_achievement_modifiers.py --criteria <Criteria.csv> --modifiers <ModifierTree.csv> --output crates/server-catalog/src/achievement/mythic_predicates.rs`; output records source hashes and checks both criterion and modifier-root IDs. Native settlement credits current authenticated copy members only after committing the run; settled run IDs do not credit twice. Real Azure Vault16645/56012 requires challenge401, key≥10, in time and player level exactly70. Seasonal250, world-state67, timerunning386, nested unknown conditions and other operators remain unsupported; active season/rating are not invented. TC names247–250 but marks their implementation NYI; these predicates use pinned DB2 requirements and native authoritative completion inputs.

Type34 LearnOrKnowSpell uses actual KnownSpells load/change generations, covering class/spec/level initialization and committed trainer/admin/spell-effect learning published by ClassProgression. Local spell assets must exist; quantities are boolean1, not cast/learn repetitions. Account aggregation takes the max of this fact. Seeded mount/pet collection lists do not stand in for HasSpell. Real42426→32298 learning credits6155; rejected trigger depth and repeats do not add quantities (TC CriteriaHandler.cpp687/1530–1535).

Guild-flag0x4000 definitions are excluded from player/account grants and snapshots, not only from window browsing. Their criteria-tree structure remains separately counted in the census, but no guild actor/progress is invented. Real5025 "Dust, Dust, and More Dust!" reproduces the former wrong-player snapshot eligibility. Historical earned IDs remain preserved; this does not certify or migrate guild achievement state.

Type74 Login admits unconditional Asset0 only (real23250). Authenticated Added<InWorld> credits one event; subsequent ticks and level updates do not. Removing/re-entering the session gives the next login credit; quantities persist across redb reopen and add across account contributions. TC CriteriaHandler.cpp552/573 accumulates login events; no holiday/timed/modifier-qualified login is inferred.

## Bounds

Fixed pages: 32 categories, 8 achievements, 16 criterion lines. Embedded achievement criteria use the same 16-line page and continuation. Names/criterion text are UTF-8 prefix-capped to 256 bytes; achievement descriptions to 2048 bytes. Category response worst-case text is 8 × (256 + 2048 + 16 × 256) = 51,200 bytes plus bounded numeric/encoding overhead; 56,397 bytes with maximum-width numeric fields in the bincode wire test, below 64 KiB. The pinned build's served criterion labels are at most 181 bytes, so the cap does not cut any currently reachable achievement criterion line. Search additionally carries at most256 Unicode characters (up to1024 UTF-8 bytes); other requests carry enum tags, scope and u32 IDs. Summary category totals cover the finite eligible source catalog and embed at most three earned rows. Dungeon encounters use the 32-bit encounter mask (validated maximum 32 rows per map/difficulty), with names capped to 256 bytes. No client-controlled page sizes or unbounded vectors from a catalog request.

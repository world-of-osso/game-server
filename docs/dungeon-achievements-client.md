# Phase 2 client hookup

Server/protocol contract for the later client change. The game-engine checkout is read-only during this phase. [Spec](specs/dungeon-achievements.md), [runtime/provenance](wiki/systems/dungeon-achievements.md).

## Dungeon objective block

`DungeonProgress` is a complete replacement snapshot on reliable ordered `InstanceChannel`. Identity: `map_id`, `instance_id`, `difficulty_id`. `encounters` is in DungeonEncounter `OrderIndex`, then ID order. Each row carries `encounter_id`, localized `name`, `defeated`, `optional: Option<bool>` and raw DB2 `flags`. `None` means optionality is unknown, not mandatory. The pinned DB2 has `Flags`, not a separately named optional column; no unverified bit is interpreted as optional. enUS names come from the local import. No per-session locale selection is implemented.

Entry/reconnect broadcasts the current snapshot to players in that copy. Each successful scripted encounter defeat broadcasts after the saved encounter mask changes. No duplicate-DONE update. Entering a continent sends an empty encounter list, clearing the block; manual reset requires an empty copy, and its next entry sends the fresh list.

Client insertion points (not edited here): receive these messages in `godot/network`, mirror snapshots into account/session UI state, and have `godot/rust/src/objective_tracker.rs:60` (`objective_tracker_view`) read that state. Extend `godot/ui-model/src/ui/screens/objective_tracker_component.rs:84` (`ObjectiveTrackerState`) with a separate dungeon block, leaving watched quest order unchanged. Clear on world transfer/disconnect, reject snapshots for a different active copy, then replace by identity. Render names in received order, check defeated rows, and label optional only for `Some(true)`.

Retail source root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:
- `Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua:202` reads stage metadata; `:379-416` reads `C_ScenarioInfo.GetCriteriaInfo`, description, quantities and completion, rendering checks/nubs.
- `Blizzard_APIDocumentationGenerated/ScenarioInfoDocumentation.lua:11-25` declares indexed criteria lookup; `:263-281` describes criterion fields. This is a dungeon block modeled after Retail criteria rendering, not an implementation of every scenario API.

## Achievement frame

`QueryAchievementCatalog` uses reliable ordered `AchievementChannel`. Request only when opening the achievement frame or selecting a category; no catalog is sent on login.

| Request variant | Response variant | Fields |
|---|---|---|
| `Categories { after_id }` | `Categories` | `categories`, `next_id` |
| `Category { category_id, after_id }` | `Category` | echoed category, `achievements`, `next_id` |
| `Criteria { achievement_id, after_id }` | `Criteria` | echoed achievement, `criteria`, `next_id` |

Start `after_id=0`; use the returned `next_id` unchanged for the next page. `None` ends pagination. IDs are exclusive ascending cursors, not offsets. Unknown category/achievement and exhausted cursors return empty terminal pages. Category rows: `category_id`, signed `parent_id` (-1 root), `order_index`, `name`; collect pages and display by authored order under the parent.

Achievement rows: `achievement_id`, `name`, `description`, `points`, `icon_fdid`, `earned`, `earned_at` (optional Unix seconds UTC), `progress_supported`, first `criteria` page and `next_criteria_id`. `earned_at=None` is a genuinely unknown historical date, not zero/today. Newly earned dates persist with the character. `progress_supported=false` means catalog metadata is available but phase-1 evaluation cannot grant that achievement; do not treat it as completed or claim Retail evaluation parity.

Criterion rows: `tree_id` (cursor/row identity), `order_index`, `criteria_id`, localized `description`, `current`, `required` (u64), `completed`, `progress_supported`. Request additional criteria with `next_criteria_id`. Keep unsupported lines visibly unevaluated; no synthetic names/counters. Existing `AchievementStateUpdate` remains the live toast/progress signal; re-request the visible category when it changes so its dates and criterion lines stay current. The later client needs a new achievement runtime/frame, not a reuse of quest-log data.

Retail sources:
- `Blizzard_AchievementUI/Mainline/Blizzard_AchievementUI.lua:572` builds category names/parents; `:1814` reads achievement name/points/date/description/icon; `:1952` reads criterion text/completion/current/required; `:2728` requests a category/index row.
- `Blizzard_APIDocumentationGenerated/AchievementInfoDocumentation.lua:5` names `C_AchievementInfo`; `:64-78` defines validity lookup. Category/achievement/criterion list APIs used by this pinned UI are globals (`GetCategoryInfo`, `GetAchievementInfo`, `GetAchievementCriteriaInfo`), not falsely attributed to that namespace.

## Bounds

Fixed pages: 32 categories, 8 achievements, 16 criterion lines. Embedded achievement criteria use the same 16-line page and continuation. Names/criterion text are UTF-8 prefix-capped to 256 bytes; achievement descriptions to 2048 bytes. Category response worst-case text is 8 × (256 + 2048 + 16 × 256) = 51,200 bytes plus bounded numeric/encoding overhead; below 64 KiB with the wire encoder. Requests contain only enum tags and u32 IDs. Dungeon encounters use the 32-bit encounter mask (validated maximum 32 rows per map/difficulty), with names capped to 256 bytes. No client-controlled page sizes or unbounded vectors from a catalog request.

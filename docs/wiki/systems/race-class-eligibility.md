# Installed race/class eligibility

Contract: [race/class eligibility](../../specs/race-class-eligibility.md). Source inspected2026-10-15; post-change Rust/native gates remain pending.

## Source and lifetime

`race_class.rs` reads `data/db2/active/ChrRaces.csv` and `CharBaseInfo.csv` once per process. Race metadata provides the race-mask bit; supported class memberships populate ordered class sets. Non-playable metadata and unconverted Adventurer14 do not become offered player pairs. The process retains that snapshot until restart. Deploy already ships the entire active DB2 publication; container test inputs also require both CSVs.

The old Retail permission slice is removed. Missing/invalid CSVs, unsupported mask bits, unknown playable-race references, duplicate race metadata and an empty Retail result fail catalog loading. There is no static Retail permission path after source failure.

The public `validate_race_class`, `playable_pairs` and mask helpers keep their existing interfaces. Creation kits and authentic TDB starts already consume those helpers, so newly admitted pairs enter the same source-backed creation path. Start-level and visage policies are unchanged.

## Separate Forever identity

The [Skyborne95/96 contract](skyborne.md#race-identity-and-creation) remains explicit and source-pinned: ten authored pairs and bits32/33. Installed Retail names those IDs as NPC rows and provides no CharBaseInfo memberships. Empty Retail race entries are removed before the separate Forever entries are added; a playable Retail collision fails rather than overwriting source identity. This is not an alternative lookup after a missing Retail CSV. Raw Forever ChrRaces/CharBaseInfo graph bytes were unavailable in this checkout; no new decode or metadata-completeness claim follows.

## Observed failure and proof boundary

The private bot driver received `Class 7 is not available for race 3` for normal Dwarf/Shaman creation while installed12.1.5.70391 CharBaseInfo explicitly contains3/7. The prior static Dwarf list also omitted installed Mage8 and Warlock9. That real admin rejection is retained in engine `data/diagnostics/bot-harness-2026-10-15/native-source-gear1/263/setup-error.json`; the root-cause change does not alter class stats or bypass creation.

The installed snapshot contains283 distinct supported Retail pairs. New regressions check those actual rows and actual Dwarf/Shaman admin creation, source starting location/equipment and saved character data. Existing mask, class rejection, start-level and Forever tests remain controls. Execution results belong in the proof ledger after they run; added test code alone is not acceptance.

The [bot fixture](../../../../game-engine/docs/wiki/systems/bot-harness.md#native-players-and-source-inputs) separately authors Draenei11 for Shaman before provisioning. That choice does not establish Dwarf eligibility or replace this server fix. The full class/rotation and all-pairs intro/asset gates remain separate.

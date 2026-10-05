# Reconstructed Zephras content

User-approved Wowhead Forever-only reconstruction for Skyborne starts; [implementation](../wiki/systems/content-import.md#Forever-Zephras-overlay).

## Requirements

- [x] Overlay map-2991 NPC templates, terrain-supported mapper observations, captured quest text, starter/ender links, supported objectives and rewards; tag every row with scrape, source URL and `reconstructed` provenance.
- [x] For reconstructed templates missing authored health/class stats, use the approved coarse estimate `unit_class=1`, `HealthModifier=1.0`; retain captured levels/rank/type/family and label estimates, analogue IDs and current class-level health curve in template source JSON. Existing authored inputs and all Retail/non-overlay stats must not be overwritten. Do not infer spellcaster classes or claim authored stats/full Classic combat.
- [x] Actual map2991 startup regression runs with1662 imported spawns and estimated giver health42/102; [server proof](../wiki/systems/content-import.md#forever-zephras-overlay) owns fresh workspace evidence and ignored-case limits.
- [x] Bounded race95 native QuestFrame acceptance of92460 and race96 visible Ventaari observed; [engine evidence](../../../game-engine-skyborn/docs/wiki/systems/forever-data.md#current-scoped-capability-matrix) owns receipts, Main-inspected captures and pending independent artifact gate. Ailee136968 remains withheld; complete NPC visuals/gameplay remain unproven.
- [x] Decode hash-matched local Forever Creature/CreatureXDisplayInfo; report encrypted/missing rows and required client displays without substituting NPC models.
- [x] Derive A/H reactions towards Forever factions 2795/2794 from captured react values. Unknown sides use neutral faction identity with explicit friendly relation; do not invent a nation or enemy.
- [x] Skip unknown Z, invalid/unknown levels and uncaptured quest pages; preserve unsupported objectives in provenance, never convert interactions to kills.
- [x] Reward item IDs require Retail/runtime item rows; spells require Retail spell rows. Preserve captured fixed XP, not a guessed Retail difficulty. Unsupported completion must stay blocked.
- [x] Reapply overlay inside content-policy regeneration after Retail content, atomically; missing inputs fail preflight before destructive regeneration.
- [x] Reject non-overlay primary-key collisions explicitly across content, faction, metadata and provenance tables; preserve original rows on failure and capture removal. Cleanup deletes only rows tagged reconstructed, before rebuilding base content tables so stale tags cannot delete fresh Retail rows.
- [x] Prove real-kit creation/reopen/login on map 2991 and supported start ground. Historical controlled giver/quest message fixtures cover Coming of Age acceptance/40 XP; Skysight keeps source mask 0, rejects level 1, accepts level 2 and blocks unimplemented event completion. These historical message fixtures do not confer rendered-client credit; current startup and bounded native evidence are linked above.
- [x] Use refreshed captured NPC-page appearances (Ailee 136968, Ventaari 139694), keep source masks unchanged, and export the full 235-CDI catalog independently of spawn/level omissions.
- [ ] Client model provisioning and native quest-side/reputation/reward-spell/event gaps remain; no rendered-client or complete-gameplay claim.

## Exclusions

No scripted events/phasing, inferred loot/vendor inventory/gossip, guessed displays or WMO floor positions. Missing pages and client assets remain reported gaps; no live world/player writes during this work.

## Tests

`scripts/tests/test_zephras_creatures.py`, `test_zephras_content.py`; `auth_character_tests.rs`, `zephras_ground_tests.rs`, `quest_tests.rs`; `zones_world_db_tests::zephras_startup_spawns_imported_givers_on_the_hosted_continent` retains the actual imported-row startup regression. Its current1662-spawn and health42/102 assertions have passing workspace evidence; native acceptance remains separate from importer tests. Python tests cover estimated fields/provenance, repeat regeneration and atomic collisions preserving Retail class/health.

# Reconstructed Zephras content

User-approved Wowhead Forever-only reconstruction for Skyborne starts; [implementation](../wiki/systems/content-import.md#Forever-Zephras-overlay).

## Requirements

- [x] Overlay map-2991 NPC templates, terrain-supported mapper observations, captured quest text, starter/ender links, supported objectives and rewards; tag every row with scrape, source URL and `reconstructed` provenance.
- [x] Decode hash-matched local Forever Creature/CreatureXDisplayInfo; report encrypted/missing rows and required client displays without substituting NPC models.
- [x] Derive A/H reactions towards Forever factions 2795/2794 from captured react values. Unknown sides use neutral faction identity with explicit friendly relation; do not invent a nation or enemy.
- [x] Skip unknown Z, invalid/unknown levels and uncaptured quest pages; preserve unsupported objectives in provenance, never convert interactions to kills.
- [x] Reward item IDs require Retail/runtime item rows; spells require Retail spell rows. Preserve captured fixed XP, not a guessed Retail difficulty. Unsupported completion must stay blocked.
- [x] Reapply overlay inside content-policy regeneration after Retail content, atomically; missing inputs fail preflight before destructive regeneration.
- [ ] Prove real-kit creation/reopen/login on map 2991, supported start ground, nearby giver spawn query and Coming of Age acceptance; report Gift of Skysight's level-2 gate and unimplemented completion.

## Exclusions

No scripted events/phasing, inferred loot/vendor inventory/gossip, guessed displays or WMO floor positions. Missing pages and client assets remain reported gaps; no live world/player writes during this work.

## Tests

`scripts/tests/test_zephras_creatures.py`, `test_zephras_content.py`; `auth_character_tests.rs`, `zephras_ground_tests.rs`, `quest_tests.rs`.

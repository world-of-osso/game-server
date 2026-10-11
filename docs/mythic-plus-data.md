# Mythic+ local data qualification

Verified: 2026-10-11. Contract: [Mythic+](specs/mythic-plus.md). This is a catalog foundation, **not an operational keystone loop**.

## Authored candidate

Local Retail CASC build `dcfc90fffd79ba00406ae46f5f657592` / `12.1.0.69933` supplies:

- `MapChallengeMode` FDID801709/layoutAAC13A73: challenge401 **The Azure Vault**, map2515, CriteriaCount2250/1800/1350 seconds. TrinityCore a352b1fa `DB2Structure.h` calls the adjacent field MaxMedals; do not treat thresholds as scores or manufacture a season.
- Canonical read-only population:225 creature spawns, all authored for `1,2,8,23`; Font entry246779/type49 at(-5051.85,1033.09,594.452), difficulties8/23. MapDifficulty includes8. Wailing Caverns/Cobrahn is **not** in this MapChallengeMode catalog; its existing encounter work is not an M+ dungeon definition.
- Local Scenario2133/ScenarioStep5383 → criteria tree138749. Its four children require encounter2582 Leymor,2585 Azureblade,2583 Telash Greywing and2584 Umbrelskul. Enemy Forces tree138754 has target410, operator9 (weighted sum); child creature187160 gives5, for example. Some children use type92 scenario events, not creature kills. Do not count every mob as1 or silently drop event requirements.
- KeystoneAffix FDID1344761/layout166FC575:50 names/descriptions/icon FDIDs. Row147 authors15 seconds lost per death; row165 explicitly disables death time loss. Neither establishes the active affix schedule nor a default penalty for keys without these affixes.
- ChallengeModeItemBonusOverride FDID2965645/layoutC75A0165:761 reward-tree override rows. These are **item bonus mappings**, not rating formulas. MythicPlusSeason FDID2400282/layoutDC94262F gives time-event references, not a ready affix rotation.

Inputs/schema references: locally cached Retail `Blizzard_ChallengesUI/Mainline/Blizzard_ChallengesUI.lua` lines675–746 (slot/start), `Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua` lines774–845 (tracker/time/deaths/affixes), API `ChallengeModeInfoDocumentation.lua` (`GetMapUIInfo`, `GetDeathCount`, `StartChallengeMode`, `GetOverallDungeonScore`); TrinityCore **a352b1fa**, `src/server/game/DataStores/DB2Structure.h` and `Server/Packets/MythicPlusPacketsCommon.h`; WoWDBDefs definitions matched to the six extracted layout hashes. Raw files, schemas and receipts retained in canonical engine `data/diagnostics/mythic-plus-2026-10-11/`.

## Copy-only import

Lead creates an independent SQLite backup at `game-server/data/world.db.mythic-plus` (not a hardlink/symlink). Extracted DB2 payloads already live under canonical engine diagnostics; this command does not fetch network data or touch world.db:

```text
python3 scripts/import_mythic_plus.py --db data/world.db.mythic-plus --source-dir /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/mythic-plus-2026-10-11/local-db2 --report /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/mythic-plus-2026-10-11/import-report.json
```

Lead owns copy review/publication. Tables are replaced atomically; `mythic_plus_import_source` records exact build/layout/FDID/SHA256 and readable-section gaps. Existing population, item and criteria tables are untouched. Text comes from the extracted English sections. Narrow integer normalization uses the matched DBD widths (not the decoder's potentially wider packed storage value).

Targeted tests:

```text
MYTHIC_PLUS_DB2_SOURCE=/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/mythic-plus-2026-10-11/local-db2 MYTHIC_PLUS_WORLD_SOURCE=/home/osso/Projects/world-of-osso/game-server/data/world.db /home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run mythic-plus python3 -W error::ResourceWarning scripts/tests/test_import_mythic_plus.py
```

Use the canonical engine `scripts/agent/agent-run` path when running from game-server.

## Required numeric-rule boundary

Retail Lua receives countdown/timeLost/rating from native APIs; it does not implement their numeric server policy. Pinned TC tree has scenario support and MythicPlus packet structures, but no challenge manager/rating implementation. No authentic current rating formula, countdown duration, default death penalty or active season/affix selection was established in these local inputs. These must remain explicit gaps; a plausible constant or zero-valued invented score would violate the feature contract. Current ordinary content_item also lacks180653/158923 although ItemSparse names both Mythic Keystone; an item import/acquisition/metadata path is still needed.

Local Criteria FDID1263817 extraction reported one unknown encrypted10-byte chunk; canonical criteria already exposes the Azure tree cited above. Record only; do not treat this as a blocker for the proven tree.

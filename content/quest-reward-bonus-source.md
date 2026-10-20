# Quest reward bonus catalog

`quest-reward-bonus.json` contains selected TrinityCore quest IDs and their authored `RewardBonusMoney`, including0. Absence means the content producer did not select that TC quest; it does not authorize borrowing a raw same-ID row for legacy/captured content. Suppression by Quest.Flags0x100 happens in ordinary settlement, not during export.

## Maintenance

The maintained `scripts/retail_content.py` selected-row mapping preserves the bonus as nullable `content_quest_template.RewardBonusMoney`: legacy rows NULL, selected TC rows their exact source value. After committing a content rebuild, `scripts/content_policy.py --quest-reward-bonus <tracked-output>` writes the catalog; its default is this directory. World-promotion exports the resulting whole content metadata too. Rebuild existing content tables with the updated producer before using the new export. The runtime reads the required world.db sibling `quest-reward-bonus.json`; deploy.sh copies this tracked file there. Missing or malformed runtime catalog is an explicit load failure.

## Initial snapshot

Generated2026-10-19 from a private SQLite backup of the current imported world, with canonical mtime unchanged.19152 selected rows; SHA256 `7caf4cdb968dfa4bb131eeee82c1c314a5e3e96fab9cf101d97e4ee4b213d9b6`.

The existing schema predates the new metadata. Initial private preparation retained two established source paths:

1. Nonzero producer-only ContentTuningID, matching raw TC ID/tuning/title exactly. The AC mapper never authors this added field.
2. Existing TC-origin creatures/objects identified by the maintained20000000 GUID offset and matching raw template entry; the maintained Retail quest relation/QuestV2 selection, with matching content tuning/title/flags and TC-mapped QuestLevel−1. This retains selected CT0 quests without using raw same-ID presence as authority.

Legacy1/7/55 stay unselected, even though raw TC1/55 have positive bonuses. Native11/62 retain120;35 retains0;54083 retains46200 even though its suppression flag prevents payment. Full private generation script, SQL witnesses, file hashes and source mtime receipt are in the feature proof packet; no canonical data was modified. Later regeneration uses the maintained explicit nullable metadata, not this one-time reconstruction.

[Quest mechanism and source contract](../docs/wiki/systems/quests.md#authored-max-level-money--2026-10-19).

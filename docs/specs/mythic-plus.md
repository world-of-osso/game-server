# Mythic+ keystone loop

Retail Mainline defines the item/Font/countdown/challenge tracker/completion/rating behavior; Forever only changes art. Local build `12.1.0.69933` DB2s and existing population/criteria data define numeric content. [Source qualification and import](../mythic-plus-data.md).

## What it must do

- [x] Import build/layout-pinned local MapChallengeMode, KeystoneAffix, ChallengeModeItemBonusOverride, MythicPlusSeason, Scenario and ScenarioStep records into an independent SQLite copy, preserving source hashes and gaps.
- [x] Preserve unrelated data, replace the catalog atomically, reject corrupt layouts and refuse canonical/symlink/hardlink destinations.
- [x] Qualify a MapChallengeMode dungeon with existing difficulty8 spawns and an authored Font of Power; selected candidate is The Azure Vault (map2515/challenge401).
- [ ] Keep keystone item identity, challenge ID, level and affixes durable; validate actual bag ownership and Font reach/copy/difficulty before activation.
- [ ] Show Font insertion/activation, countdown, elapsed/remaining time, death count/time lost, affixes and scenario boss/enemy-force criteria in both skins.
- [ ] Complete only after authored boss and force criteria; use source-backed timer thresholds for upgrades, handle failed/abandoned keys without repeat awards.
- [ ] Display authentic rating and persist completed runs without fabricating a score formula.

## How it works

- [Catalog import architecture](../wiki/systems/mythic-plus.md).
- [Local source evidence, copy-import command and numeric-rule boundary](../mythic-plus-data.md).

## Implementation inventory

- `crates/server/src/mythic_plus_runtime.rs` — deterministic lifecycle, authored weighted creature/event criteria, boss completion, threshold upgrades/overtime/abandonment depletion and terminal immutability.
- `crates/server/src/mythic_plus.rs` — durable GUID metadata, authenticated bag-owned Font slots, fresh Mythic23→Keystone8 conversion, shared ServerClock, kill/death/encounter hooks and atomic terminal key/history settlement. `GrantKeystone` is explicit administrative acquisition, not a fabricated dungeon drop rule.
- `scripts/import_keystone_item.py` — private-copy publication of authentic local Item/ItemSparse180653; unrelated items stay untouched.
- `scripts/import_mythic_plus.py` — copy-only, offline challenge catalog importer; does not enable gameplay.
- `scripts/tests/test_import_mythic_plus.py` — authentic local catalog and destination safety regressions.

## Tests asserting this spec

- `scripts/tests/test_import_mythic_plus.py` (requires named extracted local DB2 and read-only world sources).

## Known gaps (current cycle)

- [ ] Both-skin native activation/weighted-kill/encounter/terminal proof is still pending. The runtime exists separately from the offline catalog importer.
- [ ] Season activation/affix rotation, default death penalty, countdown duration and current rating calculation lack an established numeric source. `UnsourcedChallengeConfig` exposes `unsourced_pre_start_countdown_ms`/`unsourced_death_penalty_ms` default0 and `unsourced_seasonal_affix_rotation` and `unsourced_rating_by_level` default empty; `unsourced_season_period_secs`/`unsourced_season_epoch_unix` default0. Period0 disables rotation; a supplied period/epoch indexes explicitly supplied level-specific rotations. No active season/pool is presumed. Rating remains `None`, not a zero score. Configured test numbers prove mechanisms only, not Retail policy. Minimum key floor and next-dungeon selection also remain unsourced; saturating level subtraction currently has no asserted Retail floor.
- [ ] Selected affix mechanics only where authored rules exist; historical affix rows are not a current active rotation.
- [ ] Both-skin private UDP5800 native acceptance, completion/depletion and rating proof remain pending.

## Out of scope

- Canonical world.db mutation, world population reimport, protected UDP5000 realm, master merge, invented combat/force/timer/score values and Forever-only functionality.

# Mythic+ keystone loop

Retail Mainline defines the item/Font/countdown/challenge tracker/completion/rating behavior; Forever only changes art. Local build `12.1.0.69933` DB2s and existing population/criteria data define numeric content. [Source qualification and import](../mythic-plus-data.md).

## What it must do

- [x] Import build/layout-pinned local MapChallengeMode, KeystoneAffix, ChallengeModeItemBonusOverride, MythicPlusSeason, Scenario and ScenarioStep records into an independent SQLite copy, preserving source hashes and gaps.
- [x] Preserve unrelated data, replace the catalog atomically, reject corrupt layouts and refuse canonical/symlink/hardlink destinations.
- [x] Qualify a MapChallengeMode dungeon with existing difficulty8 spawns and an authored Font of Power; selected candidate is The Azure Vault (map2515/challenge401).
- [x] Keep keystone item identity, challenge ID, level and affixes durable; validate actual current-character bag ownership and Font reach/copy/difficulty before activation.
- [ ] Preserve the Mythic entry binding after the copy changes to Keystone difficulty so same-copy transports and re-entry retain the party.
- [x] Show Font insertion/activation, countdown, elapsed/remaining time, death count/time lost, configured affixes and scenario boss/enemy-force criteria in both skins. The cited Midnight Season 2 policy supplies a 10-second countdown and 0/5/15-second death penalties for +2–5/+6–11/+12 and above.
- [x] Complete only after authored boss and force criteria; use 100%, 80% and 60% of the DB2 timer for upgrades, handle failed/abandoned keys without repeat awards. Equality is treated as qualifying; exact retail boundary and rounding semantics are unsourced.
- [x] Persist completed runs without fabricating a score formula. Timed and overtime rating remain disabled; next-dungeon selection remains disabled.
- [x] Persist versioned attributable completion records for offline armory rankings: character IDs, class/spec at settlement, completion timestamp, map/challenge, original key level/affixes, elapsed time including death penalties, authored timer and in-time result. Commit these atomically with key/history settlement; abandoned runs do not rank. Existing un-attributed runs are not backfilled. Active season remains null: historical DB2 rows alone do not select a current season. See [snapshot contract](site-snapshots.md).

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
- `scripted_ai::tests::data_bosses::mythic_follower_party_completes_azure_with_owner_key_and_follower_death` exercises real class-party and Azure population spawning, the Font, follower death and release, and owner-key settlement.

## Known gaps (current cycle)

- [x] Both-skin private native activation/weighted-kill/encounter/terminal proof: Modern8→11 within85s, Forever8→7 over2492s via existing private clock-advance2251. Both use genuine NPC death/BossAI boundaries, with administrative Damage/Revive, not synthetic snapshots. [Proof and boundaries](../wiki/systems/mythic-plus.md#proof--2026-10-11).
- [x] Cited timing and affix-band values come from tracked tables. US weeks are reconstructed individually from August 18 through the end of the October 27, 2026 reset week; the alternating Fortified/Tyrannical rows are inferred from the October 27 Fortified anchor. Pacific reset boundaries account for daylight saving time. The server is US-only for this policy; other regions are not implemented. The server reads `data/mythic_plus_weeks.json` for every slot and settlement, so updates need no rebuild. The tracked `crates/server-combat/src/mythic_plus_weeks.json` is a seed, not a runtime fallback. Slotting fails if the runtime file is missing, invalid or lacks the current week; settlement still saves the result and clears display affixes if that week is unavailable. The 11 observations do not contain two complete matching cycles, so they do not establish a projection. From the server checkout, `python3 scripts/refresh_mythic_plus_week.py --policy data/mythic_plus_weeks.json` initializes the runtime file from the seed if necessary and appends the current week only after two distinct US +7–9 WCL reports agree; it refuses conflicting observations. The `wcl` CLI needs its configured credentials.
- [ ] Rating is `None` for all runs; overtime rating, exact rating rounding, the depletion floor and next-dungeon selection remain unsourced and disabled. Guile's final-minute death behavior is also unresolved.
- [ ] Selected affix mechanics only where authored rules exist; historical affix rows are not a current active rotation.
- [x] Both-skin private UDP5800 native loop proof recorded; rating remains deliberately absent under the disabled numeric policy.
- [ ] Creature health and damage use the cited WCL level fit relative to an unweakened +2 and the documented Fortified/Tyrannical maxima. The runtime treats the authored baseline as +2; no Mythic 0 comparison establishes that absolute baseline. Exact rounding and the universal damage multipliers remain inferred. Existing creatures are scaled at activation; later creatures are scaled on spawn or the next fixed server tick. Player-owned pets and class followers are not dungeon enemies and are excluded. Affixes come from the observed US week and level bands.
- [x] Font activation resolves existing ordinary creatures' level, health, armor and loot difficulty for the Keystone copy before applying key scaling. Creature-specific tuning takes precedence over map tuning; preserve current health proportion and actor identity.
- [ ] Corpse loot selects difficulty-specific loot rows but `loot.rs` creates bare item IDs without carrying MapDifficulty ItemContext or generating its item bonus lists. Keystone ItemContext 16 is not yet applied to looted item instances.
- [ ] Scenario-event92 producers, full affix combat/art, active next-dungeon pool and Azure combat mechanics beyond the existing BossAI lifecycle are not covered. Slot drag-removal/banner animation parity is unclaimed.
- [ ] Selecting Mythic dungeon difficulty before queuing a follower dungeon requests that dungeon's owned keystone and admits the class-follower party to a fresh Mythic copy; insert and activate that key at its Font. The keystone gear source selects the tracked qualified WCL talents and equipment by specialization and key band, preserves race and talent provenance separately, and rejects missing profiles rather than using Normal gear. Grouped follower death transitions count toward an active run with the level-banded death penalty; class-party completion and release still need end-to-end proof.

## Out of scope

- Canonical world.db mutation, world population reimport, protected UDP5000 realm, master merge, invented combat/force/timer/score values and Forever-only functionality.

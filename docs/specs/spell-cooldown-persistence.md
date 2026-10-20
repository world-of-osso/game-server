# Spell cooldown persistence

A player's spell and category cooldowns survive logout and server restart, and a returning client is told what is still cooling down, following TC a352b1fa `SpellHistory::SaveToDB`/`LoadFromDB` (`character_spell_cooldown`) and `SendSpellHistory`. Recipes and other spells are treated alike; no source splits them.

## What it must do

- [ ] Repeating a spell on cooldown is rejected with `OnCooldown` in session, after relog and after a server restart, without spending reagents (recipe) or casting (class spell).
- [ ] Logout saves every active spell and category cooldown as a wallclock deadline; world entry merges the unexpired ones into the live history (never shortening a running clock).
- [ ] World entry replays each spell still cooling down as one `SpellCooldownUpdate` (`duration_ms` = `remaining_ms` = time left, `category` set when that spell started the running category clock).
- [ ] Expired rows are pruned when the character's cooldowns load; a fully expired row is removed.

## How it works

- [Spell casting](../wiki/systems/spell-casting.md#spell-cooldown-persistence), [persistence](../wiki/systems/persistence.md).

## Implementation inventory

- `crates/server/src/spell_cast/history_store.rs` — `character_spell_cooldowns` redb table, save/load/prune, world-entry restore and replay.
- `crates/server/src/networking.rs` — `queue_player_despawn` saves before the player leaves the world.
- `crates/server/src/persistence.rs` — table creation.

## Tests asserting this spec

- `crates/server/src/profession/spell_cooldown_persistence_tests.rs` — Prismatic Sphere28027 (48h category1174) and Berserker Rage18499 (60s) through the real trainer/cast path, disconnect path and reopened database.
- `crates/server/src/spell_cast/history_store.rs` (`spell_cooldown_load_prunes_expired_rows_from_the_store`).

## Known gaps (current cycle)

- [ ] Daily-reset categories (TC `SPELL_CATEGORY_FLAG_COOLDOWN_EXPIRES_AT_DAILY_RESET`) keep their fixed authored duration.
- [ ] Charges, global cooldowns and school locks remain in memory only.

## Out of scope

Pet cooldowns, item-use cooldown source overrides beyond the stored category clock, and periodic saves while online.

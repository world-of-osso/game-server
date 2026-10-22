# Follower primary-stat budget

`FOLLOWER_EXPECTED_STAT_PLAYER_BUDGET` is a lead-approved project policy (2026-10-22), **not recovered Retail follower scaling or gear**. It supplies the existing player-spec follower bridge with level-driven primary power from local 12.1.5.70448 `ExpectedStat.PlayerPrimaryStat`, default ExpansionID−2 rows. The source investigation is `game-engine/data/diagnostics/follower-power-scaling-2026-10-22/report.md`.

## What it must do

- [ ] Set the follower's total pre-passive specialization primary stat to its level's ExpectedStat budget: level35=180.685135, level70=375.046387. Apply the same rule to every supported follower class/spec, not named NPCs or one dungeon.
- [ ] Feed only the specialization primary-stat delta through the existing gear-primary input. Do not add the budget on top of class base stats or create equipped item identities.
- [ ] Preserve the budget through ordinary passive/stat refresh and recompute it on level changes. Native aura modifiers still apply at their existing stages.
- [ ] Leave stamina, HP policy, mana/resource bases, secondary ratings, armor, weapon selection, mastery and pets unchanged. Mastery/pet work remains separately owned.
- [ ] Leave player character stats unchanged. Missing class/level budget data must fail explicitly, without silently selecting naked stats or another level.

## How it works

- [Existing follower bridge and data boundaries](../wiki/investigations/follower-data.md).
- [Character stat computation](../wiki/systems/character-stats.md).

## Implementation inventory

- `scripts/gen_character_stat_tables.py` — maintained native class/game-table generator; the primary-budget data must retain its explicit DB2 provenance.
- `crates/server/src/followers/spawning.rs` — initial follower stat inputs.
- `crates/server/src/stats.rs` — ordinary player/follower stat refresh.

## Tests asserting this spec

- `crates/server/src/followers/power_budget_tests.rs` — concrete five-follower35/70 initial-sheet and refresh/level-change behavior; RED proof pending capacity.

## Known gaps (current cycle)

- [ ] Targeted behavioral RED→GREEN, follower kit/stat regressions, and same-seed Azure/Stockade before/after simulator evidence.

## Out of scope

- Retail follower combat item level, complete equipment and NPC combat AI remain unsourced; the approved policy does not claim to recover them.
- No HP/secondary budget allocation, arbitrary damage multiplier, timeout widening, mastery change or pet-wiring change.

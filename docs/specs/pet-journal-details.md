# Owned pet journal health and ability choices

Owned battle-pet injuries and chosen abilities are account state, not client presentation defaults. Sources: `crates/server/src/pet_journal_state.rs`, `pet_journal_details.rs` and the existing wild/PvP turn state. [Journal documentation](../wiki/systems/battle-pets.md) owns implementation details.

## What it must do
- [ ] Initialize missing owned combat state once from the authored species/breed/quality/level stats and first unlocked species abilities; never reset present injuries/choices on login or snapshot refresh.
- [x] Persist health and selected ability IDs by account and full instance GUID across database close/reopen.
- [ ] Publish owner-only current/max health, power, speed, next-level XP, selected abilities and real species ability options; existing PetJournal remains authoritative for current level/XP.
- [ ] Validate pet ownership, one-based slot1–3, species/slot membership, required level and account-wide queued/active battle lock before acknowledging a choice; refusals do not mutate state.
- [ ] Later PvE and PvP battles consume persisted injuries and choices; accepted rounds and battle-end progress survive reconnect/restart. Journal progression and combat fields commit together before publication.
- [ ] Captured pets retain actual terminal HP; level-up does not manufacture healing. Temporary max-health aura capacity is not durable.
- [ ] A new unlocked slot gets its first authored option without resetting an existing choice.
- [ ] Non-combat species have no fabricated combat snapshot.

## How it works
- [Account journal and battle integration](../wiki/systems/battle-pets.md)
- [Wire contract](../../../shared-protocol/docs/specs/pet-journal-details.md)

## Implementation inventory
- `crates/server/src/pet_journal_state.rs` — persisted health/choice records and authored initialization/selection.
- `crates/server/src/pet_journal_details.rs` — detail producer and validated request acknowledgement.
- `crates/server/src/battle_pets.rs` — account lifecycle and SQLite metadata.
- `crates/server/src/wild_pet_battles.rs`, `pvp_pet_battles.rs` — account battle lock, durable-state consumption and accepted-round persistence.

## Tests asserting this spec
- `crates/server/src/pet_journal_state_tests.rs` — real source stats, injury/choice database reopen, ownership and locked/foreign options/invalid slots.
- `crates/server/src/wild_pet_battles_tests.rs` — production capture/level-up/next-encounter/database lifecycle.
- `crates/server/src/pvp_pet_battles_tests.rs` — source choices/injuries, queued/active refusal, forfeit, requeue and database reopen.

## Known gaps (current cycle)
- [ ] Battle consumption/round persistence and owner-only native transport proof.
- [ ] Retail ability-selection client and acknowledged values in both skins.

## Out of scope
- Heal/revive timers and costs: unsourced; no invented policy or automatic restoration.
- Battle continuation across server restart; owned health/choice persistence is required, encounter resumption is not.

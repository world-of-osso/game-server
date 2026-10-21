# Follower healer triage

`crates/server/src/followers/` implements the explicit project-authored `FOLLOWER_HEALER_TRIAGE_POLICY` over native70448 follower kits. [Policy and provenance](../wiki/systems/follower-healer-triage.md).

## What it must do

- [ ] Rank living same-copy patients using missing HP, recent effective incoming damage, tank role and remaining native periodic healing/absorb capacity; deterministic ties.
- [ ] Budget native mana and prefer efficient effective healing under low mana/low damage; prioritize rescue when projected death is within the policy emergency horizon.
- [ ] Stop a normal direct heal when its patient becomes topped; reassess/retarget without restarting an already useful cast every AI tick.
- [ ] Pre-apply missing HoTs/shields against exposed real encounter EventMap deadlines; never invent spell metadata or numeric damage forecasts.
- [ ] Approach patients using native spell range, LOS, hazard and native-sight-safe movement; do not cross an unpulled sight envelope to heal.
- [ ] Permit damage only while the living party is safe.
- [ ] Expose a common read-only triage snapshot/decision hook used by both specialization policies before generic ordered kit rows.
- [ ] Drive Crenna's learned Restoration105 heal/HoT/proc/cooldown/utility synergies through existing native execution; record absent kit/native mechanics as named gaps rather than fake effects.
- [ ] Publish paired seeds1–20 clears/deaths/pull-end mana/overheal plus tank HoT uptime and native proc consumption; separate unavailable talent metrics from zero use.

## How it works

- [Named decision policy](../wiki/systems/follower-healer-triage.md).
- [Complete Crenna native inventory](../wiki/reference/follower-resto-70448-kit.md).

## Implementation inventory

- `crates/server/src/followers/kit_ai.rs` — original kit executor and triage integration boundary.
- `crates/server/src/followers/healer_context.rs` — exposed script deadlines and native-radius injury snapshots.
- `crates/server/src/scripted_ai/{mod,event_map,the_stockade}.rs` — read-only scheduled-damage hook and actual Hogger deadline mapping.
- `crates/server/src/followers/healer_triage_tests.rs` — native input/executor behavioral regressions.
- `crates/server/src/auras/observations.rs` — test-only native application/removal/partial-spend receipts.
- `crates/server/src/dungeon_simulator/healer_metrics.rs` — passive frame/pack-death/mana/HoT/proc reporting.

## Tests asserting this spec

- `crates/server/src/followers/healer_triage_tests.rs` — tests committed, RED execution pending.

## Known gaps (current cycle)

- [ ] Complete Restoration policy depth and paired simulator metrics beyond the qualified common framework/native cooldown slice.
- [ ] `FOLLOWER_RESTO_CONVOKE_NOT_LEARNED`: absent from local105 kit.
- [ ] `UTILITY_LAYER_REZ_PENDING`: Rebirth decision/native NPC-caster execution belongs to the utility layer; reserve its generic-row veto and consume that layer after landing.
- [ ] `FOLLOWER_RESTO_SOTF_WILD_GROWTH_BONUS_UNIMPLEMENTED`: current native bonus supports Regrowth/Rejuvenation/Germination only.

## Out of scope

- Retail healer-APL parity: SimC supplies no healer APL; timing/selection is explicitly project policy.
- New spell/talent grants, guessed costs/coefficients/durations/ranges or synthetic proc/resource grants.
- Holy257 specialization depth is a separate slice consuming the shared policy hook.

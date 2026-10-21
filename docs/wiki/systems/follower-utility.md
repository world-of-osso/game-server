# Follower utility

Native selected spellbooks feed a generic project scheduler outside damage/heal rotation. [Behavior contract and all named decisions](../../specs/follower-utility.md). Neither spell inventory nor these project choices establish Retail follower AI parity.

## Representation and execution

`follower_spec_kit` remains the learned level-interval inventory. `utility::normalize_entry` overlays utility decisions on cloned entries; it does not modify the database, grant spells, alter native costs or substitute spell effects. Census includes every row and explicitly labels passive/rotation/manual exclusions. `scripts/follower_utility_census.py` reads one SQLite transaction and writes reproducible JSON/Markdown evidence.

`utility_interrupt` collects already-engaged casts, follows native effect/trigger metadata for danger classification, and orders the party opportunity after its previous accepted kicker. A reservation fingerprints the exact enemy cast while a delayed accepted spell is in flight. Controls use the same engaged candidates after ordinary party kick attempts; hostile/CC/range/immunity validation stays in the real spell backend. Native dispel eligibility is shared with the actual dispel executor before project harm/health ranking.

`utility` samples actual health fractions on the manual/server clock, records boss-pull state on the owner and reserves admitted resurrection casts on corpses. Ordinary20484/391054 effects perform resurrection: a follower corpse accepts automatically; a player receives the existing real offer/acceptance path, with the follower's actual name. Movement invokes learned ordinary speed abilities only after safe existing pursuit has been selected. Existing protected-CC swing/cast cancellation and area-footprint checks remain authoritative.

## Geometry and simulator prerequisites

Production sizing follows the shared [model-geometry policy](model-geometry.md). Authored rows win; actual displays missing model-info use the approved scaled TC player defaults in both world and proximity-aggro lookups. The former follower-only rule is removed. The [source inventory](../investigations/ordinary-model-info-gap.md) remains a source gap, not fabricated authored data.

Generic kick/defensive/dispel/control opportunities run independently before rotation/triage selection, so a normal admitted heal cannot hide a dangerous removable control. Healer triage retains ownership of its rotation entries afterward. Utility dispel ranking compares removable dangerous control first, then the healer's existing uncovered-damage score (ordinary non-healers retain lowest-health ordering). This preserves both policies when integrating the independently landed healer changes.

The isolated simulator also needs the actual production pet controller and its native level-up catalog. Pet definitions, not entity/command/cast state, move between cached worlds. The first/cached-world owned Felguard action-bar regression exposes genuine learned89766 rather than an invented player kick. The formerly idle Bo first-fight regression proves native pursuit and positive damage after geometry closure. Full seed11 then clears all three native Stockade bosses; that single replay is not the complete matched comparison.

## Evidence boundaries

At4fe537f5d, the16-case native utility suite is GREEN48.19s, including direct follower/player resurrection. The corrected118 danger/dispel and both direct resurrection effect oracles first fail4/4 against the original scheduler/executor. Later geometry and strict combat-status coverage have their own receipts; source changes require scope-aware requalification rather than borrowing the earlier16-case proof. Authoritative current receipts and comparison manifest live under `game-engine/data/diagnostics/follower-utility-2026-10-21/`.

The metrics reader counts actual party/owned-pet Interrupt events, hostile danger-class CastSuccess events following CastStart, and actual Death events for the five party actors. Instant casts and disappearance are not counted as interruptible completions/deaths. Baseline/candidate must have identical seeded roster/level, pet initialization and geometry policy; only utility behavior may vary.

## Sources

- [Utility scheduler](../../../crates/server/src/followers/utility.rs) and [cast coordination](../../../crates/server/src/followers/utility_interrupt.rs)
- [Ordinary resurrection effect](../../../crates/server/src/death/resurrection.rs)
- [Native behavior fixtures](../../../crates/server/src/followers/utility_policy_tests.rs)
- [Trace metrics](../../../scripts/follower_utility_metrics.py)

## See also

- [[follower-data]] — source provenance and prior native fixtures.
- [[dungeon-simulator]] — transport-free harness and its acceptance boundary.

## Explicitly controlled native profile identity

A controller may execute an initialized native profile without a stored NPC `Follower` component (the simulator leader is a native Player). Party kick selection includes the current explicitly supplied profile while discovering other companions from their real stored components. It does not infer profiles for arbitrary players. The prior lookup dropped the leader kick: matched seed1 recorded5baseline owner Interrupts and0candidate, and a native component-boundary regression genuinely fails1/5.36s at1f3acc19a. Initial candidate comparison is retained as failed evidence; corrected requalification and20-seed outcomes remain pending.

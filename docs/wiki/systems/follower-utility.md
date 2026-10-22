# Follower utility

Native selected spellbooks feed a generic project scheduler outside damage/heal rotation. [Behavior contract and all named decisions](../../specs/follower-utility.md). Neither spell inventory nor these project choices establish Retail follower AI parity.

## Representation and execution

`follower_spec_kit` remains the learned level-interval inventory. `utility::normalize_entry` overlays utility decisions on cloned entries; it does not modify the database, grant spells, alter native costs or substitute spell effects. Census includes every row and explicitly labels passive/rotation/manual exclusions. `scripts/follower_utility_census.py` reads one SQLite transaction and writes reproducible JSON/Markdown evidence.

`utility_interrupt` collects already-engaged casts, follows native effect/trigger metadata for danger classification, and orders the party opportunity after its previous accepted kicker. A reservation fingerprints the exact enemy cast while a delayed accepted spell is in flight. Controls use the same engaged candidates after ordinary party kick attempts; hostile/CC/range/immunity validation stays in the real spell backend. Native dispel eligibility is shared with the actual dispel executor before project harm/health ranking.

`utility` samples actual health fractions on the manual/server clock, records boss-pull state on the owner and reserves admitted resurrection casts on corpses. Ordinary20484/391054 effects perform resurrection: a follower corpse accepts automatically; a player receives the existing real offer/acceptance path, with the follower's actual name. Movement invokes learned ordinary speed abilities only after safe existing pursuit has been selected. Existing protected-CC swing/cast cancellation and area-footprint checks remain authoritative.

## Geometry and simulator prerequisites

Production sizing follows the shared [model-geometry policy](model-geometry.md). Authored rows win; actual displays missing model-info use the approved scaled TC player defaults in both world and proximity-aggro lookups. The former follower-only rule is removed. The [source inventory](../investigations/ordinary-model-info-gap.md) remains a source gap, not fabricated authored data.

Enemy cast danger remains outermost: for each cast, try primary native kicks, then existing emergency heals, then secondary damage-spell kicks before considering a lower-danger cast. Healer specialization triage runs before generic defensive/dispel/control selection and retains ownership even when it defers or a native cast refuses. Generic utility must not consume Holy Purify before its clustered Mass Dispel or replace Resto's defensive choices. Non-owned utility still precedes ordinary damage rotation. Utility dispel ranking compares removable dangerous control first, then the healer's existing uncovered-damage order (ordinary non-healers retain lowest-health ordering). Shared `TriageContext::choose_dispel_target` applies that native harm rank to owned Holy Purify and Resto Nature's Cure without changing their specialization priorities or Holy's clustered Mass Dispel ownership. This preserves both policies when integrating the independently landed healer changes.

The isolated simulator also needs the actual production pet controller and its native level-up catalog. Pet definitions, not entity/command/cast state, move between cached worlds. The first/cached-world owned Felguard action-bar regression exposes genuine learned89766 rather than an invented player kick. The formerly idle Bo first-fight regression proves native pursuit and positive damage after geometry closure. Full seed11 then clears all three native Stockade bosses; that single replay is not the complete matched comparison.

## Integration with native defensive admission

Do not erase authored `health_below` or `attacked` conditions while normalizing utility entries. Existing native kit health/victim checks and tank-major sequencing remain valid admission; observed lethal decline adds an early defensive opportunity. Generic projected-death policy does not belong in `patient_is_eligible`, which specialization-owned Resto/Holy policies also use.

The fu913 matched regate uses master40812331c and utility+a1b2483d7 with identical source snapshot09dac63b/protocol2abc7fa/profile. Master passes12 of the13 reported cases; utility fails those12. Leymor fails both at the missing required bake-receipt environment variable. `fu913-plain-master-13` and `fu913-utility-merged-red-13` under the diagnostic root retain the genuine RED boundary. Ten cases were actual admission/ownership regressions; those pass after preserving the native policies. The approved Mark fixture now ends combat through native `CombatStop` before OOC maintenance. The old Overheat chase-only oracle is replaced by actual party Interrupt-or-collection coverage, plus a separate native cooldown-spent case retaining the original chase proof; no kit/range/routing edits.

At source1afc510b6 (mastere5f37f774), final scoped native qualification passes **95/95 in103.55s**, including all twelve non-environment reported cases, both owned healer dispel harm-priority regressions (2RED before the shared selector), ordinary utility retainers and the two Overheat coverage cases. Leymor remains the separately proven shared missing-receipt environment boundary, not qualified frontal/nav behavior. `fu913-final-packet.json` records all13 classifications and proof paths. The existing `healer_triage` filter also selects one native simulator benchmark; no paired20+20 comparison was rerun. Historical simulator outcomes below are not remeasured evidence for these integration changes.

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

## Backend-aligned matched outcomes

Twenty seeds per variant, four prescribed level32 rosters containing all eleven profiles as actual NPC companions. Baseline `5b38a7b86` and candidate `b552686de` share backend `bc09af9bd`, the same pet/geometry/home/addon prerequisites, native stats and simulator profile. `aligned-source-comparison.json` restricts runtime differences to utility integration, dispel eligibility export and follower resurrection. Both use the read-only snapshot SHA-256 `40d95b0a21a243edcf12dc9a1fa1ca56ad4b45f2828026622d609d62fab5cca5`.

| Actual trace outcome | Baseline | Utility |
|---|---:|---:|
| Party/owned-pet landed interrupts | 655 | 691 |
| Dangerous cast completions after CastStart | 14 | 23 |
| Native party Death events | 1 | 0 |
| Full three-boss clears | 19/20 | 20/20 |

These observations do **not** establish improved cast suppression: dangerous completions increased. Baseline seed2 has a real tank Death followed by an absent-tank block, not a harness stall or full-party wipe; totals include unequal combat exposure from that failed clear. No statistical survival guarantee follows from twenty seeds. No cooldown/stat changes or fake history/death recovery were used.

Reproduce with `scripts/follower_utility_metrics.py`, using `sim-matched-baseline-aligned` and `sim-matched-candidate-global` under the diagnostic root above; authoritative result is `matched-metrics-qualified.json`. `matched-{baseline-aligned,candidate-global}.proof.json` retain exact cohort commands/revisions/environment. Each directory's `overall-summary.json` combines all20 reports; original `summary.json` describes only its last five-seed invocation. Earlier candidate receipts remain archived, including the leader-kick regression, rather than overwritten.

## Explicitly controlled native profile identity

A controller may execute an initialized native profile without a stored NPC `Follower` component (the simulator leader is a native Player). Party kick selection includes the current explicitly supplied profile while discovering other companions from their real stored components. It does not infer profiles for arbitrary players. The prior lookup dropped the leader kick: matched seed1 recorded5baseline owner Interrupts and0candidate, and the corrected native component-boundary regression fails1/5.51s when restoring only the historical stored-component predicate (archived source). Earlier assertions observed the wrong owner-routed inbox and are not qualification evidence. The corrected oracle reads actual engine Interrupt events before delivery; candidate utility24 plus retained healer9 pass33/33 (one pre-existing ignored) in25.84s. Initial candidate comparison is retained as failed evidence; corrected20-seed outcomes remain pending.

## Party-global danger priority

Two native casters at0/80yards expose ordering across differently positioned kickers: a nearby Fireball must not consume the first party opportunity while another ready member can stop distant Polymorph. The second case requires a level26 Garrick native Shield for distant Polymorph while a nearby Mage has Counterspell for Fireball; existing emergency-heal-before-Shield remains intact. Both cases fail genuinely at9ffd96a8c (2FAIL16.72s). Candidate danger is now outside both member rotation and primary/secondary selection. At99cd62dcc the native utility/healer/retained composite passes79 cases in110.40s; one pre-existing Resto Nature's Swiftness depth regression remains ignored and is not claimed fixed. Backend-aligned paired outcomes are recorded below.

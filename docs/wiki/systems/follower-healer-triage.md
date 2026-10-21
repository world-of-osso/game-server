# Follower healer triage

`FOLLOWER_HEALER_TRIAGE_POLICY` is project-authored decision policy, not a SimC healer APL or a Retail rotation claim. Spell identities, learned level intervals, effect amounts, durations, ranges, resource costs and proc mechanics remain the local 70448/server catalog and ordinary spell executor. This page tracks implementation and proof separately; pending items below are design, not shipped behavior.

## Policy design (pending RED→GREEN)

- Rank same-copy living patients by uncovered missing health plus a three-second projection of effective incoming damage over the last ten wall seconds. Tank weight1.25; ties by entity bits. Existing active periodic-heal ticks and depleted school-absorb pools count only while their actual aura remains valid. Observed damage is not invented future boss damage.
- Emergency means projected survival at most three seconds, or health at most25% when recent damage is unavailable. At or below30% healer mana, or low incoming damage, prefer native effective healing per native mana over raw throughput. Never rewrite costs or grant mana/procs. Emergency response favors healing before the predicted death rather than a fixed per-spell HP ladder.
- Preserve an admitted useful cast. Cancel a normal direct heal when its living target is topped, then reassess; do not cancel a party-healing channel merely because its selected anchor is full. A new genuine emergency may interrupt damage or redirect a heal.
- Prepare missing HoTs/absorbs inside a three-second window of an exposed actual encounter EventMap deadline, using the script's real spell/target semantics. No numeric damage forecast is synthesized. Unexposed timers remain a named coverage gap.
- Pursue patients through native movement/range/LOS and existing hazard/native-sight safety. A blocked approach holds instead of crossing unpulled sight circles. Damage-weaving is allowed only while every living patient is covered and not projected into danger.

## Restoration105 ownership

Crenna209072 uses the whole local default follower kit, level gated; the census below is not a grant of all abilities at level32. Holy257 specialization decisions are maintained separately on the shared framework.

Supported mechanics to integrate: own Lifebloom maintenance and near-expiry bloom refresh; own Rejuvenation/Germination and actual Abundance threshold; Swiftmend/158478→114108 Soul of the Forest consumption; Omen113043→16870 Clearcasting→native free Regrowth; Nature's Swiftness132158; stacked-ally Efflorescence145205; group Wild Growth48438/Tranquility740; Ironbark102342, Innervate29166, Tree of Life33891; Nature's Cure88423; actual Rebirth20484/Revive50769; Mark1126; owned Moonfire164812/Sunfire164815 then Wrath5176 only when safe.

[Complete107-row native kit/talent/utility inventory](../reference/follower-resto-70448-kit.md), including exact learned level intervals and existing authored conditions.

Convoke is **not present in Crenna's local105 kit**: `FOLLOWER_RESTO_CONVOKE_NOT_LEARNED`. Do not add a spell solely because an example rotation mentions it. Current70448 Abundance207383 is a threshold-driven207640 buff, not old per-Rejuvenation stacks; see the native aura implementation. `FOLLOWER_RESTO_SOTF_WILD_GROWTH_BONUS_UNIMPLEMENTED`: existing Soul-of-the-Forest script boosts Regrowth/Rejuvenation/Germination, not Wild Growth. AI must use supported empowered heals without pretending the missing Wild Growth multiplier exists.

## Evidence / limitations

Initial base c6be257c5, frozen protocol03cebdb. Own simulator baseline e479f2de9; same seeds1–20 and source root for before/after. Baseline build underway; no success/metrics claim yet. Simulator has frozen-plane navigation rather than production LOS/nav; native movement proofs are required separately. Level32 cannot measure talents only learned later; absent gains must be reported as N/A, not 0% proc use.

## Sources

- `data/world.db`: `follower_profiles` entry209072; all `follower_spec_kit` spec105 rows; resolved native spell tables (read-only70448 inputs).
- `crates/server/src/followers/{kit_ai,kit,spawning}.rs` — original ladder, kit loading and learned-spell initialization.
- `crates/server/src/auras/{abundance,lifebloom}.rs`, `spell_cast/scripts/{clearcasting,soul_of_the_forest,swiftmend}.rs` — existing native synergy behavior.
- [Follower data](../investigations/follower-data.md) — native spell qualification/proc receipts.
- [Dungeon simulator](dungeon-simulator.md) — benchmark constraints and positioning proof.

## See Also

- [[follower-data]]
- [[dungeon-simulator]]
- [[spell-casting]]

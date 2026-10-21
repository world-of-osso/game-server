# Follower healer triage

`FOLLOWER_HEALER_TRIAGE_POLICY` is project-authored decision policy, not a SimC healer APL or a Retail rotation claim. Spell identities, learned level intervals, effect amounts, durations, ranges, resource costs and proc mechanics remain the local 70448/server catalog and ordinary spell executor. This page tracks implementation and proof separately; pending items below are design, not shipped behavior.

## Policy design (pending RED→GREEN)

- Rank same-copy living patients by uncovered missing health plus a three-second projection of effective incoming damage over the last ten wall seconds. Tank weight1.25; ties by entity bits. Existing active periodic-heal ticks and depleted school-absorb pools count only while their actual aura remains valid. Observed damage is not invented future boss damage.
- Emergency means projected survival at most three seconds, or health at most25% when recent damage is unavailable. At or below30% healer mana, or low incoming damage, prefer native effective healing per native mana over raw throughput. Never rewrite costs or grant mana/procs. Emergency response favors healing before the predicted death rather than a fixed per-spell HP ladder.
- Preserve an admitted useful cast. Cancel a normal direct heal when its living target is topped, then reassess; do not cancel a party-healing channel merely because its selected anchor is full. A new genuine emergency may interrupt damage or redirect a heal.
- Prepare missing HoTs/absorbs inside a three-second window of an exposed actual encounter EventMap deadline, using the script's real spell/target semantics. No numeric damage forecast is synthesized. Unexposed timers remain a named coverage gap.
- Pursue patients through native movement/range/LOS and existing hazard/native-sight safety. A blocked approach holds instead of crossing unpulled sight circles. Damage-weaving is allowed only while every living patient is covered and not projected into danger.

## Shared specialization boundary (pending implementation)

Common triage captures ranked patients/positions/projected death, native heal candidates, emergency state, safe-to-DPS and known upcoming scripted damage. A pure per-spec `decide(&TriageContext)` runs next, before generic ordered rows; read-only World/caster/party provide actual aura, cooldown and resource state.

The contract is `SpecVerdict::Cast { decision, owned: SpellSet } | Defer { owned: SpellSet }`, where `SpellSet` is a static slice of spell IDs. Both outcomes retain ownership even if native casting refuses the chosen action: common candidate fallback and generic rows must skip owned spells. Returning no action must never resurrect an older missing-aura/HP-threshold row. Resto and Holy use the same dispatch boundary; no runtime trait-object registry is required.

## Restoration105 ownership

Crenna209072 uses the whole local default follower kit, level gated; the census below is not a grant of all abilities at level32. Holy257 specialization decisions are maintained separately on the shared framework.

Supported mechanics to integrate: own Lifebloom maintenance and near-expiry bloom refresh; own Rejuvenation/Germination and actual Abundance threshold; Swiftmend/158478→114108 Soul of the Forest consumption; Omen113043→16870 Clearcasting→native free Regrowth; Nature's Swiftness132158; stacked-ally Efflorescence145205; group Wild Growth48438/Tranquility740; Ironbark102342, Innervate29166, Tree of Life33891; Nature's Cure88423; actual Rebirth20484/Revive50769; Mark1126; owned Moonfire164812/Sunfire164815 then Wrath5176 only when safe.

[Complete107-row native kit/talent/utility inventory](../reference/follower-resto-70448-kit.md), including exact learned level intervals and existing authored conditions.

Convoke is **not present in Crenna's local105 kit**: `FOLLOWER_RESTO_CONVOKE_NOT_LEARNED`. Do not add a spell solely because an example rotation mentions it. Current70448 Abundance207383 is a threshold-driven207640 buff, not old per-Rejuvenation stacks; see the native aura implementation. `FOLLOWER_RESTO_SOTF_WILD_GROWTH_BONUS_UNIMPLEMENTED`: existing Soul-of-the-Forest script boosts Regrowth/Rejuvenation/Germination, not Wild Growth. AI must use supported empowered heals without pretending the missing Wild Growth multiplier exists.

Additional native-consumer gaps found by source inspection (not yet behavioral qualification): `FOLLOWER_RESTO_NPC_RESURRECTION_OFFER_UNIMPLEMENTED` — `death/resurrection.rs::build_offer` requires a Player caster, while Crenna is an NPC; `FOLLOWER_RESTO_EFFLORESCENCE_HEALING_UNQUALIFIED` — native145205 summons47649, but no existing qualified Efflorescence healing consumer/placement fixture was found. Do not replace either with direct HP grants. Tree-of-Life/utility coverage remains pending native behavioral qualification; enumeration alone is not support.

## Initial common slice (five core regressions GREEN)

`healer_triage.rs` now records target-health-unit damage at the ordinary log boundary, ranks patient risk, subtracts live native HoT/shield coverage, compares effective healing/native mana, reads native charge/cooldown readiness, cancels topped normal direct casts, and gates damage. `healer_policy.rs` dispatches pure105/257 hooks with explicit owned-spell vetoes. Resto initially chooses common ranked candidates; Holy has an empty-ownership seam for the separate policy agent. Known upcoming timers, source-radius clusters and full Restoration policy depth are not yet implemented by this slice.

Six observed RED cases at simulator22f7bfa65/P03cebdb, server opt1/deps3,135.22s: five patient/cost/cast assertions and the missing benchmark observation schema. Failures were behavioral assertions, not compile/setup errors. Candidate0caa78225 integrated at simulator de4d815aa/P03cebdb: `followers::kit_ai::healer_triage_tests`, seven cases,6PASS/1FAIL in11.91s. All five core regressions GREEN: tank weight, recent damage, native HoT coverage, low-mana Swiftmend and topped-cast cancellation. Naturally earned Clearcasting→useful free Regrowth also GREEN. Nature's Swiftness-before-emergency-Regrowth is genuine RED (native aura132158 absent). Exact evidence: `data/diagnostics/healer-ai-2026-10-21/framework-native.log`. Unused speculative snapshot fields/helpers were removed from the finite framework pending real consumers. Post-master integration module filter6PASS/0FAIL/3 explicit next-slice ignores19.96s, zero build warnings. Scoped production check found a pre-existing simulator-only `has_uncleared_fight` warning; its fix is owned by agent13179, excluded here at the lead's request.

Holy agent13119's topped-target starvation hypothesis reproduced with native damage → native other heal topping the patient → repeated policy updates: the native Regrowth cast restarted despite full HP (genuine RED6.65s). A shared normal-direct-heal classification now rejects that full-target admission and governs cancellation; channels and preparatory HoTs are not excluded by this check. Finite ddf9396da integrated T395b2431f/P03ceb: module filter7PASS/0FAIL/3 next-slice ignores12.67s, including the fresh-injury positive control; scoped production checkPASS6.81s and zero build/check warnings before the requested exclusion of13179's helper change. Helper exclusion does not alter triage/native test behavior. Finite framework is not the full Restoration policy/final benchmark gate.

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

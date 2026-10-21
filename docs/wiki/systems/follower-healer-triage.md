# Follower healer triage

`FOLLOWER_HEALER_TRIAGE_POLICY` is project-authored decision policy, not a SimC healer APL or a Retail rotation claim. Spell identities, learned level intervals, effect amounts, durations, ranges, resource costs and proc mechanics remain the local 70448/server catalog and ordinary spell executor. This page tracks implementation and proof separately; pending items below are design, not shipped behavior.

## Policy design (pending RED→GREEN)

- Rank same-copy living patients by uncovered missing health plus a three-second projection of effective incoming damage over the last ten wall seconds. Tank weight1.25; ties by entity bits. Existing active periodic-heal ticks and depleted school-absorb pools count only while their actual aura remains valid. Observed damage is not invented future boss damage.
- Emergency means projected survival at most three seconds, or health at most25% when recent damage is unavailable. At or below30% healer mana, or low incoming damage, prefer native effective healing per native mana over raw throughput. Never rewrite costs or grant mana/procs. Emergency response favors healing before the predicted death rather than a fixed per-spell HP ladder.
- Preserve an admitted useful cast. Cancel a normal direct heal when its living target is topped, then reassess; do not cancel a party-healing channel merely because its selected anchor is full. A new genuine emergency may interrupt damage or redirect a heal.
- Prepare missing HoTs/absorbs inside a five-second project-policy window of an exposed actual encounter EventMap deadline, using the script's real spell/target semantics. No numeric damage forecast is synthesized. Unexposed timers remain a named coverage gap.
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

`healer_triage.rs` now records target-health-unit damage at the ordinary log boundary, ranks patient risk, subtracts live native HoT/shield coverage, compares effective healing/native mana, reads native charge/cooldown readiness, cancels topped normal direct casts, and gates damage. `healer_policy.rs` dispatches pure105/257 hooks with explicit owned-spell vetoes. Resto initially chooses common ranked candidates; Holy has an empty-ownership seam for the separate policy agent. The finite framework slice excludes upcoming timers, source-radius clusters and full Restoration policy depth.

## Shared context slice (native GREEN)

`CreatureAi::scheduled_damage` defaults to no exposed schedules. Hogger exposes the actual queued Vicious Slice/victim and Maddening Call/source-area request deadlines via read-only `EventMap::remaining_ms`; clocks, dispatch, repeats and effects are untouched. Common context only reads engaged same-copy sources and native spell radii; a five-second preparation policy permits missing HoTs before damage without synthesizing numeric damage. These are request deadlines, not promised impact times (cast delays and native refusals still apply). Other scripts/random-target schedules are unexposed coverage, not guessed timers.

`TriageContext.upcoming` contains source/spell/request-ms/affected-patients. `clusters` contains spell/anchor/injured members, using captured actual patient positions and native/script-owned radius. Group-heal admission requires the anchor's local cluster, never a map-wide injury count. Both new behavioral cases were observed RED before implementation: actual Hogger engagement/full-health tank preparation7.65s and scattered-versus-stacked Wild Growth6.33s. Integrated T134f81bd3/P03ceb module filter `followers::kit_ai::healer_triage_tests`:9PASS/0FAIL/1 explicitlyignored NS-depth regression14.38s, no test-build warnings. The preparation case additionally checks actual1–2s/3–4s source request ranges, read-only observation and Reset deadline removal. Scoped sim package checkPASS23.42s; only pre-existing `has_uncleared_fight` warning remains (13179-owned, explicitly excluded by lead). Changed-source formatting passes. Evidence: `data/diagnostics/healer-ai-2026-10-21/context-{green,check}.log`.

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

## Gate regression continuation (native GREEN)

Gateha847 exposed real policy regressions beyond the initial core tests. A context+starvation native replay retained five RED cases (Ironbark, Rejuvenation, no-reapply setup, eligible-HoT Swiftmend and the mana fixture); Henry's Magic dispel was already GREEN after the topped-start fix. The corrected six-case replay plus common regressions gives15PASS/0FAIL/1 NS-depth ignore24.05s (`gate-regressions-targeted.log`).

After merging masterff190cb1 into finite5cf87d561 (integrated simulatorT486527611/P03ceb), requested `followers::` and `dungeon_simulator::campaign::` filters give285PASS/0FAIL/9 existing-or-next-depth ignores206.38s, threads4. Leymor runs separately with the lead-qualified private2515 asset view/receipt and gives1PASS17.46s, threads1. Scoped sim server checkPASS8.61s; only the lead-excluded13179-owned `has_uncleared_fight` warning remains, with no unused triage API. Changed-source format checkPASS; delta readability audited directly, no suppression. Evidence: `regression-full-followers-campaign.log`, `regression-leymor-private-assets.log`, `regression-final-check.log` under `data/diagnostics/healer-ai-2026-10-21/`. This does not qualify the unimplemented Restoration-depth/paired-benchmark slices.

Ironbark is not a zero-throughput heal: its class decision now observes the authored attacked/health/missing-aura conditions and native readiness before ordinary healing. Shared `Patient.projected_death_seconds` is restored and prioritizes the most threatened eligible defensive recipient; Holy can reuse that exact forecast. `TriageContext.ready(spell_id)` reads learned rows plus native cooldowns/charges/school/costs, ignoring only transient GCD.

Swiftmend urgent admission retains its authored HP window and native target restriction, even when forecast HoTs would cover the deficit later. Routine cost ranking only spends Swiftmend when the actual native prerequisite state exists; it does not exploit the controlled-caster owner waiver to replace Rejuvenation on a70%-HP un-HoTed owner. Ordinary direct spell execution and that native waiver are unchanged.

The OOC mana-progress fixture previously cast a normal direct Regrowth on an already-full caster. Required topped-cast cancellation prevented the spend, so its intended recovery precondition never occurred. It now casts a learned native instant Mark1126 and asserts actual mana expenditure before retaining the unchanged60-second recovery/stall observations; no damage, fake mana grant or altered production recovery is introduced.

## Paired benchmark observations (implementation, qualification pending)

The test-only recorder samples after each actual simulator app update. Pull ends are the last native Death of each source pack that actually exchanged Damage with the party, not route decisions, corpse disappearance, party OOC or next-pull recovery. Mana/current max and healer life state are sampled on that0.05s frame; missing components stay null. Existing0.5s party samples remain the mana curve. Combat time and alive-tank combat time are separate denominators; own active native periodic-heal auras integrate tank coverage, with level-available native kit HoTs initialized to zero and triggered children added only when observed.

Test-only aura receipts count actual16870/114108 applications/refreshes, native charge gain, overcap refresh, expiry/other removal, and exact Consumed reasons. The partial2→1 spend boundary is observed separately; AuraRemoved alone is never treated as consumption. Rates with no opportunities are null/N/A. These callbacks only update an optional observation resource, never effects, actor resources or RNG. The same observer commits must be used on the source-aligned baseline and candidate; no20-seed comparison has run yet.

First observer qualification exposed an incorrect measurement assumption: native followers have no Player `KnownSpells` component. `spawning::initialize` applies the level-filtered passive list directly and AI executes level-gated catalog rows. Measurement now reads those actual rows through the existing test-only simulator seam and records active native talent auras, rather than inventing a Player spellbook or a fallback. This was a setup failure, not schema GREEN.

The first high-level pressure run also exposed invalid fixture preconditions: a bare spell-harness NPC has no threat table/Idle state, so AddThreat cannot establish an attacker; arbitrary source spell damage was not a requested recipient-health percentage after native scaling. Pressure fixtures now use real Garrick initialization, native threat engagement with an asserted victim, and ordinary direct-damage commits calibrated through native health-loss scaling. The channel control asserts actual forecast survival is shorter than its native duration before checking admission. Earlier assertions without these preconditions are not channel-safety or attacked-tank proof.

Rebirth decision and combat resurrection are owned by agent13120's utility layer (`origin/follower-utility`), not the Restoration hook. `UTILITY_LAYER_REZ_PENDING`: Rebirth20484 is reserved in the class-owned generic-row veto but never selected here. Native NPC-caster resurrection support is pending that layer's landed commit; no healer-local resurrection path or fake offer is added.

## Restoration upkeep continuation (GREEN pending)

Owned Lifebloom reads the selected living tank's actual own aura expiry/duration. It refreshes inside the qualified inclusive30% native bloom window through ordinary casting, bypassing only the generic missing-aura row that previously forbade refresh. Rescue retains priority; missing full-health upkeep requires observed incoming damage or an exposed preparation deadline, and settled OOC HoTs may expire naturally instead of manufacturing mana expenditure. No bloom/heal/proc is granted by policy.

## Efflorescence source preparation (qualification pending)

Runtime `content_creature_template`, models and addon lack47649; authenticated `tdb_creature_template` names it Efflorescence. The existing `retail_content.CLASS_SUMMON_ENTRIES` already reaches47649 through trait108116→native145205 summon effect, so no hard-coded closure override is needed. A durable SQLite backup was processed with `scripts/add_class_summon_templates.py --db <prepared-copy>`:33 exact class-summon templates were added with source models/addons and no world-spawn count change. SQL diff/manifest and backup live under the healer diagnostics directory; canonical publication is lead-owned. A real source-backed import/idempotency/no-spawn test is added before publication.

Source-only SotF scope check:158478/effect221440 and114108/effect127235 are effect6/aura4 dummy rows with four zero effect masks and zero class-option masks. These fields do not prove Wild Growth exclusion or inclusion. Label741 also contains unrelated class heals and is not bound by either dummy effect. Tooltip wording alone is not an affected-spell authority; WG scope remains unqualified, with no guessed multiplier added.

Nature's Swiftness now prepares a real emergency Regrowth through its learned native132158 cast. Readiness does not incorrectly require Regrowth's pre-buff mana budget; native costs/cast time are re-read after the actual buff. An admitted friendly heal is not restarted merely to prepare NS. The actual active NS aura admits a useful Regrowth, never a topped recipient; all instant/free/healing changes remain native spell modifiers. The previously observed level13 RED is enabled for GREEN qualification.

Owned Nature's Cure uses the row's actual native dispel types and eligible harmful aura before routine healing, after rescue preparation. It does not repeatedly select a lower-HP non-dispellable recipient or claim a dispel without the real native effect. An admitted useful friendly cast may finish instead of being restarted.

Lead confirmed paired comparison control: landed framework/master655258480 (including native target118), with identical frozen inputs; historical legacy healer is optional and separately labelled. Pressure/buff fixtures register their actual initialized companions with GroupRegistry and the authenticated owner's ClientId, rather than treating an arbitrary Vec as native party membership. This keeps Mark/raid-target behavior on the same native boundary as the real simulator.

Unsafe recent party pressure cancels an already-admitted hostile cast even when no heal is affordable; native cancellation, no synthetic recovery. LOW_MANA remains the shared project decision threshold, not a native spell effect. Genuine RED: resto_policy_stops_admitted_damage_when_recent_pressure_is_temporarily_topped; qualification pending.

Learned Innervate29166 is selected at the shared low-mana threshold with actual healing demand, after emergency/dispel decisions; only native readiness and periodic regeneration apply. No free-cast effect or mana grant is invented. Genuine low-mana Innervate RED precedes this selection; qualification pending. Lead published33 source-derived class-summon templates,45 models,4 addons including47649 after sqldiff/integrity checks; runtime Efflo healing still unqualified.

Tranquility selection uses the learned native row, actual native-radius injury cluster and caster survival longer than native channel duration. It precedes single-patient Nature’s Swiftness preparation; generic maintenance cannot bypass this channel admission. Genuine stacked-pressure RED precedes selection, strict unsafe-caster fixture retained; qualification pending.

Abundance preparation reads the actual passive207383 threshold and owned active primary774 HoTs. One useful eligible Rejuvenation may complete that threshold; no topped recipient, fake stack/discount, or hardcoded native threshold. The native buff207640 alone supplies cost/crit effects. Genuine fifth-HoT RED precedes selection; qualification pending.

Efflo runtime qualification fixture now requires ordinary145205→real47649 at patient placement→summon81262→owner-attributed81269 useful healing after its native period; RED not run yet. Script source is TrinityCore a352b1fa `src/server/scripts/Spells/spell_druid.cpp` lines719–778 (archived D/tc-a352b1fa-spell_druid.cpp): after-cast summon initializes81262, its periodic dummy asks the owner to cast81269 at the summon, with three random injured targets. Native70448 rows match the IDs/period/radius; this is not permission to invent alternate effects.

Frozen-input first depth run: NS/Innervate/Cure/unsafe-damage cancellation GREEN4; eight native-party cases failed during fixture setup because Harness owner has Level, not UnitLevel. Group roster now reads actual initialized Level for every member; no fallback or fabricated level. Those eight are not behavioral failures or Efflo RED.

Corrected native group run: Ironbark, late Lifebloom/bloom, full-party Mark, fifth-HoT Abundance, unsafe-channel rejection and supported SotF consumption GREEN6. Tranquility admission still RED; fixture now logs real patients/clusters/recovery/costs. Efflo reached missing summon before81262; its narrow template helper deliberately defaults summon-level tables, so qualification now loads the complete frozen native WorldData rather than fabricating those inputs. Native runtime gap is not yet isolated.

Tranquility native740 effect0 targets caster1, range index1 has zero range; passing a remote injured patient first invokes pursue with that zero range. Selection now channels on the caster and counts authored-threshold injuries in the caster-anchored native-radius cluster. No native targets/range/timing changed. Targeted pressure qualification remains pending.

Native Efflo script slice (after genuine81262 RED): capture actual new145205 summons across its synchronous native effects, then initialize their81262 after effects; each periodic dummy asks the native owner to cast81269 with ordinary checks (source DONT_REPORT_CAST_ERROR, not FULL_MASK). Native destination/radius selection remains unchanged. Source Spell.cpp `SelectRandomInjuredTargets` keeps full-health units eligible and ranks injury→player→actual group before random ties/cap3. NPC treat-as-raid static-flag classification still has no producer, as in existing Prayer of Mending; that priority subcase remains unqualified. Effect179 old-area-trigger cleanup and AI Efflo admission remain separate unqualified boundaries, not completed by the healing hook.
The bounded before/after summon snapshot represents the source's per-cast ExecuteLog without adding a general execution-log subsystem; replace it with that native receipt if the runtime gains one or Efflo gets deferred/nested same-spell summon effects.

New Efflo AI admission fixture keeps actual clustered damage stable (recent rate below emergency threshold), isolates the learned native145205 row and requires an actual summon. This separates native script healing from the common estimator’s zero direct-heal metadata rejection; RED not run yet, no selector implementation added.

Additional depth qualification now explicitly covers learned level13 Barkskin on its actually attacked healer, level33 Germination second owned HoT under stable uncovered native pressure, and level70 Tree of Life after a real native Tranquility channel/cooldown. These tests are unrun RED candidates; no corresponding policy implementation is present yet. Rebirth remains utility-owned, Convoke remains absent rather than granted.

Efflo recast lifecycle now has an unrun native fixture: two ordinary casts, real GCD, actual prior area-trigger entity, moved placement and exactly one current owned area trigger. It qualifies TC145205 effect2 old-area removal separately from81262 healing and AI admission. No cleanup implementation before RED.

Tree activation alone is not throughput qualification. TC a352b1fa SpellAuraEffects.cpp1436–1530 gives form2 actual5420/81097 boost auras; local70448 rows provide their spell modifiers. A new unrun fixture requires real immediate Regrowth healing with those native boosts, then ordinary cast time and removed boosts after leaving form. No percentages, spellbook grants, or replacement buff may be fabricated; native-effect RED precedes implementation.

After genuine attacked-healer RED, Barkskin joins the owned defensive selector before throughput and external Ironbark. Both retain actual authored HP/attacked/missing-aura conditions and native readiness; self rows cannot select another patient. No damage reduction is synthesized. Qualification pending.

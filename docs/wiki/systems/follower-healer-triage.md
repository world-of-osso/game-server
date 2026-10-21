# Follower healer triage

verified: 2026-10-21

`FOLLOWER_HEALER_TRIAGE_POLICY` is project-authored selection over native70448 execution, not a SimC healer APL or Retail rotation claim. [Contract](../../specs/follower-healer-triage.md), [complete107-row Crenna inventory](../reference/follower-resto-70448-kit.md), [follower data](../investigations/follower-data.md), [simulator constraints](dungeon-simulator.md).

## Common decisions

- Rank living same-copy patients by uncovered missing health plus three seconds of recent effective incoming damage. Damage history covers ten seconds, converted through native health-loss scaling. Tank weight1.25; deterministic entity ties. Actual remaining HoT ticks and depleted absorb pools count only while valid.
- Emergency means projected survival≤3s or HP≤25%; mana≤30% or no incoming damage favors effective native healing/native mana. These are policy thresholds, not altered spell effects. No resource/proc grants.
- Reject/cancel topped normal direct heals, preserving useful friendly casts and group-healing channels. Cancel an admitted hostile cast while party pressure is unsafe even when no heal is affordable.
- Prepare missing HoTs inside a five-second policy window of an exposed real EventMap request deadline. Hogger exposes Vicious Slice/victim and Maddening Call/source-area clocks; request time is not guaranteed impact time. Other scripts remain unexposed, not guessed forecasts.
- Use actual native-radius injury clusters, existing pursuit/range/LOS/hazard controllers and native-sight safety. The simulator's frozen-plane navigation does not prove production navigation or pixels.

`TriageContext` exposes ranked patients, native heal candidates, upcoming requests, injury clusters, enemies, actual owned auras and learned/native recovery readiness. Readiness ignores transient GCD only, not costs, charges, school or cooldowns.

`SpecVerdict::Cast { decision, owned: SpellSet } | Defer { owned: SpellSet }` always retains its static owned-ID slice. Native refusal or no chosen action must not resurrect generic owned rows. Both105 and257 use this shared pure decision boundary; Holy policy is separately owned.

## Restoration decisions

Native level intervals remain authoritative: a level32 benchmark does not learn high-level abilities. Ordinary costs, cast times, effects, aura restrictions and modifiers remain native.

| Capability | Selection / native boundary | Proof |
|---|---|---|
| Ironbark / Barkskin | Authored attacked/HP/missing-aura conditions before throughput; personal rows stay self-targeted | Actual threat victim and scaled native damage GREEN |
| Lifebloom | Living tank own-aura upkeep; refresh inside inclusive30% native bloom window; no settled-OOC spam | Actual late refresh and native33778 bloom GREEN |
| Nature's Swiftness | Learned132158 before emergency cast-time Regrowth; re-read native costs after buff | Level13 aura→useful Regrowth GREEN |
| Nature's Cure | Actual eligible native harmful aura before routine healing | Magic589 removal and native Dispel GREEN |
| Swiftmend / Soul | Urgent real eligible low HP; routine use requires native prerequisite, not owner waiver exploitation | Eligible-HoT rescue, no Rejuv starvation, earned114108→useful Regrowth and expiry/consumption distinctions GREEN |
| Clearcasting | Native earned16870 supports useful free Regrowth; no topped recipient | Natural acquisition, native costs and partial charge-spend telemetry GREEN |
| Rejuvenation / Abundance | Useful primary774 may complete actual207383 threshold; native207640 supplies discount | Fifth primary HoT and actual cost reduction GREEN |
| Germination | Learned155675, existing owned primary and absent secondary, stable uncovered demand; waive only project primary-missing condition | Ordinary774 creates real155777 GREEN |
| Innervate | Learned29166 under actual healing demand/low mana | Native self aura GREEN; it is periodic mana regeneration, not a zero-cost aura |
| Tranquility | Caster-anchored cluster and native caster survival longer than channel; cast on caster (native target1/range0) | Useful157982 child healing and unsafe-channel rejection GREEN |
| Tree of Life | Learned33891 under sustained authored group injury after safe Tranquility opportunity; preserve useful casts | Actual5420/81097 boosts→immediate useful Regrowth→boost removal/ordinary cast time restored GREEN |
| Mark | Native1126 and actual GroupRegistry/ClientId companion membership | All five living native members GREEN |
| Damage weaving | Generic native attack/area rows only while party safe | Unsafe admitted hostile cancellation with no affordable heal GREEN |

`FOLLOWER_RESTO_SOTF_WILD_GROWTH_BONUS_UNIMPLEMENTED`: existing native bonus supports Regrowth/Rejuvenation/Germination, not a qualified Wild Growth bonus.70448 dummy effects158478/221440 and114108/127235 have zero masks; those rows prove neither WG inclusion nor exclusion. Do not derive an affected-spell multiplier from tooltip prose or unrelated labels.

`FOLLOWER_RESTO_CONVOKE_NOT_LEARNED`: no local105 kit row; never grant it for a rotation example. `UTILITY_LAYER_REZ_PENDING`: Rebirth20484 is reserved/vetoed here but never selected; utility owns decisions and native NPC-caster execution. No healer-local fake resurrection offer. Other manual-only controls/forms/utilities remain enumerated in the linked inventory, not claimed automated.

## Efflorescence

The existing `retail_content.CLASS_SUMMON_ENTRIES` reaches47649 through trait108116→145205; no closure override is needed. Lead published33 authentic class-summon templates,45 models and4 addons, inserts only with sqldiff/integrity proof. Source producer/idempotency/no-spawn test PASS1/6.838s. Never edit canonical DB from the healer worktree.

TC a352b1fa `spell_druid.cpp719–778` initializes new145205 summons with81262, whose periodic dummy asks the owner to cast81269 at the summon. `Spell.cpp::SelectRandomInjuredTargets` ranks injury→player→actual group, randomly caps the boundary tier at3, and keeps full-health recipients eligible. Native70448 supplies period/radius/coefficients. Ordinary child checks are retained: source DONT_REPORT_CAST_ERROR is not FULL_MASK.

Runtime qualification is GREEN for ordinary145205→actual47649 at patient placement→81262→owner-attributed81269 useful healing. Source effect2 removes only the caster's previous145205 area trigger before replacement; actual old-entity/moved-position recast fixture is GREEN. Stable-field AI admission uses learned readiness, authored HP/min-injury conditions and actual native-radius geometry, skipping a covered active owned field. Its genuine zero-direct-heal-estimate RED is GREEN without synthesizing a heal estimate.

The synchronous before/after summon snapshot substitutes for TC's per-cast ExecuteLog receipt. Replace it if the runtime gains that receipt or Efflo gets deferred/nested same-spell summons. NPC static-flag4 treat-as-raid priority has no producer; that priority subcase remains unqualified. Area-trigger replacement does not authorize deleting native summons.

## Observations and paired inputs

Test-only recorder samples after app update every0.05s. Pull end is a source pack's actual last native Death after party Damage exchange, not corpse disappearance, party OOC or next-pull recovery. Mana/current max/life state come from that frame; absent components stay null. Existing0.5s samples preserve the mana curve.

Tank coverage integrates actual own active native periodic-heal auras over alive-tank combat time; combat and alive-tank denominators remain separate. Native aura receipts distinguish application, charge gain, overcap refresh, partial2→1 spend, actual Consumed, expiry and other removal. No opportunity means null/N/A. Optional receipts never alter effects, resources or RNG. NPC kit availability comes from initialized level-gated rows, not nonexistent Player KnownSpells.

Lead-selected control is landed framework/master655258480 with identical frozen inputs, not historical legacy AI. Baseline181b44bb0 adds only the same seven-file passive observer. Candidate2d4dc7856 contains the Restoration slice. Six observer files match byte-for-byte; parent `auras/mod.rs` differs only by native Tree apply/remove hooks, not callbacks. Protocol03cebdb971894aa1fa69b2e4ba7e7fae2702d515; sim server opt1/deps3; Stockade level32, seeds1–20, jobs2.

Evidence root: `game-engine/data/diagnostics/healer-ai-2026-10-21/`. `paired-input-manifest.json` pins source revisions, observer files, graph and frozen SQLite SHA2566b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c; `paired-stockade-ground-hashes.json` pins40 geometry/LOS files. Rehash after each batch. Raw reports preserve all failures; never discard uncleared runs from denominators.

Baseline20 completed:11cleared/8timeout/1stalled,1party death, no missing reports;905.95s. Native healer overheal50.875%; weighted tank HoT uptime Lifebloom78.52%, Rejuvenation68.23%, Regrowth80.78%; Clearcasting2153/2429 charges, Soul2947/2963. `baseline-framework-metrics-summary.json` and `baseline-framework-655258480/` are source evidence, not a candidate improvement claim. Candidate20 completed:10cleared/9timeout/1stalled,1party death, no missing reports;900.73s. No clear-rate or mortality improvement is demonstrated. Native overheal51.673%; tank HoT uptime Lifebloom95.76%, Rejuvenation74.85%, Regrowth81.47%; Clearcasting2676/2949 charges, Soul2903/2917. Median observed pull-end mana87.84%→89.35%,10th-percentile58.91%→64.20%; per-run mean minimum2.15%→4.55%. `paired-healer-metrics.json`, `paired-runs.csv`, `paired-mana-curve.csv` (283223samples) and `paired-pull-end-mana.csv` (3040pack ends) preserve denominators and all40run outcomes. Native combat/proc RNG uses thread_rng, so equal scenario seeds are not common random numbers. These descriptive measurements support better HoT coverage, not causal survival/efficiency improvement; overheal rose0.80percentage points. DB/40geometry hashes stayed unchanged after both batches.

## Proof ledger

Historical finite framework2468b84fd landed ed677e53e:285followers/campaign passes, separate qualified-asset Leymor1pass, scoped check/fmt PASS. Gate fixes retained authored defense priority, genuine Swiftmend eligibility and paid native1126 mana-recovery precondition; they did not change native spell effects.

Depth receipts are separate scopes, not one fresh aggregate run:

| Receipt | Result / scope |
|---|---|
| `resto-native-depth-frozen-first.log` |4PASS: NS, Innervate, Cure, unsafe damage cancel; eight fixture setup failures subsequently corrected |
| `resto-native-party-level-corrected.log` |6PASS: Ironbark, late Lifebloom, Mark, Abundance, unsafe channel, Soul; two subsequent native REDs |
| `resto-native-tranquility-self-and-efflo-inputs.log` | Tranquility PASS; Efflo genuine81262 RED after real summon/placement |
| `resto-efflo-hook-and-remaining-depth-red.log` | Efflo native chain PASS; five genuine decision/lifecycle REDs |
| `resto-four-fixes-and-native-tree-red.log` |4PASS: stable Efflo/Barkskin/Germination/area replacement; native Tree boost genuine RED |
| `resto-native-tree-and-defensive-channel-green.log` |4PASS: Tree boosts/activation, affected Ironbark/Tranquility |
| `baseline-framework-observer-contract.log` | Native accounting contract PASS1/80.91s |

Fixture errors are not behavioral REDs: Harness owner exposes Level, not UnitLevel; native party-target spells need real companion GroupRegistry; narrow creature fixtures omit summon-level tables, so Efflo uses full frozen native WorldData. Attack fixtures assert a real ThreatTable/Idle→AddThreat victim; native damage is scaled to recipient-health units and asserted before policy checks.

Integration after candidate2d4 merges newer master rotation/native work separately; frozen comparison must retain its exact source provenance rather than attribute unrelated party-AI changes to Restoration.

## Sources / see also

- Native70448 resolved tables and kit intervals in the frozen read-only DB; source observations use ordinary executor paths.
- TC a352b1fa `spell_druid.cpp719–778`, `Spell.cpp::SelectRandomInjuredTargets`, `SpellAuraEffects.cpp1513–1516,1570–1617`; archived under the evidence root.
- [[follower-data]], [[dungeon-simulator]], [[spell-casting]].

## Master integration gate

Source06cd80b2b includes masterc6bc60006 and the final native Efflo cap/injured-player-priority fixture. Focused `kit_ai`, companion group buffs, Germination, Abundance and observation contract:39PASS/0FAIL/0ignored58.74s, compile5m15s. Scoped sim server checkEXIT0/45.12s; sole inherited `has_uncleared_fight` warning remains explicitly lead-excluded/13179-owned. `cargo fmt --all --check` PASS. Changed Rust manually audited for readability (analysis CLI unavailable); no new suppressions, synthetic effects or unrelated fixes. Evidence `resto-final-master-integration-{targeted,check}.log`, `resto-final-format-check.log`. Documentation-only qualification updates do not invalidate that code proof.

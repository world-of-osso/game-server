# Class completeness continuation

Verified: 2026-10-12. Full spell/talent goal remains open across all 13 classes / 40 specs; probe dispatch is not whole ability, legal talent build, or rotation certification. [Baseline](class-completeness.md), [contract](specs/class-completeness.md).

## Census milestones

| Revision / dependency | Working at probe boundary | Partial | Broken | Proof |
|---|---:|---:|---:|---|
| 1efd4c1b / shared252f4099 | 2841 | 4607 | 379 | Baseline retained at its recorded revisions |
| 92afa752 / shared6ee323ea | 2841 | 4607 | 379 | All 7827 memberships, exit0, 734.82s |
| 8b7d4723 / shared6ee323ea | 2844 | 4687 | 296 | All 7827 memberships, exit0, 462.12s |
| 4e187808 / shared6ee323ea | 2851 | 4687 | 289 | All 7827 memberships, exit0, 479.06s |
| ccac02c0 / shared6ee323ea | 2854 | 4687 | 286 | All 7827 memberships, exit0, 478.52s |
| c9d545a7 / shared6ba2f5cd | 2854 | 4720 | 253 | All 7827 memberships, exit0, 789.23s |
| c63bbbdd / shared6ba2f5cd | 2854 | 4774 | 199 | All 7827 memberships, exit0, 810.59s |
| cbe56541 / shared6ba2f5cd | 2854 | 4776 | 197 | All 7827 memberships, owned prepared snapshot, exit0, 568.84s |
| 479cc6f4 / shared6ba2f5cd | 2854 | 4779 | 194 | All 7827 memberships, exit0, 474.79s |
| b6886dec / shared6ba2f5cd | 2854 | 4781 | 192 | All 7827 memberships, exit0 |
| 7c1e1b33 / shared6ba2f5cd | 2854 | 4787 | 186 | All 7827 memberships, exit0, 560.59s |
| 96e4cb20 / shared6ba2f5cd | 2854 | 4788 | 185 | All 7827 memberships, exit0, 556.76s |
| 9f2c5cbd / shared6ba2f5cd | 2854 | 4791 | 182 | All 7827 memberships, exit0, 603.33s |
| 42c85bc2 / shared6ba2f5cd | 2860 | 4785 | 182 | All 7827 memberships plus fresh source-gap analysis, exit0, 539.22s |
| 8fcddcd9 / sharedd503cd3 | 2870 | 4775 | 182 | All 7827 memberships, current explicitly proved named consumers, exit0, 622.23s |
| 672a43a4 / sharedd503cd3 | 2874 | 4771 | 182 | All 7827 memberships plus exact231040s2 consumer, exit0, 705.59s |
| 78ea6e06 / shared2a48bdc | 2878 | 4767 | 182 | All 7827 memberships plus exact441675s3 consumer, exit0, 736.94s |
| 174a2e84 / shared2a48bdc | 2889 | 4756 | 182 | Native aura229, all7827 memberships, exit0, 696.66s |
| a5de3082 / shared00d33e7 | 2897 | 4748 | 182 | Native aura429/raw mastery, all7827 memberships, exit0, 745.53s |
| 578c18d7 / shared00d33e7 | 2901 | 4744 | 182 | Native434249 ready-time credit, all7827 memberships, exit0, 716.33s |
| 36758fce / shared00d33e7 | 2905 | 4740 | 182 | Native429433 counters/charged missile modifiers, all7827 memberships, exit0, 602.80s |
| 116d3eff / shared00d33e7 | 2911 | 4734 | 182 | Native467 bonus-stat negation, all7827 memberships, exit0, 606.66s |
| 1deb60e3 / shared00d33e7 | 2928 | 4717 | 182 | Native142 physical base armor, all7827 memberships, exit0, 539.21s |
| 1441a273 / shared00d33e7 | 2934 | 4711 | 182 | Native305 player minimum speed, all7827 memberships, exit0, 565.22s |
| dfc71656 / shared00d33e7 | 2965 | 4680 | 182 | Native466 final player armor, all7827 memberships, exit0, 477.51s |
| d41f51c2 / shared00d33e7 | 2969 | 4676 | 182 | Zero-amount284/285 child linkage, all7827 memberships, exit0, 545.53s |
| 8d143318 / shared00d33e7 | 2983 | 4662 | 182 | Native330 moving cast/channel masks, all7827 memberships, exit0, 549.22s |
| 5e015838 / shared00d33e7 | 3000 | 4645 | 182 | Native232/234 receiver duration rule, all7827 memberships, exit0, 515.50s |
| 78413ce2 / shared80bec38 | 3000 | 4645 | 182 | Mainline-integrated current-source census, all7827 memberships, exit0, 581.54s |

First census resolves target24 for native117418 in three specs; integrated master's native1804 adds three unsupported item-target26 failures. Net broken change is zero, not zero repairs. Second census repairs fixture fidelity: production already loads authored teleport destinations, while Cargo runs fixture startup from the crate directory, where relative `data/world.db` fails. Census now explicitly loads the same native destination/map data and supplies actor identity, facing, instance manager and ungrouped state. This is not newly implemented production teleport behavior.

## Shared mechanics and proof

- `a237459f`: native enemy cone24 follows caster facing, DB2 ConeDegrees and the target-slot radius. Native117418 RED2 unsupported-target failures → GREEN2: damage, front/back/side/range/friendly exclusions, facing and target cap. Unit-center geometry remains; combat-reach expansion is not certified.
- `404908ce` → `7baf6265`: native3567 destination RED missing row → GREEN authored MapTransfer destination. Far-teleport acknowledgment/arrival is not proved by this new test.
- `3b66c134` → `8b7d4723`: native363534 Rewind reproduces missing fixture group state, then GREEN alongside the city control. Incoming-damage history remains a separate missing mechanic.
- `50be4847` → `ca817b6c`: native212036 mass resurrection RED explicit-dead-target rejection → GREEN native corpse selection, one group-member offer despite distant ghost position, stranger exclusion, acceptance, and DB2-derived restoration percentage. Target119 uses stored corpse positions and party/raid membership. Corpse state lacks a separate instance key; cross-copy/cross-map ghost bodies remain uncertified. Subsequent census milestones above retain the probe-only boundary.

## Later shared roots

- Native spellcaster22 objects load TDB Data0 spell, Data1 charges, Data2 partyOnly and Data8 interaction radius. Native10059 creates176296 and owner/group uses reach the authored17334 ->121857 destination. RED128c6197 -> GREENcb213f02; de877b89 owner/party/range/dead/stranger/map controls3/3. Nonzero additional conditions remain explicit; positive-charge destruction and all non-portal effects are not certified.
- Identity-only PlayerCondition class/race checks consume native masks with ChrRaces playable-bit mapping and preserve signed low/high race words. Native11416/924 Ironforge portal REDb3e9a1fd -> GREEN5e121e9f. Unknown predicate fields are never discarded. Pin: ConditionMgr.cpp2903-2959, DBCEnums.h2189-2203.
- Target91 was initially implemented with the wrong deterministic direction; that claim/assertion is invalidated. Pinned SpellInfo.cpp339 specifies RANDOM. REDb5c8076d -> GREENc63bbbdd preserves authored ring radius and ownership while choosing random directions. Targets72/86 share the native algorithm for1276452/123904; position RED17ab5ee1 ->4428089e, then genuine missing templates exposed the next data boundary.
- Native63508 Xuen and258584 Imp Lord existed in TDB but the importer retained only world-spawn/transform/toy templates. Class/spec/talent declared-child closure now retains authentic templates/models/addons without creating world spawns. Native import REDebea74cb ->51bc8993 GREEN; prepared copy runtime at1763eab4 spawns both owned actors on the correct random rings. Native import controls2/2 include existing-row preservation and repeat application. No guardian AI/stat-inheritance certification follows.

`python3 scripts/add_class_summon_templates.py --db <prepared-copy>` applies the same rule additively; it rejects the canonical `world.db` basename. Fixtures snapshot the native database read-only, apply the producer to their owned copy, then load production WorldData. Canonical data remains untouched.

## Native ground, rectangle, ritual and controlled-pet follow-up

- Native196718 target62 uses ground height beneath the caster, not airborne caster height: RED98922823 -> GREEN64f216e8. Native197214/392983 target129 uses authored caster-facing Width/radius: RED61e7d171 -> GREENb6886dec, with front/back/ally/range controls. Darkness avoidance and combat-reach expansion remain uncertified.
- Native698/342601 rituals use TDB Data1 participant counts3/5 and native completion IDs62330/342607, not legacy18541: metadata RED -> GREEN7c1e1b33 plus19 meeting-stone controls. Persistent Doomguard/sacrifice behavior remains open.
- Native31687 summons creature78116 (Effect21314), not historical510. Controlled-pet recall, persistence/reopen, dismissal and death/resummon have native targeted proof through96e4cb20;9 pet controls pass. Mana uses native class-level/template values. School-specific spell-power inheritance and complete pet stats/AI remain uncertified.
- Native1804 exposes the general RealPointsPerLevel omission: REDa35cdb30 zero versus14 at level28 -> GREENe9b7b565, including levels29/80. General value calculation follows pinned SpellInfo.cpp::CalcValue and its expected-stat exclusions.

### Item-target lock boundary

Effect33/target26 resolves an owned bag GUID, reads native ItemSparse.LockID and ordered Lock.db2 alternatives, and persists UNLOCKED separately by GUID without consuming the item or inventing loot. Missing/foreign/unlocked/non-lock templates reject; eligibility is rechecked at completion before the success receipt. Stored unlock state survives reopen and owner transfers without changing existing item-row encoding.

**Reference limit:** pinned TC CanOpenLock calls CalcValueAsInt without a caster. That code cannot certify Retail level-based lockpick thresholds. This implementation uses the normal caster-aware native effect value (RealPointsPerLevel1, SpellLevel14 for1804), not a guessed multiplier. Level29 versus native Lock3240.Skill15 is a source-derived implementation test, **not independently verified Retail threshold parity**. No trade-slot, cast-item-key, non-lockpicking item-type, game-object lockpicking or lockbox-loot certificate follows. Native1804 item-target RED55466315 -> GREEN9f2c5cbd: four controls prove accepted/unconsumed4638, reopen/recast rejection, level28 versus15 native skill threshold, foreign/missing/LockID0 rejection, and removal during the real1.5sec cast. Adjacent gathering10/10 and Disenchant8/8 remain GREEN. The native missing-item corpus remains conditional/partial, not a whole-ability success.

## Chrono Flames partial-mechanic follow-up

Native431442 overrides Living Flame361469 with431443. The replacement previously produced a cast receipt without its Fire/healing child. REDec0fb941 ->0b8990d2 exposes the next native boundary:361500 can be a delayed missile, so reading history from the root sees no current impact. The repeat now executes after the actual361500/361509 child lands, using native Arcane431583/431483 and existing copied-damage/fixed-heal paths without a second source multiplier or crit roll.

Only active learners record their own effective Damage/Heal by recipient and kind on the server clock. The native five-second window excludes unrelated actors/targets, overheal and overkill. Preservation uses431442 field0 (15%); other specs usefield2 (25%). Native DescriptionVariables852 defines SP*2.5*(1+versadmg); Overclock1260647 field0 adds40%, corroborated by the3.5SP variable. Missing scripted child records reject before payment. History is entity-local and bounded by that native window.

Atb4b88fa7, six real-ID controls pass: native Fire/Arcane replacement, owner/target exclusions, window expiry, both caps, Preservation effective healing versus overheal, and missing-child pre-payment rejection. Current-primary-hit inclusion and earlier Chrono copies are literal history semantics, not independently measured Retail timing/recursion parity. Full Lifecinders/Chrono variants, legal hero purchases and live Evoker proof remain open. Fresh source-gap and native-runtime census at42c85bc2 promotes six visibility memberships inherited from integrated master (Rogue56814 and Warlock126, three specs each), not six Chrono memberships. The inherited analyzer still lacks the explicit431442 numeric consumers despite their targeted behavioral proof. Compared with the user's253-broken milestone: working+6, partial+65, broken−71. Remaining182 broken are170 absent SpellStore memberships and12 missing native destination memberships (120145/344587/395277/1259190 in each Mage spec). Neither missing data nor native coordinate gaps are fabricated or erased. No generic Dummy no-op or blanket aura4 closure.

## Protective Growth named aura4 consumer

Native433748 is one of the highest-membership unresolved named aura4 roots (four Druid specs). Its literal12.1.0.69933 description binds the native8% field to the learner carrying their own8936 Regrowth. Native REDd10d193f shows incoming spell damage1000 rather than920 and a real melee swing10 rather than9. The consumer folds into existing per-school recipient multipliers when the aura set changes, covering spell and melee paths without a second damage hook. Foreign Regrowth, another recipient and absent talent do not activate it; cancellation and native expiry remove it. No unrelated Dummy fields are certified. GREEN43ee8117 and integrated8fcddcd9 both pass four native controls. The current source census registers only those exact numeric consumers, plus the independently tested Chrono/Overclock fields; ten memberships move partial→working at the probe boundary.

Private UDP5841 live runs use `fb_class2growth`/Fbgrowth and native Blackrock Invader42937. Both Modern and Forever show native8936 CastSuccess, self HoT/healing and incoming melee, with the authored player model attached. Exact8% mitigation is proved by the deterministic native spell/melee tests, not inferred from variable live NPC rolls. PNGs: `/syncthing/AgentShared/2026-10-11/class-completeness2/druid-{modern,forever}.png`; combat and client receipts remain in durable `live/druid-*-*.log`. Night-Elf texture receipt gaps were authenticated against the pinned local CASC root in a run-owned cache/product overlay; no canonical bytes, indexes or world.db were written. Own server/client/cage PIDs stopped after capture. This adds Druid's live activation evidence; the all13-class gate is still open.

## Lingering Healing named aura4 consumer

Native231040s2 extends Regrowth8936 only for self casts. Its independent index0 aura107/class-mask16 already extends Rejuvenation774; it does not match Regrowth's native mask64. Native REDced0f945 records6000 versus9000ms in the replicated aura and premature expiration. GREEN672a43a4 passes three Lingering controls and four adjacent Protective Growth controls. The named index1 consumer adds its native3000ms only for the caster's own Regrowth, after existing duration modifiers. Foreign casts and other recipients retain the native base duration; Rejuvenation's existing masked extension is unchanged. Optional percentage-duration interactions beyond this native control remain uncertified, not silently generalized.

## Fount of Strength named aura4 consumer

Native441675 already supplies20 raw Energy and200 raw Rage maximums through its two aura35 effects. Its index2 states that Frenzied Regeneration also increases maximum health by10%. Native RED6c23385f records public maximum health1110 rather than1221 during own22842. GREEN78ea6e06 passes three native health/resource/lifecycle controls. The named consumer folds into the existing max-health stat path only while the learner carries that aura; foreign recipients/casters do not activate it. Cancellation and native expiry restore health maxima, retaining valid current health and the existing native max-power bonuses. This controlled field/lifecycle proof does not certify other percentage-stacking or healing-snapshot interactions.

## AoE avoidance shared aura229

Native1966 Feint reduces area damage by its own−40% field; native1297073 supplies an independent−4% field. RED45f1900b records1000 versus600 damage. Implementation7af44e07/531a4005 reduces damage at the actual effect boundary rather than marking every sibling effect as AoE. Native school-mask overlap, multiplicative factors, cancellation/expiry, single-target sibling exclusion and real periodic ticks pass five controls atf90da7e3. Amounts and selectors remain authored; no guessed coefficients. Initial expanded fixtures omitted their native tick catalog/aura rows; those failures are fixture gaps, not new product regressions. Final current targeted log: slot-local `data/diagnostics/class-completeness4-2026-10-12/feint-controls2.log`.

Pinned TC a352b1fa `Spell.cpp::DoEffectOnLaunchTarget`, `SpellAuraEffects.cpp` periodic damage/leech and `Unit.cpp::CalculateAOEAvoidance` define per-effect area/cone or area-aura selection, TreatAsArea/TreatAsNPCAoe flags, school-mask factors and integer truncation. Periodic IgnoreDamageTakenModifiers bypasses avoidance. Flags and periodic leech are source-backed paths, not yet separately behavior-certified by these five controls. Creature-only avoidance aura310 and other Feint effects are not certified. Census regeneration follows this batch; whole class/talent acceptance remains open.

## Representative both-skin class gate — 2026-10-12

All13 representatives now have actual cast/mechanic receipts and inspected Modern/Forever PNGs. This closes only the representative-live gate, not the full13/all40 spell/talent goal. Current source/runtime census remains2889/4756/182. No coefficients or unavailable spell/destination rows were invented.

| Class | Native mechanic observed in both skins | Proof scope |
|---|---|---|
| Warrior |23881 damage,117313 heal,+80 raw Rage|private class4warrior, new native receipts|
| Paladin |20473→25914 self heal,+1 Holy Power|private class4paladin, new native receipts|
| Hunter |56641 damage,+20 Focus through77443|private class4hunter, new native receipts|
| Rogue |1966 Feint aura/cast|private class4rogue activation; exact40% AoE reduction has separate CPU proof|
| Priest |139 Renew aura and real140-point ticks|private class4priest, new native receipts|
| Death Knight |49184 Frost damage|private class4deathknight; optional incidental procs are not certified|
| Shaman |77472 self healing|private class4shaman, new native receipts|
| Mage |1449 area damage|private class4mage; observed resource events are not full charge-policy parity|
| Warlock |686 completed missile damage|private class4warlock, new native receipts|
| Monk |117418 cone damage and123154 aura|retained2026-10-11 proof, server88c21ec5/shared6ba2f5cd|
| Druid |8936 self HoT/incoming melee with433748 learned|retained2026-10-11 proof; exact8% reduction has separate CPU proof|
| Demon Hunter |162243 damage250/253,+25 Fury|private class4demonhunter, rebuilt engine9563568be|
| Evoker |431443 replacement→361500 Fire1139/1141+431583 Arcane285/285|private class4evoker;431442 granted, not legal hero-purchase proof|

All new runs use native level80 race-legal actors, private UDP5841, fresh redb, refreshed read-only canonical world.db copy plus authentic class summon closure, server174a2e84/shared2a48bdc. Earlier nine new classes used engine2908b5517; DH/Evoker used9563568be. Admin grants isolate mechanics, not talent legality. Setup refusals, dead-player attempts and incomplete-visual attempts are excluded. Player/equipment bytes were authenticated against the pinned local CASC root in an owned overlay, never relabelled or downloaded from CDN.

Requested26 PNGs: `/syncthing/AgentShared/2026-10-12/class-completeness/`; Monk/Druid retain their original earlier evidence dates. Exact logs, source authentication, input/action scripts and native builds: slot-local `data/diagnostics/class-completeness4-2026-10-12/`. Private servers/clients stopped; engine-s4 cache/products links restored to their original targets. Native zero-display gear bug and its RED/GREEN are documented in the [engine class proof](../../game-engine/docs/wiki/investigations/class-spell-live-proof.md#class-completeness-continuation--2026-10-12). Remaining spell/mastery/guardian/legal-build gaps stay open.

## Summon owner damage and raw mastery — batch18

Aura429 applies the current owner's authored percentage to controlled-pet and guardian direct spell/melee damage. Owner personal damage, foreign summons and copied source damage exclude this modifier; transfer/removal immediately stops the old owner's bonus. Native21563 Command,137012 Restoration and76657 Beast Mastery have real damage controls. Native137012 includes its authored RealPointsPerLevel term despite having no SpellLevels row; no coefficient was invented.

CharacterStats retains raw mastery points independently of mastery percent. Attr8 MasteryAffectsPoints adds BonusCoefficient × raw points before points modifiers. Self-owned flagged aura amounts refresh after owner stats change without recreating the aura. Native173184 mastery gain/removal changes actual pet damage;413984 timed metadata/expiry survives refresh. Both self and foreign413984 fixture recipients receive native effects at the production aura boundary because its area selector has MaxTargets1; this is refresh/expiry proof, not target-selection proof. Co-located recipients tied for the single slot after arena integration, so an explicit owner could not guarantee self application. TC a352b1fa Spell.cpp:1445–1450 randomly limits ordinary area targets; it likewise does not guarantee the explicit unit. Type540 gameplay and periodic tick-phase refresh remain uncertified.

At `a5de3082` / shared`00d33e7`, summon controls4/4, adjacent stats/itemstats21/21, AoE5/5, Chrono6/6 and Growth/Lingering7/7 pass warning-free. Initial missing schema fields were compile failures, not behavioral RED. Logs: slot-local `data/diagnostics/class-completeness4-2026-10-12/summon-damage-integrated-controls4.log` and `summon-adjacent-*.log`. No new live claim. Census at `a5de3082` / shared`00d33e7` records all7827 memberships:2897 working/4748 partial/182 broken. Eight memberships move to working;429 repairs also help roots that retain other gaps. The182 broken remain170 missing imported rows and12 Mage destinations; none were fabricated or removed.

Post-mainline fixture correction: `7892dbac` records a native blocked guardian swing13 versus the old unconditional18 assertion. Combat code did not change; +100 hit chance never excluded creature dodge/parry/block/crit. `d6ead749` validates32 actual swing receipts against their recorded outcome and native owner multiplier, and requires damaging hits rather than accepting avoidance-only evidence. Summon4/4 passes against shared`80bec38`; production combat/RNG is unchanged. Logs: `integration-guardian-{diagnostic,controls}.log`.

## Control of the Dream — batch19

Native434249's s1 caps unused-ready credit at15 seconds. The next cooldown of the named Druid major abilities subtracts that real elapsed time; active cooldown time does not count. Charge abilities count only time at full charges, using exact natural completion timestamps or the actual restoration timestamp. The credit shortens the next charge deadline, not the nominal rate or every subsequent queued recharge.

Native132158 Nature's Swiftness demonstrates ordinary cooldowns;194223 Celestial Alignment and390414 Incarnation demonstrate category2328. Native468743 Whirling Stars supplies the second charge and authored flat recovery adjustment: casting with one available charge does not accrue full-charge credit. Known-spell sync starts a newly acquired ability's clock at learning, preserves unchanged named aliases and discards lost abilities. Own talent removal and foreign copies grant no credit.

`f7a0f057` genuine RED60000 versus45000; `80c53fcc` genuine RED15000 milliseconds credited before ability learning. At `92dd4b92`, eight native controls pass, with adjacent charge-haste4/4 and cooldown-haste3/3 retained at their documented source scope. Logs: `control-dream-{red,learning-red,learning-green}.log`, `dream-*-controls.log`. At `578c18d7`, final dream8/8 and adjacent cooldown-haste3/3 pass after zeroed-clock simplification. Full batch19 census records all7827 memberships:2901 working/4744 partial/182 broken; four434249 memberships move to working. These prove cooldown/charge policy, not full Convoke/treant/Incarnation effects, legal talent purchases, persistence across reconnect or independently observed Retail timing.

## Blooming Infusion and consumed missile modifiers — batch20

Native429433 counts completed8936 Regrowth casts independently of78674 Starsurge/eligible191034 Starfall casts, using its authored s1=5. Starfall requires the named137013 Balance passive. The two counters grant native429474 offensive or429438 healing infusion. Native family masks and next-use semantics consume the corresponding buff;429438's options row has no proc charge, so its matching completed cast removes the one-use buff explicitly. Triggered copies, periodic ticks, cancelled/rejected casts and foreign talent copies do not count; unlearning resets counters.

`62de709c` genuine RED2 missing buffs → `1364d5ef` generation GREEN, then genuine delayed-Wrath RED60 versus122.44. The damage bonus disappeared because launch consumed429474 before missile impact read live spell modifiers. `95792364` retains already matched charged op/percent/amount values on the per-cast SpellInfo clone; live charged modifiers are excluded once that clone is frozen. Uncharged modifiers retain their existing live behavior. This is not a full damage/crit/stat snapshot or certified inheritance through independently constructed triggered-child contexts. Pinned TC a352b1fa Spell.cpp8569–8641 retains launch-target damage/healing before impact.

At `0a862575` / shared`00d33e7`, eight native controls pass: both counters, real instant/ordinary next uses, delayed impact after consumption, no retroactive new buff, cancellation/triggered exclusion, Balance-only mixed counting, removal/foreign/relearn reset and missing-child rejection before payment/counter change. Adjacent Stormkeeper3/3, Tidal Waves1/1 and Backdraft1/1 pass. Full batch20 census at `36758fce` records all7827 memberships:2905 working/4740 partial/182 broken; four429433 memberships move to working. Full Starfall/hero/loadout/PvP behavior and independently observed Retail proc timing remain open. Logs: `blooming-{red,green1,green2,controls2}.log`, `blooming-adjacent-*.log`.

### Symbiotic Relationship source gap

Native474750 names474754's10%/8% transfer fields and474760 heal child, but its tooltip does not settle raw versus effective healing. The fetched pinned TC a352b1fa `src/server/scripts/Spells/spell_druid.cpp` contains no474750/Symbiotic handler. The lead has no retained basis evidence: `HealInfo::GetHeal()` would mean raw, `GetEffectiveHeal()` effective. No basis chosen and no transfer implemented. Owned receipt: `data/diagnostics/class-completeness4-2026-10-12/symbiotic-source-gap.txt` and pinned `spell_druid.cpp`.

## Bonus-stat negation — batch21

Native aura467 modifies bonus stats without erasing create/base stats. Pinned TC a352b1fa Unit.cpp9882–9895 applies its additive percentage to TOTAL_VALUE, then the ordinary total-stat percentage; SpellAuraEffects.cpp3981–3995 validates Stats indices0–4. StatModifiers now retains a separate primary_bonus_pct vector. Existing spec-based gear selection is unchanged; the new field modifies selected gear plus flat aura bonuses before adding base stats and applying primary_pct.

Native162697/162698/162699/162701/162702/162700 cover Agility DPS, Strength DPS, Intellect DPS, Intellect healer, Strength tank and Agility tank. They suppress authored off-stat bonuses while preserving base stats and the replicated UnitStats sheet. Native13365/13366/13372 provide real five-point flat buffs, not invented coefficients.

`5c8f9734` genuine RED2: Hunter strength102 versus97 and intellect135 versus130. `b56afb05` fixes the shared stat fold. At `2fa57f74`, four controls pass, including all six native spec categories, removal, gear/flat/total-percent ordering and unselected/spirit fields; adjacent stats/itemstats21/21 and summon-damage4/4 pass. Pure calculator cases use explicit fixture operands, not additional production coefficients. Logs: `bonus-stat-{red,green}.log`, `bonus-stat-adjacent-*.log`. No new live or legal-build certificate. Batch21 census at `116d3eff` records all7827 memberships:2911 working/4734 partial/182 broken. Thirty-seven memberships gain the467 behavior; six tank memberships move to working, while31 DPS/healer memberships retain the separate466 bonus-armor gap.

## Physical base-armor percentages — batch22

Native aura142's Normal-school bit multiplies gear/base armor before adding stat-derived armor. Independent aura factors multiply. Pinned TC a352b1fa Player::UpdateArmor (StatSystem.cpp255–276) fixes this ordering; HandleAuraModBaseResistancePCT (SpellAuraEffects.cpp3663–3700) limits non-player application to pets. Ordinary NPCs/guardians do not gain base armor from this modifier. HunterPet covers the shared controlled-pet path; SummonedCreature additionally checks native CONTROL_PET properties. Physical spell and melee armor reads use the same live pet multiplier. Nonphysical base-resistance modeling remains explicitly unimplemented.

`49d0752a` genuine RED pet hit677 versus545.45 under native396571's75%. Earlier149-point baseline failures were fixture fidelity: caster UnitLevel was absent despite Character Level10, selecting ArmorConstant0 and the mitigation cap. The fixture now supplies both unit levels. `6209336c` repairs the shared armor paths. At `4c8a6992`, five controls pass: pet spell/melee damage, ordinary-NPC exclusion, native75%×200% factor composition with stat-armor ordering, removal and187827 timed expiry. Adjacent stats/itemstats21/21, physical spell1/1, player swings5/5 and Demon Spikes3/3 pass. An initial unqualified player-damage selector ran zero tests and is not proof.

Logs: `base-armor-{baseline-diagnostic,red2,green,controls}.log`, `base-armor-adjacent-*.log`. Batch22 at `1deb60e3` records all7827 memberships:2928 working/4717 partial/182 broken;17 memberships move to working while other142 roots retain their separate gaps. No new live, all-specific-script override or legal-build certificate.

## Minimum-speed floors — batch23

Aura305 uses the strongest positive native percentage as a floor after normalization and the strongest slow (pinned TC a352b1fa Unit.cpp8931–8943). Independent floors do not multiply; roots/stuns still prevent movement. Player run/flight/swim rates share this fold. Removal/expiry recomputes remaining rates without changing other speed mechanics.

`5ccfb332` genuine RED2: Ghost Wolf2645 produced0.65/0.5/0.65 instead of1/1/1; Chi Torpedo115008's isolated native floor produced the same slowed rates instead of3/3/3. `655be822` implements the floor. At `8002244a`, six native controls pass: strongest floor, normalization/slow ordering, root/stun, removal, timed expiry to the weaker floor, faster-speed preservation and foreign-recipient isolation. Expanded fixture compile errors were not behavioral RED. Chi Torpedo applies only its floor/normalization effects at the production aura boundary; this is not whole-dash certification.

Adjacent movement1/1, normalization1/1 and mount10/10 pass. Logs: `minimum-speed-{red,green,controls3}.log`, `minimum-speed-adjacent-*.log`. Non-player template-relative minimum-speed rates and backward movement remain uncertified; no new live proof. Batch23 at `1441a273` records all7827 memberships:2934 working/4711 partial/182 broken; six memberships become working at the controlled player probe boundary.

## Final armor factors — batch24

Pinned TC a352b1fa Player::UpdateArmor (StatSystem.cpp255–276) applies aura466 after gear/base percentages and stat-derived armor; independent factors multiply. HandleModBonusArmorPercent (SpellAuraEffects.cpp3973) recomputes armor on amount/application changes. Native161394 supplies75%,262496 supplies100%,238666 supplies−15% for8000ms. Native162697/162698/162699/162701 author zero for466; these remain zero, not inferred suppression or extra armor.

`147f0a3e` genuine RED: calculated213 armor versus372 under native161394. Its second stacking expectation initially omitted an already learned Tempered Scales passive and was corrected against the baseline; it is not separate valid RED. `8e0adc12` adds the final multiplier. Expanded `5bcd7c64` exposes genuine fractional-boundary RED355 versus356: stat armor was rounded before the final factor. `7d9397af` retains the fraction until final armor conversion.

Six controls now pass: base/stat ordering with actual incoming physical damage, independent positive/negative stacking, removal to remaining factors, native negative expiry, native zero class fields and fractional rounding. Adjacent stats/itemstats21, physical base armor5, bonus-stat4 and summon4 pass. Concrete calculator fixtures isolate armor arithmetic; only the native selected aura effects are applied, not unrelated ability scripts. Pet owner-armor inheritance, ordinary-NPC aura466 parity and whole abilities remain uncertified.

Logs: `bonus-armor-{red,green,controls-red,controls-green}.log`, `bonus-armor-adjacent-*.log`. No new live proof. Batch24 at `dfc71656` records all7827 memberships:2965 working/4680 partial/182 broken. All31 changed memberships belong to the four zero-valued class armor fields; nonzero shared behavior is separately proved by native positive/negative controls.

## Zero-amount linked auras — batch25

Pinned TC a352b1fa HandleAuraLinked (SpellAuraEffects.cpp5299–5335) serves284 and285: zero amounts leave child values authored, child target requirements/channel-provided unit masks select the caster, reapplication synchronizes stacks without recasting, and removal selects the child by caster. NeedsToBeTriggeredByCaster (SpellInfo.cpp1721–1754) fixes original-caster versus recipient ownership. Linked casts retain active hit depth rather than resetting recursion limits.

`37a22263` genuine RED3: Ironfur192081 retained8 instead of13 mastery points and declared child auras were absent. The earlier fixed-stat harness could not prove the mastery delta and is superseded. `3cbd7d4e` implements zero-amount links. At `3062ed26`, seven controls pass: native1251877 mastery5, recipient/original-caster ownership, native96493/96494 stack and clock preservation, channel98979/98973 caster selection, parent expiry, foreign replacement preservation, depth-limit exclusion and nonzero-conflict exclusion. Selected native aura effects are tested at production application/hit boundaries, not full-ability certification.

Nonzero285 fields stay explicit source gaps. Pinned TC would overwrite child basepoint0 with the parent amount; no newer approved source resolves changed signs/magnitudes. Runtime retains285 warning receipts; the source analyzer certifies only zero-valued native linked indexes.

| Parent→child | Parent amount | Native child basepoint0 | Pinned copy result / retained boundary |
|---|---:|---:|---|
|371905→400734 After the Wildfire|3|10|3; child Dummy magnitude replaced, action uncertified|
|388491→388496 Secret Infusion|4|0|4; child Dummy action uncertified|
|388511→388512 Overflowing Mists|2|0|2; proc/heal semantics uncertified|
|392221→392223 Waking Dream|8|−1000|+8; negative cast-time modifier becomes positive|
|471492→471491 Ursol's Warding|10|−1|+10; damage-taken reduction becomes an increase|
|427775→1224451 A Fire Inside|1|1|1; no numeric conflict, but nonzero linkage/child226 remain outside this zero-amount proof|

Adjacent Unending Resolve2, stats/itemstats21, Chrono6 and Blooming8 pass. Logs: `linked-aura-{red2,green,controls}.log`, `linked-aura-adjacent-*.log`; exact native rows in `linked-aura-native-effects.json`. No new live proof. Batch25 at `d41f51c2` records all7827 memberships:2969 working/4676 partial/182 broken; Ironfur's four build-union memberships become working at the controlled probe boundary. All six nonzero linked-root memberships remain partial. Existing child/script/proc gaps remain, not marker no-ops.

## Masked moving casts and channels — batch26

Pinned TC a352b1fa Unit::CanCastSpellWhileMoving (Unit.cpp3298–3313) treats330 as a matching-family/mask permission; the amount is not a speed, chance or percentage. Attr13 `0x08000000` prevents the aura exception. Spell::CheckMovement (Spell.cpp7175–7196) uses it both at preparation and while channeling; existing intrinsically mobile spells remain mobile.

`4f9ea446` genuine RED4: native Spiritwalker's Grace79206 still rejects moving Lightning Bolt188196, native Hover358267 rejects moving Disintegrate356995, and gaining Grace leaves the active cast's permission false. The initial import compiler error was not RED. `10ef9c2f` adds mask-based permission at admission/creation and refreshes active cast permissions on aura changes. `a2f3d5d7` splits the active-cast lookup and extends exclusions.

Eight controls pass: real moving Lightning Bolt damage, real moving Disintegrate channel/ticks, gain/removal/expiry during an active cast, foreign family, empty mask within the same family, foreign recipient and the Attr13 veto. The empty-mask and veto controls isolate native metadata on a real Lightning Bolt fixture; they are not native empower execution proof. Tests assert server CastState (the replicated component) and actual cast outcomes, not newly captured client-side replication/live UI. Hover applies only its330 field here, not dash geometry or complete ability parity.

Adjacent ordinary movement cancellation2, Disintegrate1, queue/override11 and Blooming8 pass. Logs: `cast-mobility-{red2,green,controls}.log`, `cast-mobility-adjacent-*.log`. Other movement-exception aura types, charged-buff consumption, empowerment, legal builds and native live movement remain uncertified. Batch26 at `8d143318` records all7827 memberships:2983 working/4662 partial/182 broken. Fourteen memberships move to the working probe boundary; remaining330 roots retain their separate gaps.

## Receiver mechanic durations — batch27

Pinned TC a352b1fa WorldObject::ModSpellDuration (Object.cpp1725–1789) applies `min(sum aura232, strongest negative aura234)` to finite harmful auras. SpellInfo.cpp2669–2680 combines the spell/category mechanic with actually applied effect mechanics. Native Attr7 NoTargetDurationMod, positive auras and permanent durations bypass target changes. Util.h72–87 truncates the integer percentage delta before addition; rounding the final multiplied duration is different.

`75d8daaf` genuine RED5: native205708 Chilled remains8000 instead of5600 under Unbreakable Stride400804,3200 under stacked232 fields,6400 under strongest234, and4800 under mixed types; the explicit8333ms integer fixture remains8333 instead of7500. `5622e628` implements the pinned rule. At `1da68d9d`, twelve controls pass, including receiver ownership, stacking/not-stack/mixed arithmetic, integer truncation, unrelated mechanics, removal affecting only new applications, replication-component duration and real expiry, positive/permanent exemptions and the native target-duration veto. Selected native effects and explicit metadata fixtures isolate this boundary, not full ability/PvP parity.

Oppressing Roar372048's17 authored +50 fields remain a named partial gap: pinned `min(50,0)=0` cannot implement a positive-only duration extension. Positive232 amounts still participate in the pinned mixed sum; no positive-extension or sign rule is invented. Imported287297/289186 use mechanic selector1825, which cannot fit the pinned64-bit mask; they stay explicitly unsupported without modulo/bitmask reinterpretation. Neither occurs in the current class closure. Source analysis certifies only nonpositive valid232 fields and valid234 fields; runtime warning receipts preserve the gaps.

Adjacent Growth/Lingering7, Polymorph13, moving-cast8 and linked-aura7 pass. Logs: `mechanic-duration-{red,green,controls}.log`, `mechanic-duration-adjacent-*.log`. This proves server state/replication-component output, not new client/live reception. Duration-by-dispel, full diminishing returns, legal talent combinations and whole Oppressing Roar remain outside this proof. Batch27 at `5e015838` records all7827 memberships:3000 working/4645 partial/182 broken. Seventeen memberships become working; all three Oppressing Roar memberships retain a232 source gaps and runtime warnings.

## Mainline integration — 2026-10-12

Server `40f54418` integrates `c9062b6e`; engine `a2b4c01e6` integrates `4f71f6142`; protocol advances to `80bec38`. Only wiki-index conflicts required resolution; both sides were preserved with the prescribed union tool. Cargo dependency/lock content matches mainline. Active leased slots remain retained for the open goal.

At server `d6ead749`/shared`80bec38`,129 targeted server cases pass, including corrected native guardian outcomes. The first `warlock_pet_tests` selector ran zero tests and is not proof; qualified `pet_control::warlock_tests` proves nine lifecycle cases. Server cargo check and workspace fmt pass. Engine core/appearance seven matching cases pass, extension cargo check runs through the maintained depot native command/locks, and changed engine Rust files pass rustfmt check. No new live/client proof or full suite was run. Per-command logs: `integration-oct12-*`, `integration2-oct12-*`, `integration-warlock-qualified.log`, `integration-server-{check,fmt}.log`, `integration-engine-{core-full,check,rustfmt}.log`.

Post-integration current-source census at `78413ce2`/shared`80bec38` records all7827 memberships, exit0,581.54s:3000 working/4645 partial/182 broken, with no status changes from batch27. Authentic source revalidation still finds zero names for the37 missing IDs and zero positions for the four Mage destination IDs. The full spell/talent goal is still open.

## Retained boundaries

At the seventh census, 170 memberships have no imported SpellStore row; 12 still lack authentic fixed destinations. Other selectors, ritual/controlled-pet/item-lock behavior, Dummy/proc consumers and talent legality remain missing. No memberships removed or coefficients invented. The earlier Monk evidence is retained in the completed representative gate above; Monk's native117418 cone damage363, target aura123154 and cast receipt were observed in both Modern/Forever on private UDP5841 (`fb_class2monk`). Native server88c21ec5/protocol6ba2f5cd and matching engine-s4 artifacts; own client/server PIDs stopped after captures. PNGs: `monk-modern.png`, `monk-forever.png` in the requested directory. These are cone-mechanic receipts, not whole Fists-of-Fury/channel/rotation parity.

Diagnostics, native source receipts, logs and frozen protocol snapshots: `game-engine/data/diagnostics/class-completeness2-2026-10-11/`. Handoff: `/home/osso/.worktrees/handoff-class-completeness2.md`. PNG destination: `/syncthing/AgentShared/2026-10-11/class-completeness2/`.

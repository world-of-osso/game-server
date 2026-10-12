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

CharacterStats retains raw mastery points independently of mastery percent. Attr8 MasteryAffectsPoints adds BonusCoefficient × raw points before points modifiers. Self-owned flagged aura amounts refresh after owner stats change without recreating the aura. Native173184 mastery gain/removal changes actual pet damage;413984 timed metadata/expiry survives refresh. The foreign413984 fixture applies native effects at the production aura boundary because its area selector has MaxTargets1; it is not foreign target-selection proof. Type540 gameplay and periodic tick-phase refresh remain uncertified.

At `a5de3082` / shared`00d33e7`, summon controls4/4, adjacent stats/itemstats21/21, AoE5/5, Chrono6/6 and Growth/Lingering7/7 pass warning-free. Initial missing schema fields were compile failures, not behavioral RED. Logs: slot-local `data/diagnostics/class-completeness4-2026-10-12/summon-damage-integrated-controls4.log` and `summon-adjacent-*.log`. No new live claim. Census at `a5de3082` / shared`00d33e7` records all7827 memberships:2897 working/4748 partial/182 broken. Eight memberships move to working;429 repairs also help roots that retain other gaps. The182 broken remain170 missing imported rows and12 Mage destinations; none were fabricated or removed.

## Control of the Dream — batch19

Native434249's s1 caps unused-ready credit at15 seconds. The next cooldown of the named Druid major abilities subtracts that real elapsed time; active cooldown time does not count. Charge abilities count only time at full charges, using exact natural completion timestamps or the actual restoration timestamp. The credit shortens the next charge deadline, not the nominal rate or every subsequent queued recharge.

Native132158 Nature's Swiftness demonstrates ordinary cooldowns;194223 Celestial Alignment and390414 Incarnation demonstrate category2328. Native468743 Whirling Stars supplies the second charge and authored flat recovery adjustment: casting with one available charge does not accrue full-charge credit. Known-spell sync starts a newly acquired ability's clock at learning, preserves unchanged named aliases and discards lost abilities. Own talent removal and foreign copies grant no credit.

`f7a0f057` genuine RED60000 versus45000; `80c53fcc` genuine RED15000 milliseconds credited before ability learning. At `92dd4b92`, eight native controls pass, with adjacent charge-haste4/4 and cooldown-haste3/3 retained at their documented source scope. Logs: `control-dream-{red,learning-red,learning-green}.log`, `dream-*-controls.log`. At `578c18d7`, final dream8/8 and adjacent cooldown-haste3/3 pass after zeroed-clock simplification. Full batch19 census records all7827 memberships:2901 working/4744 partial/182 broken; four434249 memberships move to working. These prove cooldown/charge policy, not full Convoke/treant/Incarnation effects, legal talent purchases, persistence across reconnect or independently observed Retail timing.

## Retained boundaries

At the seventh census, 170 memberships have no imported SpellStore row; 12 still lack authentic fixed destinations. Other selectors, ritual/controlled-pet/item-lock behavior, Dummy/proc consumers and talent legality remain missing. No memberships removed or coefficients invented. The earlier Monk evidence is retained in the completed representative gate above; Monk's native117418 cone damage363, target aura123154 and cast receipt were observed in both Modern/Forever on private UDP5841 (`fb_class2monk`). Native server88c21ec5/protocol6ba2f5cd and matching engine-s4 artifacts; own client/server PIDs stopped after captures. PNGs: `monk-modern.png`, `monk-forever.png` in the requested directory. These are cone-mechanic receipts, not whole Fists-of-Fury/channel/rotation parity.

Diagnostics, native source receipts, logs and frozen protocol snapshots: `game-engine/data/diagnostics/class-completeness2-2026-10-11/`. Handoff: `/home/osso/.worktrees/handoff-class-completeness2.md`. PNG destination: `/syncthing/AgentShared/2026-10-11/class-completeness2/`.

# Class completeness continuation

Verified: 2026-10-11. Full spell/talent goal remains open across all 13 classes / 40 specs; probe dispatch is not whole ability, legal talent build, or rotation certification. [Baseline](class-completeness.md), [contract](specs/class-completeness.md).

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

Atb4b88fa7, six real-ID controls pass: native Fire/Arcane replacement, owner/target exclusions, window expiry, both caps, Preservation effective healing versus overheal, and missing-child pre-payment rejection. Current-primary-hit inclusion and earlier Chrono copies are literal history semantics, not independently measured Retail timing/recursion parity. Full Lifecinders/Chrono variants, legal hero purchases and live Evoker proof remain open. Source census refresh follows the current implementation; no generic Dummy no-op or blanket aura4 closure.

## Retained boundaries

At the seventh census, 170 memberships have no imported SpellStore row; 12 still lack authentic fixed destinations. Other selectors, ritual/controlled-pet/item-lock behavior, Dummy/proc consumers and talent legality remain missing. No memberships removed or coefficients invented. Required live coverage remains open for twelve classes; Monk's native117418 cone damage363, target aura123154 and cast receipt were observed in both Modern/Forever on private UDP5841 (`fb_class2monk`). Native server88c21ec5/protocol6ba2f5cd and matching engine-s4 artifacts; own client/server PIDs stopped after captures. PNGs: `monk-modern.png`, `monk-forever.png` in the requested directory. These are cone-mechanic receipts, not whole Fists-of-Fury/channel/rotation parity.

Diagnostics, native source receipts, logs and frozen protocol snapshots: `game-engine/data/diagnostics/class-completeness2-2026-10-11/`. Handoff: `/home/osso/.worktrees/handoff-class-completeness2.md`. PNG destination: `/syncthing/AgentShared/2026-10-11/class-completeness2/`.

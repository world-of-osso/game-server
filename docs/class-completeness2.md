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

## Retained boundaries

At the seventh census, 170 memberships have no imported SpellStore row; 12 still lack authentic fixed destinations. Other selectors, ritual/controlled-pet/item-lock behavior, Dummy/proc consumers and talent legality remain missing. No memberships removed or coefficients invented. Required live coverage remains open for twelve classes; Monk's native117418 cone damage363, target aura123154 and cast receipt were observed in both Modern/Forever on private UDP5841 (`fb_class2monk`). Native server88c21ec5/protocol6ba2f5cd and matching engine-s4 artifacts; own client/server PIDs stopped after captures. PNGs: `monk-modern.png`, `monk-forever.png` in the requested directory. These are cone-mechanic receipts, not whole Fists-of-Fury/channel/rotation parity.

Diagnostics, native source receipts, logs and frozen protocol snapshots: `game-engine/data/diagnostics/class-completeness2-2026-10-11/`. Handoff: `/home/osso/.worktrees/handoff-class-completeness2.md`. PNG destination: `/syncthing/AgentShared/2026-10-11/class-completeness2/`.

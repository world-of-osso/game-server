# Class completeness continuation

Verified: 2026-10-11. Full spell/talent goal remains open across all 13 classes / 40 specs; probe dispatch is not whole ability, legal talent build, or rotation certification. [Baseline](class-completeness.md), [contract](specs/class-completeness.md).

## Census milestones

| Revision / dependency | Working at probe boundary | Partial | Broken | Proof |
|---|---:|---:|---:|---|
| 1efd4c1b / shared252f4099 | 2841 | 4607 | 379 | Baseline retained at its recorded revisions |
| 92afa752 / shared6ee323ea | 2841 | 4607 | 379 | All 7827 memberships, exit0, 734.82s |
| 8b7d4723 / shared6ee323ea | 2844 | 4687 | 296 | All 7827 memberships, exit0, 462.12s |

First census resolves target24 for native117418 in three specs; integrated master's native1804 adds three unsupported item-target26 failures. Net broken change is zero, not zero repairs. Second census repairs fixture fidelity: production already loads authored teleport destinations, while Cargo runs fixture startup from the crate directory, where relative `data/world.db` fails. Census now explicitly loads the same native destination/map data and supplies actor identity, facing, instance manager and ungrouped state. This is not newly implemented production teleport behavior.

## Shared mechanics and proof

- `a237459f`: native enemy cone24 follows caster facing, DB2 ConeDegrees and the target-slot radius. Native117418 RED2 unsupported-target failures → GREEN2: damage, front/back/side/range/friendly exclusions, facing and target cap. Unit-center geometry remains; combat-reach expansion is not certified.
- `404908ce` → `7baf6265`: native3567 destination RED missing row → GREEN authored MapTransfer destination. Far-teleport acknowledgment/arrival is not proved by this new test.
- `3b66c134` → `8b7d4723`: native363534 Rewind reproduces missing fixture group state, then GREEN alongside the city control. Incoming-damage history remains a separate missing mechanic.
- `50be4847` → `ca817b6c`: native212036 mass resurrection RED explicit-dead-target rejection → GREEN native corpse selection, one group-member offer despite distant ghost position, stranger exclusion, acceptance, and DB2-derived restoration percentage. Target119 uses stored corpse positions and party/raid membership. Corpse state lacks a separate instance key; cross-copy/cross-map ghost bodies remain uncertified. Census for this batch pending.

## Retained boundaries

At the second census, 170 memberships have no imported SpellStore row; 12 still lack authentic fixed destinations. Portal interaction, other implicit selectors, Dummy/proc consumers and talent legality remain missing. No rows removed or coefficients invented. Both-skin live proof, one fixed mechanic per class, remains pending; no realm binary or client started in this continuation.

Diagnostics, native source receipts, logs and frozen protocol snapshots: `game-engine/data/diagnostics/class-completeness2-2026-10-11/`. Handoff: `/home/osso/.worktrees/handoff-class-completeness2.md`. PNG destination: `/syncthing/AgentShared/2026-10-11/class-completeness2/`.

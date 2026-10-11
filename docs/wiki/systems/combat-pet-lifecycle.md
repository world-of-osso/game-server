# Combat pet lifecycle

Verified: 2026-10-11, pet-lifecycle server5af60fe/protocol203acd3. Requirements live in [pet lifecycle](../../specs/pet-lifecycle.md); this is bounded implementation/proof, not complete Retail parity.

## Durable state

`pet_stables_v2` stores five active records plus200 stabled positions. Records retain identity/name/level/family/stance, current health and optional hunter specialization. `pet_stables` remains read-only migration input: both the stance-bearing and older pre-stance encodings convert on first read, then the new table takes precedence. Legacy/new records without saved health initialize from actual pet stats at spawn; subsequent spawns preserve saved health (bounded by current maximum), including zero.

Family default specialization comes from the local `creature_family`/`chr_specialization` join, excluding PetOverrideSpec0x20 as in pinned TrinityCore DB2Stores. Demon records do not receive hunter specs. Changed pet health saves after combat ticks; the queued player logout/despawn boundary takes an immediate snapshot before marking the owner pending removal. Dismiss snapshots health before removing the world pet.

## Stable operations

`PetStableRequest` runs after NPC hello/gossip/close validation. Only an in-world Hunter connection with the matching StableMaster role can list or mutate its own stable. Reliable ordered replies carry actual records, family/name/icon metadata, imported non-override specs and unlocked Call Pet slots.

Swap preserves the summoned entity when moving among unlocked active positions; moving it into storage unsummons it while retaining health. Rename updates stored and live NPC identity; specialization updates the record. Abandon removes the selected record, not other owned pets. Lists do not rewrite unchanged storage. Same-slot moves are harmless. Closed interaction, invalid/locked positions, invalid names and unsupported specs fail without mutating records.

The real Erma menu9821 includes StableMaster option0 plus separate battle-pet healing options. Tests route the authored combat-pet option instead of assuming a single-entry gossip menu. HostClient replies are collected in First before Lightyear's Last clear.

## Proof and remaining scope

Baseline632f3aea restored both wounded23 and dead0 pets at full378. At8358c09 all11 hunter tests pass, including reopened-redb wounded/dead health and health-change→queued logout→flush without a fixed tick. Earlier live audit confirms hunter tame/relog/dismiss/call and native Imp summon/relog/menu-dismiss, but only the baseline live binary was exercised.

At5af60fe hunter15/15 pass without warnings: real NPC hello/gossip selection, stable→rename→spec→redb reopen→active→Call→abandon→reopen, active-slot ownership preservation, same-slot move and non-mutating refusals. Protocol203acd3 concrete operation/name/spec/dead-health wire roundtrip passes1/1. Current live stable UI/operations, corpse revive, numerical Mend/Feed, other demons and both-skin post-fix captures remain pending. No shared realm or world.db writes.

## Sources

- [Storage](../../../crates/server/src/persistence_pets.rs), [runtime](../../../crates/server/src/hunter_pet.rs), [stable operations](../../../crates/server/src/pet_stable.rs), [logout](../../../crates/server/src/networking.rs), [behavioral tests](../../../crates/server/src/hunter_pet_tests.rs).
- Installed Retail `Blizzard_APIDocumentationGenerated/PetConstantsDocumentation.lua`:5 active,200 stabled,205 total.
- Local world.db: Wolf family1→Ferocity74; override specs535/536/537 have flag0x20.
- Pinned TrinityCore a352b1fa DBCEnums/DB2Stores reference under game-engine/data/diagnostics/pet-lifecycle-2026-10-11/reference/.
- Proof ledger/logs: game-engine/data/diagnostics/pet-lifecycle-2026-10-11/; handoff `/home/osso/.worktrees/handoff-pet-lifecycle.md`.

## See Also

- [[warlock-pets]] — controlled demon lifecycle and power model.
- [[spell-casting]] — cast/effect ownership.

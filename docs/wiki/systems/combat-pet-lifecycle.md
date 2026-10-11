# Combat pet lifecycle

Verified: 2026-10-11, pet-lifecycle server8358c09. Requirements live in [pet lifecycle](../../specs/pet-lifecycle.md); this is bounded implementation/proof, not complete Retail parity.

## Durable state

`pet_stables_v2` stores five active records plus200 stabled positions. Records retain identity/name/level/family/stance, current health and optional hunter specialization. `pet_stables` remains read-only migration input: both the stance-bearing and older pre-stance encodings convert on first read, then the new table takes precedence. Legacy/new records without saved health initialize from actual pet stats at spawn; subsequent spawns preserve saved health (bounded by current maximum), including zero.

Family default specialization comes from the local `creature_family`/`chr_specialization` join, excluding PetOverrideSpec0x20 as in pinned TrinityCore DB2Stores. Demon records do not receive hunter specs. Changed pet health saves after combat ticks; the queued player logout/despawn boundary takes an immediate snapshot before marking the owner pending removal. Dismiss snapshots health before removing the world pet.

## Proof and remaining scope

Baseline632f3aea restored both wounded23 and dead0 pets at full378. At8358c09 all11 hunter tests pass, including reopened-redb wounded/dead health and health-change→queued logout→flush without a fixed tick. Earlier live audit confirms hunter tame/relog/dismiss/call and native Imp summon/relog/menu-dismiss, but only the baseline live binary was exercised.

Stable operations/UI, corpse revive, numerical Mend/Feed proof, other demon summons and both-skin post-fix captures remain pending. No shared realm or world.db writes.

## Sources

- [Storage](../../../crates/server/src/persistence_pets.rs), [runtime](../../../crates/server/src/hunter_pet.rs), [logout](../../../crates/server/src/networking.rs), [behavioral tests](../../../crates/server/src/hunter_pet_tests.rs).
- Installed Retail `Blizzard_APIDocumentationGenerated/PetConstantsDocumentation.lua`:5 active,200 stabled,205 total.
- Local world.db: Wolf family1→Ferocity74; override specs535/536/537 have flag0x20.
- Pinned TrinityCore a352b1fa DBCEnums/DB2Stores reference under game-engine/data/diagnostics/pet-lifecycle-2026-10-11/reference/.
- Proof ledger/logs: game-engine/data/diagnostics/pet-lifecycle-2026-10-11/; handoff `/home/osso/.worktrees/handoff-pet-lifecycle.md`.

## See Also

- [[warlock-pets]] — controlled demon lifecycle and power model.
- [[spell-casting]] — cast/effect ownership.

# Battle pets — Phase 1

## Contract

- Read BattlePetSpecies and companion names/displays from LOCAL CASC exports. Never manufacture missing species or display records.
- Import that export with `python3 scripts/import-battle-pets.py <battle-pets.json> <world.db>`. Runtime reads the required `battle_pet_species` table from `data/world.db`, shipped by `deploy.sh`, like the Toy catalog. Missing table fails startup explicitly; runtime never reads a slot-local JSON export.
- ItemEffect learn-spell entries for companion summon spells grant distinct owned instances with species, breed, quality, level, and persistent instance identity.
- Journal belongs to authenticated account, not active character, survives server restart, and permits at most three instances per species.
- Summon selects an owned instance and creates one replicated, owned, non-combat creature using species CreatureID/display. It follows with the existing Retail FollowMovementGenerator offset; replacing, dismissing, logging out, or leaving the owner's map removes it.
- Existing CollectionStateUpdate carries the journal and active instance; mount-only messages do not replace the journal. SummonPet uses the instance's 64-bit identity.

## Source companion-target identity

- [x] One runtime current-critter identity is distinct from owned journal instance identity, guardian/pet state and general summoned ownership. Register only source Companion-title minions; journal actors without that authored source must not be promoted automatically. Existing journal spawn behavior is preserved; missing summon metadata remains a named gap.
- [x] Source temporary summon slots apply across journal and legacy actors: equal positive slots replace, different slots coexist, slot0 claims no slot. Removing the latest critter clears its identity without restoring older owned actors; removing an older actor must not clear the latest. Removed journal actors also clear their active journal snapshot.
- [x] Source target90 requires the explicitly selected current critter, never caster/foreign/guardian substitution. Source target150 resolves the caster's actual current critter independently of another selection. Missing/removed/other-map critters select nothing; owned-instance records alone are insufficient.
- [x] Real journal requests and sourced legacy summon/replacement/natural-expiry/dismiss/map-exit/owner-teardown cases prove this state boundary. This does not admit unresolved companion-toy scripts or certify native rendering/wire-field parity.
- [x] DismissPet resolves the authenticated owner's authoritative current critter (journal or legacy), matching TC DismissCritter. It never substitutes an older journal tuple, selected foreign actor, guardian or source noncompanion. No current critter means refusal without removing those actors or deleting owned records.
- [x] Removing a current journal actor clears its matching active journal tuple/snapshot; removing a current legacy actor preserves any older journal actor/tuple and never restores it as current. Private journal replacement cleanup remains separate from RPC dismissal targeting.
- [x] Real current Yellow Balloon, legacy critter, older-journal preservation and foreign/noncompanion/guardian cases prove the dismissal contract. No protocol fields or invented UseToy toggle; native wire/presentation parity remains separate.

## Owned combat state

Owned health and selected ability per slot persist separately from journal identity/level/XP. Newly learned or legacy unmigrated pets initialize at calculated maximum health with the existing first-unlocked authored choices; later encounters consume persisted health and choices instead of resetting them. Battle completion saves health and progression together, including forfeits and disconnects. Injured/dead pets remain so across relog/server restart; no invented heal/revive timer or cost.

Owner-only journal detail snapshots carry health/max health/power/speed/next-level XP, selected abilities, and species options with actual icons/names/required levels. Ability-change requests validate authenticated ownership, one-based slot, species membership, level and queue/battle lock before persistence and acknowledgement. Refusals preserve state. PvE and PvP use the same owned state; only living pets can become active.

## Initial phase boundary

Turn battles, wild encounters, capture, rewards, and battle queue are Phase 2 or later. The existing simplified shared battle simulator does not establish Retail turn-engine parity.

## Verification

Transactional catalog replacement/preservation; process startup with only the shipped world.db and no JSON export; real UseItem/ItemEffect tests; species cap without consuming rejected item; disk database close/reopen and account isolation; owned follower display/ownership/dismiss/logout; protocol round trip; client journal tests and native captures in both art skins.

# Battle pets — Phase 1

## Contract

- Read BattlePetSpecies and companion names/displays from LOCAL CASC exports. Never manufacture missing species or display records.
- Import that export with `python3 scripts/import-battle-pets.py <battle-pets.json> <world.db>`. Runtime reads the required `battle_pet_species` table from `data/world.db`, shipped by `deploy.sh`, like the Toy catalog. Missing table fails startup explicitly; runtime never reads a slot-local JSON export.
- ItemEffect learn-spell entries for companion summon spells grant distinct owned instances with species, breed, quality, level, and persistent instance identity.
- Journal belongs to authenticated account, not active character, survives server restart, and permits at most three instances per species.
- Summon selects an owned instance and creates one replicated, owned, non-combat creature using species CreatureID/display. It follows with the existing Retail FollowMovementGenerator offset; replacing, dismissing, logging out, or leaving the owner's map removes it.
- Existing CollectionStateUpdate carries the journal and active instance; mount-only messages do not replace the journal. SummonPet uses the instance's 64-bit identity.

## Owned combat state

Owned health and selected ability per slot persist separately from journal identity/level/XP. Newly learned or legacy unmigrated pets initialize at calculated maximum health with the existing first-unlocked authored choices; later encounters consume persisted health and choices instead of resetting them. Battle completion saves health and progression together, including forfeits and disconnects. Injured/dead pets remain so across relog/server restart; no invented heal/revive timer or cost.

Owner-only journal detail snapshots carry health/max health/power/speed/next-level XP, selected abilities, and species options with actual icons/names/required levels. Ability-change requests validate authenticated ownership, one-based slot, species membership, level and queue/battle lock before persistence and acknowledgement. Refusals preserve state. PvE and PvP use the same owned state; only living pets can become active.

## Initial phase boundary

Turn battles, wild encounters, capture, rewards, and battle queue are Phase 2 or later. The existing simplified shared battle simulator does not establish Retail turn-engine parity.

## Verification

Transactional catalog replacement/preservation; process startup with only the shipped world.db and no JSON export; real UseItem/ItemEffect tests; species cap without consuming rejected item; disk database close/reopen and account isolation; owned follower display/ownership/dismiss/logout; protocol round trip; client journal tests and native captures in both art skins.

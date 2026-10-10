# Battle pets — Phase 1

## Contract

- Read BattlePetSpecies and companion names/displays from LOCAL CASC exports. Never manufacture missing species or display records.
- ItemEffect learn-spell entries for companion summon spells grant distinct owned instances with species, breed, quality, level, and persistent instance identity.
- Journal belongs to authenticated account, not active character, survives server restart, and permits at most three instances per species.
- Summon selects an owned instance and creates one replicated, owned, non-combat creature using species CreatureID/display. It follows with the existing Retail FollowMovementGenerator offset; replacing, dismissing, logging out, or leaving the owner's map removes it.
- Existing CollectionStateUpdate carries the journal and active instance; mount-only messages do not replace the journal. SummonPet uses the instance's 64-bit identity.

## Phase boundary

Turn battles, wild encounters, capture, rewards, and battle queue are Phase 2 or later. The existing simplified shared battle simulator does not establish Retail turn-engine parity.

## Verification

Real UseItem/ItemEffect tests; species cap without consuming rejected item; disk database close/reopen and account isolation; owned follower display/ownership/dismiss/logout; protocol round trip; client journal tests and native captures in both art skins.

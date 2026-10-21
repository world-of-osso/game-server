# Azure translocation books

Azure Vault books expose their authored gossip and move the reader's same-copy party through the named project policy `AZURE_TRANSLOCATION_BOOK_ENDPOINTS`. [Derivation and implementation](../wiki/systems/azure-translocation.md).

## What it must do

- [ ] Preserve source menu IDs, destination body text and the `Proceed onward.` choice for entries194602/194712/194713/194714/194715.
- [ ] Revalidate ordinary interaction range and map identity before applying the option.
- [ ] Move the reader and living same-copy party members, followers and companion pets to the qualified receiver-book spawn, without changing resources or admitting a different copy.
- [ ] For194715, use authenticated AreaPOI7116 inside Umbrelskul's nav component, as explicitly approved when no receiving book exists.
- [ ] Freeze the same directed teleport links in the simulator graph; actors cannot walk a teleport as an ordinary path or traverse it backwards.

## How it works

- [Azure translocation policy](../wiki/systems/azure-translocation.md).
- [Dungeon simulator](dungeon-simulator.md).

## Implementation inventory

- `crates/server/src/npc_interaction/azure_translocation.rs`: shared endpoint policy and native party teleport.
- `crates/server/src/npc_interaction/gossip.rs`: source book menus/text/options.
- `crates/server/src/npc_interaction.rs`: native validated option dispatch.
- `crates/server/src/dungeon_simulator/azure_extraction.rs`: frozen directed links from that same policy.

## Tests asserting this spec

- `npc_interaction::tests::azure_book_tests`: actual interaction/selection loopback, receiver positions, movement epoch, same-copy preservation and range refusal.
- `dungeon_simulator::graph::azure_follower_graph_covers_native_normal_population_and_has_an_authored_route`: exact population census and authored boss route.

## Known gaps (current cycle)

- [ ] `AZURE_TRANSLOCATION_ACTIONSET_DATA_UNAVAILABLE`: native gossip-to-spell bindings and server ActionSet definitions are absent. Endpoint policy is not a claim to have recovered them.
- [ ] Native Green proof and seeded traversal qualification.

## Out of scope

- Generic ActionSet execution or invented teleport destinations.

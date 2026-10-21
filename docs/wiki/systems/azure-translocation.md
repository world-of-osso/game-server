# Azure translocation

`AZURE_TRANSLOCATION_BOOK_ENDPOINTS` is an explicit project policy approved2026-10-21. Native gossip text identifies destinations, staged navigation identifies components, and receiver-book spawn positions provide endpoints. Missing server ActionSets remain a named gap, not fabricated spell behavior. [Contract](../../specs/azure-translocation.md).

## Derivation

Source chain: `tdb_creature_template_gossip` → `tdb_gossip_menu` → `tdb_npc_text.BroadcastTextID0` → `tdb_broadcast_text.Text`. Every option is source `Proceed onward.`/option0. The spell names are only `Translocate`; all eight descriptions/subtexts are empty. Option SpellID is NULL, and no book SmartAI or spellclick binding supplies it.

| Source entry / source GUIDs | Menu / broadcast | Authored destination text | Qualified receiver |
|---|---|---|---|
|194602 /29001868|28961 /228162|top ring within the Crystal Chambers|194618 /29001870|
|194712 /29002007,29002008,29002009|29009 /228160|middle ring within the Crystal Chambers|195434 /29001886|
|194713 /29001906|29011 /228350|Mausoleum of Legends Entrance|195432 /29001915|
|194714 /29001918|29012 /228351|bottom ring within the Crystal Chambers|199545 /29002013|
|194715 /29002014,29002015,29002016|29013 /228354|Sindragosa's Lament|AreaPOI7116; no receiver book exists|

Receiver positions come directly from `content_creature`/TDB source spawns. No offset is added. All13 book spawns are accounted for above (nine sending books, four receiving-only books).

Native `NavigationMap::path`143 candidate queries qualified these component sets:

1. Entrance100022, Leymor29001810, book29001868.
2. Books29001870,29002007,29002008,29002009.
3. Books29001886,29001906.
4. Azureblade29001975, books29001915,29001918.
5. Telash29001910, books29002013,29002014,29002015,29002016.
6. Umbrelskul29002017; no book.

Component IDs are diagnostic representatives, not authored Blizzard IDs. Full production source formation IDs remain graph identities. Walk edges never connect otherwise-disconnected components; the policy adds nine directed book edges.

## Final endpoint

Lead explicitly broadened the policy to permit an authored safe-location/trigger/POI inside the final component. Only2515 safe location100022 is the entrance; no2515 AreaTrigger or WaypointSafeLoc exists. Authenticated70448 `AreaPOI.csv` row7116 supplies WoW(-5364.33008,1035.76001,157.716995), WMOGroup67571. Native staged-nav query from that exact point to Umbrelskul succeeds with six corners.

The POI label is `The Arcane Conservatory`, a portal label, **not** evidence that its containing room has that name. Conversely7115's `Sindragosa's Lament` label is at height339.549, outside the final157.8-height component. Do not use the label alone as floor identity. CSV SHA256:`0fb24e7cafe4aeb0fa9c2a3d039b734ec66752f56b356e619ec63ed59bd35b03`.

## Fidelity boundary

Native Translocate spells384106/384120/384347/384348/384349/384350/385983/385990 use effect226, whose ActionSet IDs are917018/917035/917469/917471/917476/917481/920394/920403. No definitions or exact gossip-to-spell pairing were found. `AZURE_TRANSLOCATION_ACTIONSET_DATA_UNAVAILABLE` stays open. The explicit endpoint policy replaces neither the general spell engine nor unrelated gossip behavior.

## Proof ledger

- `41677f1ae`: actual native gossip/selection three REDs6.60s: menu0/no authored option/body. Range case reaches the same missing-menu boundary first.
- Graph missing-file RED → source census/authored four-boss route GREEN1.80s. Initial extraction215 packs (225 source actors plus a virtualPOI node),540 walk edges,9 policy links,179 hostile route steps. Source actors29001897/29001912 lack native nav reach and remain outside the authored route.
- Artifacts: `/home/osso/.worktrees/logs/stockade-simulator-2026-10-20/azure-book-components.json`, `azure-final-poi-endpoint.json`, `azure-books-red.log`, `azure-graph-extraction.log`, `azure-graph-green.log`. No live clear claimed.

## Sources

- Local authenticated build12.1.5.70448 `data/db2/12.1.5.70448/AreaPOI.csv` and its extraction receipt.
- Read-only `data/world.db`: named TDB/content tables above; source positions preserved.
- Qualified staged bake: `game-engine/data/diagnostics/follower-dungeons-2026-10-21/staging/navmesh/2515`; receipts in that diagnostic directory show byte identity with the private bake.
- Native navigation query implementation: `crates/navigation/src/query.rs`; full corridor required, nominal5yd vertical nearest extent, no incomplete-path acceptance.

## See Also

- [[dungeon-simulator]] — headless scripted combat qualification, not live3D acceptance.
- [[boss-scripts]] — native four-boss behavior.

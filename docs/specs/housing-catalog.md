# Build-qualified housing catalog

## Contract

- Import only local DB2 exports authenticated against the audit CSV/DB2 SHA-256 receipts and exact active Retail product/build/build key.
- Preserve every authored column and literal unknown field without reinterpretation. JSON field values retain the CSV representation.
- Key records by build key, DB2 table name and record ID. Keep older builds separate; replace one build atomically and idempotently without touching unrelated world tables.
- Preserve source FDID/layout/hash/row count, encrypted dropped-record counts and absent-table receipts. Missing source data is not a gameplay default.
- Catalog entries, StartingQuantity and raw plot Cost do not grant ownership, establish grant semantics, or establish currency units.

## Lead import (canonical database mutation; agents must not run)

From game-server after landing the housing-core importer:

```text
python3 scripts/import_housing.py --evidence /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/housing-core-2026-10-12/qualified-sources --db /home/osso/Projects/world-of-osso/game-server/data/world.db --build 12.1.0.69933 --build-key dcfc90fffd79ba00406ae46f5f657592
python3 scripts/import_housing.py --evidence /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/housing-core-2026-10-12/qualified-sources --db /home/osso/Projects/world-of-osso/game-engine/data/cache/housing.sqlite --build 12.1.0.69933 --build-key dcfc90fffd79ba00406ae46f5f657592
```

This combined export includes the original audit, authenticated cornerstones and catalog acquisition sources:47 tables/70,552 rows/54 unreadable records. It supersedes the older35-table and cornerstone-only imports; each import replaces this build's records, so do not run an older export afterward. The same command with an isolated output database verifies the export. Static catalogs are not proof of playable neighborhoods, Housing Dashboard, decor acquisition, editor or visitor replication.

### Lead-only preview asset publication

After landing the client WMO receipt support and closure correction, publish the34authenticated preview assets (2M2,8skins,6WMO root/groups,18BLPs;2,037,802bytes) with the maintained no-clobber importer:

```text
python3 /home/osso/Projects/world-of-osso/game-engine/scripts/import_model_asset_chains.py --data /home/osso/Projects/world-of-osso/game-engine/data --product wow --staged-manifest /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/housing-core-2026-10-12/core7/preview-model-assets-staged.json --resolution /home/osso/.worktrees/housing-core-evidence/housing-resolution.sqlite --fragment housing-core7-preview
```

Staged source paths are durable, isolated diagnostic copies, not pool-slot paths. Content-key MD5 and SHA256 authenticate every payload against the frozen resolution snapshot; source identity/hash is retained. The isolated importer proof produces34receipts/8skin aliases and includes actual WMO6429362's auxiliary texture5984930, previously missed by the MOMT-vs-pixel-ID closure bug. This command mutates canonical data/cache and is lead-only; agents must not run it. It does not change world.db, housing.sqlite, grants or permissions and is not a full no-install/housing-art certification.

## Housing map metadata

Load authored Map.db2 types 7 (House) and 8 (Neighborhood), retaining map2735/2736/2783 names and WDT FDIDs. They are not dungeons and generic continent/dungeon admission remains refused until the housing service authorizes a particular neighborhood/owner/visitor copy. Loading map metadata is not hosting playable housing.

Property copies are keyed by authored map ID and authoritative neighborhood/house identity. Housing authority must reserve entry for a character before ordinary teleport/arrival can resolve that copy; unreserved characters and reservations for another property are refused. Copies use the common instance-ID allocator but have no synthetic MapDifficulty, dungeon locks or reset schedule. A reservation allocates identity without initializing map content. The first ordinary transfer arrival or saved-map login initializes that copy's authenticated objects/creatures exactly once; later arrivals reuse the same population. This admission primitive alone does not prove playable housing.

No verified first-house exterior/interior grant source is available. Do not infer default grants from component membership, UnlockItemID=0 or the placeholder House row. Rank3 may use one bounded authentic vendor-item redemption path backed by world.db/TDB npc_vendor or an authored DB2 item effect (lead approval, 2026-10-11); never turn StartingQuantity into ownership without grant semantics.

## World of Osso policy: cornerstone plot claim grants its house

Approved2026-10-16 project policy, **not a Retail grant claim**: successful purchase/claim through the authored NeighborhoodPlot cornerstone grants the persisted House ownership record tied to that plot. Existing `purchase` commits payment and `OwnedHouse { plot_id, neighborhood_id, owner_character_id, account_id }` together, rejecting wrong-neighborhood, unaffordable, occupied and duplicate ownership. Claiming does not grant decor, a selected exterior component set or an initial room layout. `OwnedHouse.plot_id` is the runtime relationship; no missing static DB2 House template relation is fabricated. Native proof must walk from the authored neighborhood entry and interact with the actual cornerstone, not teleport next to it.

## Default unlock sources (not a starter layout)

The pinned Retail generated `PlayerHousingConstantsDocumentation.lua` names room UnlockedByDefault4, exterior-type UnlockedByDefault1, fixture IsDefaultFixture1/UnlockedByDefault2, theme UnlockedByDefault1 and wallpaper UnlockedByDefault1. Existing imported Flags fields identify14HouseRoom rows with default-unlock bit4,4default exterior types,17themes and17wallpapers. This narrows the prior source gap: default availability is sourced; first-house instance quantities, initial exterior choice/placement, starting room layout and level/budget thresholds are not established by those flags. Preserve unlock availability separately from storage/grants and assembly. Unknown NeighborhoodPlot vectors and unnamed GameParameter values remain uninterpreted.

Dye source contracts are also narrower than category matching: `HousingDecorSharedDocumentation.lua` says slots currently accept colors of any category. `Blizzard_HouseEditorCustomizationDyeTemplates.lua` charges one real DyeColor.ItemID per changed preview dye slot and validates cumulative item counts. These sources do not establish the native gradient/shader payload or a legal starter assembly. Do not invent dye items, enforce category restrictions absent from Retail, or substitute colors.

Catalog3D preview uses the existing M2/WMO loaders in an isolated world and does not mutate this database. Its pixel20/MOMT23 WMO appearance retains the primary renderer's black AO TODO; complete Retail AO remains a named source gap. The client contract documents the native framing and partial source boundary.

The [lead import](#lead-import-canonical-database-mutation-agents-must-not-run) remains unchanged: all these records already exist in the47-table export; agents must not re-import canonical world.db or housing.sqlite merely to decode their flags.

## Authoritative catalog coordinates

The server reads build-qualified NeighborhoodMap entries and NeighborhoodPlot cornerstone/teleport coordinates from the imported catalog. Supported shipping neighborhoods are map2735/2736; record4/map2640 has no matching Map.db2 row. Plot prices retain authored copper amounts under the Retail cornerstone MoneyFrame/GetMoney contract. Names may be authentically empty; do not generate neighborhood/plot names or grants. Missing catalog imports must produce an explicit unavailable error.

## Authored map ground

Deployment ground selection includes spawned maps, registered runtime HOSTED_CONTINENTS and Map.db2 House/Neighborhood types7/8 even without creature/gameobject spawns. The continent IDs come directly from the runtime registry in `crates/server/src/maps/instances.rs`; housing IDs come from the same map rows/type authority used by housing admission. No second housing/continent ID list exists in deploy.sh. Explicit-map and all-registered extraction keep their existing scope; missing ground remains visible in the file collector's report.

Neighborhood2735/2736 and interior2783 use their real WDT/ADT and placed WMO collision. No generated flat floor substitutes for a missing map or house grant. Explicit registered-map extraction uses the existing local-CASC ground walker, including maps with no NPC spawns:

```text
python3 scripts/extract_ground.py --world-db <housing-world.db> --cache <private-housing-assets> --engine <game-engine-checkout> --casc-local <asset-resolver>/target/debug/casc-local --failed <private-housing-assets>/ground-failed.tsv --map 2735 --map 2736 --map 2783
```

Opt-in `housing_ground_ -- --ignored --nocapture` with `GAME_SERVER_GROUND_DIR=<private-housing-assets>` checks complete root/obj0/WMO geometry, actual210/225/25WDT roots and real support at Founder's authored entry/cornerstone/plot teleport. Client textures/M2 receipts, LOS bake and both-skin rendered admission remain separate proof gates. `scripts/build-host.py --bin los-bake --features bake` builds the existing geometry baker through the maintained host helper; default server-tool builds do not include it. Bake each selected map privately with `los-bake --data <private-housing-assets> --world-db <housing-world.db> --out <private-housing-assets>/los --map <2735|2736|2783> --all --no-eyes`; this preserves existing player-eye data and does not publish to canonical assets. Map bytes alone do not grant exterior or interior components.

## Authenticated cornerstones

Use NeighborhoodPlot.CornerstoneGameObjectID only as a reference to the same-build GameObjects record. Preserve that object's actual Pos, quaternion, name, type48, display and GameObjectDisplayInfo bounds. NeighborhoodPlot.CornerstonePosition is a different authored marker; never substitute it for a missing object. First copy arrival spawns available source cornerstones directly from the authoritative housing catalog; generic world startup population replacement cannot discard them. Ordinary later arrivals do not duplicate them. Plot payment requires that exact object in the player's copy and the existing five-yard gameobject model-box reach check, including nonfinite refusal. Ordinary UseGameObject opens the cornerstone through the same living-player/range validation and authenticated account/character link; identical coordinates in another copy and unauthenticated connections are refused. Its HousingReply carries the inspected plot identity without claiming ownership.

Pinned local GameObjects/layout44BD453C supplies all55 Founder's Point cornerstones; Razorwind's55 referenced cornerstone records are absent. Its plot records remain browsable, but absent objects cannot be purchased through a fabricated marker. No first-house grant follows from either source.

The [combined lead import](#lead-import-canonical-database-mutation-agents-must-not-run) includes decoded local GameObjects841620 and pinned DBD revision07d008f28907c00827cacadf2d7c1a40f578b6a3 alongside the original audit and acquisition sources; hashes and rows are checked by the same importer. It neither invents TDB templates nor mutates unrelated world tables.

## Atomic ownership and basic storage

The housing persistence table records account ownership, paid plot claims, storage counts and stable placed-instance IDs independently of character serialization. Purchase and inventory copper debit commit in one transaction; competing buyers cannot both claim one plot. No first-house exterior/room/decor grants are inferred.

A bounded redemption candidate is authenticated by vendor252312/item245259/decor1693 (Small Val'sharah Bookcase), ExtendedCost0, item_sparse.BuyPrice500000 and collect spell1238340/effect324/MiscValue1693. Normal bag UseItem and the authenticated redemption request consume one owned Retail bag item and grant one stored copy atomically; neither creates vendor items nor waives acquisition costs. The server verifies the pinned HouseDecor.ItemID, on-use trigger, zero player condition and exact collect effect/decor linkage. Vendor provenance/cost is checked by the normal merchant acquisition path; collection requires ownership, not a vendor-only acquisition. Wrong build, altered linkage or absent source refuses collection. The collector now resolves every same-build HouseDecor.ItemID → ItemXItemEffect → on-use ItemEffect → effect324 target with zero player condition/base points, rather than a bookcase-only branch. Ownership remains an actual Retail bag item, not membership. Source census2722 matched links includes34 vendor items; quest/crafting/trade-acquired items use that same item collector without requiring a vendor listing. This does not manufacture quest/achievement rewards or recipes. Missing/nonzero-condition chains remain unavailable, not inferred. Bag updates and an authoritative owned-storage snapshot follow successful collection; repeating use of the consumed item cannot duplicate decor.

Only an owner inside that house edits. Place consumes stored quantity, remove restores it, move/rotate preserve quantity, nonfinite transforms are refused. Account ownership/storage/layouts survive database reopen. Visitors require explicit owner HouseAccess configuration, receive the same saved layout, cannot edit and never receive the owner's private account storage. Unset access remains unknown and grants no visitor permission. Anyone permits all visitors; No One clears all relationship flags; Limited permits only selected Neighbors/Guild/Friends/Party relationships. Neighbors own a plot in this neighborhood; Guild compares nonzero persisted guild IDs of the assigned owner and visitor; Friends uses the assigned owner's persisted character friends list and the real persisted visitor name; Party uses current server GroupRegistry membership (pending invites are not membership). No client relationship claim is accepted. Battle.net friend authority is not available in this server.

The runtime service binds requests to authenticated account/character/player links. Neighborhood entry reserves the authored copy before the existing transfer handshake. Purchase additionally requires authoritative presence in that neighborhood and within the server's 5-unit gameobject interaction distance of its cornerstone; nonfinite positions cannot satisfy proximity. Accepted payment updates persisted gold and the live replication queue. Plot lists overlay durable ownership without exposing account IDs. Home Interior admission uses authored NeighborhoodMap7/map2783 entry and house-owned copies, without granting exterior/room components. Missing first-house grant data remains explicit: the raw authored terrain is not claimed to be an assembled house. Persist the visited house identity and authenticated account independently of instance IDs; fresh saved-house login rechecks access, so revoked visitors are refused. LeaveHouse returns to that house's authored NeighborhoodPlot teleport.

Successful neighborhood transfers persist the authenticated character's map/neighborhood identity independently of process-local instance IDs. A fresh server resolves a character saved on that housing map into the same logical neighborhood copy; absent/mismatched admission is refused. The full wire/WorldPortAck purchase test closes and reopens redb and creates a fresh instance manager before exercising login admission.

Native controls, playable terrain, component placement, item collection integration, editor networking and visitor convergence still require end-to-end proof. Persistence/domain tests alone do not certify them.

## Behavioral proof

Focused `scripts/tests/test_import_housing.py`: exact authored fireplace/plot data; unknown-field retention; no ownership/currency invention; idempotence; unrelated table preservation; wrong build/key and altered CSV/DB2 refusal; transactional rollback across tables.

## Precision transforms and persistence schema1

Expert commits atomically retain full Y-up position, yaw/pitch/roll in radians and positive finite uniform scale. Owner/in-house authority and stored/placed conservation remain unchanged; accepted full transforms publish to same-copy observers. WMO decor retains its authenticated InitialScale (Retail NoWMOScale); no arbitrary outdoor bounds or scale extrema are invented. Basic edits preserve the other axes and scale.

Startup migrates pre-precision redb schema0 exactly once within initialization's transaction: preserve saved position/yaw and derive pitch, roll and scale from the same-build HouseDecor InitialRotation/InitialScale. Source degrees/Z-up convert to engine radians/Y-up; missing or invalid source aborts without replacing the row. Schema1 runtime has no legacy transform decoder. Reopening schema1 never resets later precision edits. No canonical world.db mutation is needed; source catalog promotion remains lead-owned.

## Decor-reward quest acquisition

Same-build HouseDecor item membership identifies existing TDB quest rewards, never grants them. Import the authored TDB objective14 as CriteriaTree, preserving its root/required amount/text. Only an unconditional CompleteAll4 root with one AcquireItem36 leaf and IsForQuest32 is enabled; unknown types, flags, timers/modifiers/world-state conditions or missing objectives block completion instead of falling through to empty legacy objectives. Successful ordinary paid merchant acquisition accumulates its actual delivered item quantity in the matching active quest; failed buys and other items give no credit. Normal quest turn-in, reward quantities and bag collection remain their existing authority paths.

Pinned Furniture Favor92572 uses root216882/leaf216883/criterion109153/item253168 and rewards one253173 (Meadery Storage Barrel9247). CPU source/credit proof is separate from live acquisition: TDB has no253168 vendor listing; source vendor rows239598–239600 point at unresolved VendorItemID252826. Never substitute253168 for that reference or invent stock/cost/conditions. Recipe spells1239278/1239321 lack SkillLineAbility entries; GnomishFence's collect effect has MiscValue0. Those missing contracts remain explicit acquisition gaps.

## Finder relationships and plot admission

ListPlots overlays Retail None/Stranger/Friend/Self ownership relative to the requester. Friend status comes only from the character's persisted friends list and the real owner character name; account-owned houses are Self even for another owned character. No client-supplied friendship grants access. Owner-saved HouseAccess relationships are checked for interior visiting and persisted visited-house relog. Finder VisitPlot admits the authenticated neighborhood copy and ports to NeighborhoodPlot.TeleportPosition without payment, ownership, decor or a fabricated reservation timeout. Neighborhood map-entry orientation follows the existing source return path.


### Dye authority boundary (server/wire only)

Retail HouseEditorCustomizationDyeTemplates.lua DyeCostIsValid/GetPreviewDyeInfos and generated C_HousingCustomizeMode define staging, explicit commit and one owned dye item per new/changed nondefault slot. Catalog binds literal DecorDyeSlot IDs to the actual decor and DyeColor IDs to ItemID; unknown/duplicate/wrong-decor slots refuse. Generated HousingDecorDyeSlot explicitly accepts colors of any category; category is labeling only, never an invented restriction. Unchanged slots spend nothing; clear returns default color and does not refund paid items. Preview/cancel never consume; commit atomically validates owning account/inside-house/current instance and current authenticated Retail bag quantities, writes dyes and consumes each changed slot's cost. Foreign-skin item definitions do not pay Retail costs. Inventory replication is queued only after acceptance.

A preview belongs to one connection and current player entity, not durable character/account state. Cancel or successful commit clears it; replacing the player on relog cannot restore/commit a prior player's draft. Only committed layouts replicate to observers. Schema3 preserves existing permission selections, full transforms and undyed counts exactly, adding no dyes or new decor grants. Paid dyes survive Remove→stored variant→reopen→explicit variant re-place. Stored totals include variant counts once. Default PlaceDecor with no variant ID selects only an undyed copy; another account cannot spend a private variant.

No native recoloring/variant-selection feature or live acquisition claim. Currently paid1693/2330 have no slots. Corrected source census: dyeable decor has direct RewardItem links in TDB tdb_quest_template (including92961→245384/decor374), despite no joins in the older quest_template used by the first census. Quest eligibility/physical delivery and native recoloring are not yet live-proved; absence from the older table is not a source gap. No dyeable-decor TDB vendor/crafting join was found. Nine source dye items/recipes exist, but gradient-to-model rendering is not authenticated. Concrete source/atomicity/reopen/wire tests establish backend authority only. Canonical DB/cache imports remain lead-owned; this slice uses already-qualified47table data and requires no new import.

### HouseSettings persistence/source boundary

Retail `Blizzard_HousingHouseSettings.lua` HouseAccessFlags, OnAccessTypeSelected and OnSaveClicked define the four Limited choices, Anyone/No One clearing behavior and explicit Save. SaveHouseAccess changes only house flags; plot/blueprint access and owner assignment stay unavailable and unchanged. Only the owning account saves settings. Schema0/1 permissions migrate once to the schema2 access representation retained in current schema3: unknown remains unknown, false becomes all false, true becomes Anyone with all four relationship flags selected as Retail does. Account IDs, paid plots, storage, complete transforms and visited identities are preserved; no legacy runtime access path remains. Reopen/relog rechecks current relationships, never a stored admission grant. Party settings persist, parties do not; current authoritative membership decides entry.


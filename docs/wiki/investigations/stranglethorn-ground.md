# Stranglethorn audit underground destination

Verified 2026-10-10. The achievements live audit fell because its destination used a legacy spawn height against Retail terrain, not because STV ADTs were missing. Server collision and teleport semantics are unchanged.

## Root cause

Audit snapshots in engine `data/diagnostics/achievements-2026-10-10/live-modern/` record WoW XY `(-11625.6,-54.5569)`, dead/falling client positions and an authoritative position restored to Z `11.0223`. Legacy `creature` guid532, Hemet Nesingwary Jr.715, is `(-11628.6,-54.5569,11.0223)`; the audit offset X three yards. Runtime `content_creature` guid20337067 instead places Hemet at `(-11653.4,-61.026,16.313)`.

Azeroth WDT775971 MAID tile `(row32,col53)` references root778052. It exists and parses; its terrain at the audit XY is **16.238565**, 5.216265 yards above the erroneous Z. The sampled high-resolution MCNK hole mask is zero. Shared collision correctly rejects a floor above step reach. Changing collision to lift underground teleports would hide the bad input.

**Correction:** use runtime `content_creature` NPC coordinates for live interaction audits, not legacy `creature` coordinates. For this exact offset XY, use Z16.23857. No database writes, asset extraction or server physics repair was necessary.

## Proof

Private UDP54136, fresh redb, account `fb_stvground`, prebuilt canonical binaries at8f1858e9; host root read-only through bubblewrap. Wrong Z11.0223 fell to serverY-10.5377 then-36.01769; corrected Z16.23857 stayed at16.23857 in both client and server, health46500. Nine Retail taxi destinations across Booty Bay, Grom'gol, Rebel Camp, Fort Livingston, Explorers' League, Hardwrench and Bambala stayed at supported heights. Late Hardwrench/Bambala health loss is recorded melee combat, not falling; dead Bambala samples are not positive survival proof. Real-tile tests separately assert reachable ground at all ten hubs, including runtime Hemet.

`stv_audit_teleport_requires_retail_ground_height` exercises actual admin set-position plus200 gravity ticks: the legacy underground destination falls; corrected destination remains supported/alive. This is a characterization regression of working physics, not a claimed production RED/GREEN fix.

## Root-tile census

Explicit ignored diagnostic `ground::stv_tests::census_all_map_wdt_root_tiles` opens world.db read-only and uses production WDT/root ADT+liquid parsing. Set `STV_GROUND_CENSUS` to an output JSON path, then run the filter with `--ignored --nocapture`.

Across1185 Map.db2 rows,729 available terrain maps reference44090 nonzero root-tile entries: **0 missing, 0 unparseable**. Map0:1176, map1:1002, map530:826, map571:1131; all zero gaps. Map2991:72, likewise zero gaps.228 global-WMO maps have no root ADT census. Remaining map metadata gaps:105 zero WDT FDIDs,110 absent WDT files,13 existing WDTs without MAID. Their unseen tile inventories are not certified. No root-absent ADTs were encountered; no CASC extraction was needed. Census excludes WMO geometry/obj/tex/LOD assets and does not certify those render/collision paths.

## Sources

- [Real-tile regression and census](../../../crates/server/src/stv_ground_tests.rs)
- [Ground loading](../../../crates/server/src/ground.rs)
- [Terrain parser](../../../crates/server/src/terrain_height.rs)
- Durable evidence: engine `data/diagnostics/stv-ground-2026-10-10/`: destinations, point snapshots, private-server.log, combat.txt, behavior-tests.log, all-map-census.json and census-summary.json.

## See Also

- [Content import](../systems/content-import.md) — runtime content views versus legacy reference rows.

# Follower navigation component splits

Read-only canonical full-map native-link investigation for six boss endpoints and Azure's qualified book boundary. No connector or profile change is justified merely by disconnected components, nearby doors, vehicle flags or a missing source row. [Continuation contract](../../specs/navigation.md); [navigation implementation](../systems/navigation.md).

## Native coverage

Both-direction native-link searches over every canonical tile find zero intersection between each endpoint component. Exact refs/vertices, closest coarse-polygon points and unchanged input hashes are recorded in game-engine `data/diagnostics/follower-nav-gorek-2026-10-22/component-investigation/`.

| Map / endpoints | Tiles / component polygon counts | Closest coarse gap | Physical-source verdict |
|---|---|---|---|
|2527/Halls189722,189727|40;5990→9968, same pair|5.422yd, mostly vertical|No sourced internal traversal object; custom exitAT10 touches entrance side only.|
|2526/Vexamus194181|12;1272→16949|94.804yd|Four doors are on boss side; exitAT8 touches entrance side. Recruiter SmartAI casts pledge auras, not movement.|
|2651/Darkflame208743,208747|100;4857→8159, same pair|24.000yd vertical|Doors touch one side each; CandlelightAT285 is encounter geometry. Vehicle flags/boarding spells do not provide a route/destination.|
|2661/Goldie214661|4;3203→872|9.845yd, mostly vertical|Brewery doors444272/444271 are14.130/15.111yd from entrance component but0.007/0.087yd from boss component. No crossing action.|
|2515/AzureRune29001884 or book29002007→receiver29001886/pack29001895|4;1520→1638|80.357yd, mostly vertical|Qualified native gossip-backed [directed book policy](../systems/azure-translocation.md) supplies the middle-ring crossing; do not fabricate an ordinary walk edge.|

Distances above are minimum polygon-to-polygon coarse-surface distances, not necessarily the authored route location. The source-object report separately measures every candidate position to **each complete component**, with native polygon refs. Raw collision windows contain real shallow/flat endpoint support faces and zero missing assets; local exact-vertex topology does not join those support faces even at55°or90°. This does not prove a full-map raw-geometry topology or authorize a slope change.

## Traversal-source pass

Build70448 AreaTrigger CSV and SQLite projection have zero map rows for the four boss maps; no `areatrigger_teleport` originates in or lands at their safe locations. The placed TDB GO inventory has no TYPE_TRANSPORT11/MO_TRANSPORT15 candidate. Dynamic/custom AT rows are the two dungeon exits and Darkflame Candlelight. Source absence remains a coverage limitation, not proof that no client-authored transport exists.

Door, vehicle and SmartAI rows were checked with literal source typing. Timed source_type9 action-list IDs are followed only through explicit action references, not numeric collisions with creature IDs. Academy's15SmartAI rows provide conversations/pledge buffs/close-gossip; Goldie's matched spawn reset casts454194(effect6/aura4). Boarding spell46598 has aura236 but no sourced cross-component destination. Pinned TC Darkflame scripts bind the Candle King encounter gate, with no traversal jump/teleport handler found. Ordinary GO door state is not an offline static bake input, so “door closed in bake” is not established by a GO state field alone.

## Authored WMO placements

Existing ADT/WMO readers resolve these measured boundary windows:

| Map | MODF placement / root FDID | Overlapping render-group FDIDs | Authored selected set |
|---|---|---|---|
|2527|33746373 /4325339|4532807|0|
|2526|36445174 /3947017;36447207 /4335681|3947700;4352531|0;1|
|2651|44107304 /5162358|5466061|0|
|2661|44593122 /5244088|5245896/5279094/5422559|0|

Render overlap is provenance, not collision classification. Current collision collection loads root groups through the existing ground-face reader and selects rootset0 plus the authored MODF set. No source establishes that enabling a different doodad set is the correct world state. No sourced destructible/trapdoor/phaseable GO crossing was identified. Detailed selected doodad IDs and object-anchor/component distances are retained in the evidence report.

## Final raw-support boundary classification

The final diagnostic measures raw collision support patches seeded at the corresponding native boundary, then expanded through exact shared raw edges with the existing slope50°/support-face selection. It does not substitute generated nav vertices or assign remote roof faces to a component by arbitrary proximity. Exact pairwise triangle distances and source vertices are retained in `raw-seeded-support-gap-results.json`.

| Named gap | Raw horizontal gap | Raw vertical step | Verdict |
|---|---:|---:|---|
|`FOLLOWER_NAV_COMPONENT_SPLIT_2527` (both bosses)|0.000843yd|4.954492yd|Exceeds1.6yd climb.|
|`FOLLOWER_NAV_COMPONENT_SPLIT_2526`|78.499696yd|51.230000yd|Exceeds clearance/step limits.|
|`FOLLOWER_NAV_COMPONENT_SPLIT_2651` (both bosses)|0.640860yd|23.956675yd|Exceeds1.6yd climb.|
|`FOLLOWER_NAV_COMPONENT_SPLIT_2661`|0.209433yd|9.942910yd|Exceeds1.6yd climb.|

No corresponding measured support pair is under climb; none proves a rasterization/missing-span/erosion defect. Radius0.388yd (two-cell erosion≈0.520833yd) is clearance, not permission to bridge a horizontal void. Height2.03128yd (eleven0.2yd cells) is headroom, not an allowed step. Small XZ separation between stacked levels does not imply walking connectivity.

An initial all-nearby-triangles/nearest-native-component assignment produced a false Darkflame zero gap on roof faces29–37yd from either actual component. It was rejected, not used to justify a bake change; final seeded support patches avoid that attribution mistake. This is a measured-boundary classification, not enumeration of every remote raw surface or proof of authored Retail travel intent. Full-map native disconnection and absence of a qualified source crossing remain established. Per lead decision, retain the four named gaps and stop corrective changes: no connector, slope/profile relaxation or further bake.

## Candidate experiment

The pinned TC low-hanging filter omission received a separate one-map private experiment; its [recorded negative Goldie outcome](../systems/navigation.md#low-hanging-filter-candidate--2026-10-22) does not establish these physical causes. No other-map bake, canonical publication or synthesized connector follows from this investigation.

## One-way collision-drop qualification

The subsequent lead-approved investigation qualifies `NAV_ONE_WAY_DROP_FROM_COLLISION` before any link/bake change: an upper-component walkable edge, directly-below lower component, clear vertical collision path, downward-only authored progression and existing fall damage. No connector was approved or implemented. The six-pair table and exact geometry live in the evidence directory's `one-way-drop-qualification-table.md`.

Complete native boundary projections find1866 Halls and1135 Darkflame samples; Academy/Goldie have none. Encounter rows and actual component attribution establish Irideus→Gulping/Khajin, entrance→Vexamus, Waxbeard→Blazikon/Darkness and I’pa→Goldie. These establish component direction, not an exact intermediate travel sequence. The nearest Halls/Goldie raw supports go upward in that direction, so unsigned nearest gaps cannot qualify downward travel.

Existing-reader expanded geometry export passes at acae0551c in0.37s: Halls1764/Darkflame4982triangles, zero collector problems. Windows cover native samples predicting less than100% ordinary maxHP fall damage, with3yd margin; they do not exhaust every possible raw ledge.235 sampled Halls raw-boundary projections all intersect collision above target level; the minimum native63.5yd ray hits an intermediate surface atY-9.485732. No direct body-clear drop is demonstrated there.

Darkflame's minimum24.2yd native projection lies inside upper floor next to2.634yd-high collision. A different raw top edge at(534.269,86.534424,110) has SOURCE support ref281475865905538. An outward shaft atXZ(533.88,110) reaches rawY56.281814 on TARGET ref281475865905401:30.252610yd. Conservative radius0.388/height2.03128 approach/shaft boxes intersect zero triangles. **This remains a physical departure candidate, not a qualified link:** the shaft is0.389yd outward; the literal raw-edge ray contacts supportY83.90045, static L-shaped probes do not prove a continuous falling trajectory, and nine landing samples do not prove exhaustive support. Replayable evidence: `drop_ledge_body_probe.py` and `darkflame-raw-ledge-body-probe.json`.

Existing `shared::movement::fall_damage_percent` predicts18.979% maxHP ordinary damage for that candidate, versus92.416% for the native Halls minimum. CurrentHP and modifiers matter. `knockback.rs::apply_landing_damage` returns for non-Player; networking landing handling is player-owned. The helper does not establish follower damage application or immunity. No fall-code change was authorized. All four named gaps remain; no geometry/navmesh/worldDB mutation or further bake follows from this table.

## Sources

- Read-only `world.db`: `tdb_gameobject`, templates, transports, `tdb_areatrigger`, `tdb_areatrigger_teleport`, safe locations, creatures, SmartAI, spellclick and native spell effects.
- Authenticated local build70448 `AreaTrigger.csv`; hash in `sourced-traversal-objects.json`.
- Native query/codec: `crates/navigation/src/component_diagnostic.rs`; existing collision readers: `crates/los/src/bake` through `boundary_geometry_diagnostic.rs`.
- Pinned TrinityCore a352b1fa generator, dynamic GO, transport and Darkflame scripts, saved verbatim under the parent evidence directory's `tc-a352b1fa-mmaps/`.
- Qualified Azure packet: `sim-azure-evade-2026-10-22/azure-walk-components-for-navigation.json`, graphSHA c9172a1299322569fa674652e37f719926f01191cf1c1fa489158308d42a68c8. It does not qualify the unrelated14305AFTER report.

## See Also

- [[azure-translocation]] — explicit book endpoint policy and remaining ActionSet gap.
- [[navigation]] — resource-limited partial continuation versus completed disconnection.

# Mythic Keystone runtime

Verified: 2026-10-11. The [contract](../../specs/mythic-plus.md) owns scope; [local source qualification](../../mythic-plus-data.md) owns catalog identities/numeric evidence. This implements the qualified Azure Vault loop, not complete Retail season/combat parity.

Future attributable completion records for the static armory are separate from the frozen wire/run ABI; see [[site-snapshots]]. Legacy anonymous history is not backfilled.

## Content

The offline catalog importer validates destination independence and six DB2 layouts before one atomic replacement. A separate copy-only item publication script projects existing Item/ItemSparse180653 into content_item. Neither writes canonical world.db.

`mythic_plus` reads the catalog at startup. Keystone metadata is redb-keyed by the actual item GUID; the normal inventory model supplies the bag item. An explicitly requested administrative grant is acquisition for the private fixture, not an invented dungeon drop rule. Reliable InstanceChannel requests validate authenticated current-character ownership, live Font bounds, same dungeon/copy and fresh Mythic difficulty. Start revalidates, converts the copy23→8 and depletes the stored key immediately, so abandonment/restart cannot duplicate an upgrade.

Each copy holds one run. The pure lifecycle reads injected shared ServerClock time, counts each qualifying creature entity once at its authored weight, retains scenario-event weights, consumes encounter DONE from the existing instance framework and records real death transitions. Azure bosses use the shared data-selected BossAI lifecycle. `encounter_compat.csv` preserves their four authored encounter IDs and saved ordinals when integrating the data-driven boss registry; no new combat spell numbers are authored. Only required bosses plus forces complete the run. Terminal key metadata and summary history settle in one redb transaction, idempotently by run ID. Final state stops accumulating time/kills/deaths.

Unsourced numeric fields default disabled/0; their names and boundaries live only in the contract. Positive seasonal periods index explicitly supplied epoch/rotation rows; no active season is presumed. Current-character changes on a persistent connection invalidate a previously slotted character's activation authority.

## Proof — 2026-10-11

- Behavioral RED/GREEN: pure clock/countdown/death/weighted creature+event criteria, terminal upgrades/overtime/abandonment, disabled policy, explicit rotation/minimum-floor policy; authentic SQLite catalog/Font authority; real connection character switch, out-of-range revalidation and reopened redb settlement.
- Private UDP5800 with independent imported world.db and fb_* accounts. Native physical Font click, bag pickup/slot click and Activate in Modern and Forever; real server snapshots, no previews.
- Modern final UI revision00e308e8c: run2,419/410 forces, all four encounters,85000ms, key8→11/upgrade3. Earlier run1 independently proved two Crystal Fury187160 kills→10 forces, deaths2/time lost0 and267000ms completion.
- Forever same revision: run3,424/410 forces, all four encounters,2492000ms, key8→7/upgrade0. Existing admin clock-advance2251 provided accelerated-clock overtime, not37.5minutes of wall-clock proof. Administrative Damage used reported NPC max health through production death/BossAI; Revive kept the test actor alive among the authored population.
- Evidence: canonical engine data/diagnostics/mythic-plus-2026-10-11/live/{modern,modern2,forever}/ contains PNGs plus receipts/logs. Only inspected PNGs are published to /syncthing/AgentShared/2026-10-11/mythic-plus/. None of this claims full-suite, independent-review or natural-party combat proof.

## Boundaries

Scenario-event92 weights are preserved/testable but producers are not connected. Missing Azure appearance receipts remain separate from authoritative kill progress. Full affix art/combat, next-dungeon season pool, key-level combat scaling, source minimum key floor and Azure boss mechanics remain explicit contract gaps. NPCs instantiated before activation keep their baseline authored Mythic stats. Both-skin UI implementation is [client-owned](../../../../game-engine/docs/specs/mythic-plus.md).

## Sources

- [Runtime adapter](../../../crates/server/src/mythic_plus.rs), [pure lifecycle](../../../crates/server/src/mythic_plus_runtime.rs), [catalog](../../../crates/server/src/mythic_plus_catalog.rs).
- [Azure encounter identities](../../../crates/server/src/scripted_ai/encounter_compat.csv), [encounter DONE hook](../../../crates/server/src/scripted_ai/instance_script.rs).
- [Source qualification](../../mythic-plus-data.md), [contract](../../specs/mythic-plus.md), [catalog importer](../../../scripts/import_mythic_plus.py), [item projection](../../../scripts/import_keystone_item.py).

## Party key sharing

The built-in [Mythic Mentor contract](../../specs/mythic-mentor.md) owns party summary visibility and bag authority. Producer: [mythic_mentor.rs](../../../crates/server/src/mythic_mentor.rs). Final native/private proof and screenshots are [client-owned](../../../../game-engine/docs/wiki/systems/mythic-mentor.md); server6a07a86d's targeted persisted-bag/roster/recipient/raid test passes. No addon messaging or peer bag GUID disclosure. Follower-party integration reads grouped names/classes from the registry's roster identity, not character storage: companions use runtime-only IDs and have no bag keys. A real companion-party regression reproduced the old character lookup panic, then verifies five rows with all four NPCs having no key.

## See Also

- [[content-import]] — authored population; not duplicated here.
- [[boss-scripts]] — shared encounter lifecycle, not fabricated Azure combat scripts.

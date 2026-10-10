# Retail item scrapping

The Scrapping Machine accepts bag items for material recovery. Server source: `crates/server/src/scrapping.rs`; [evidence and current boundary](../wiki/systems/scrapping.md). This implementation is **preflight only**, not completed scrap gameplay.

## What it must do

- [ ] Accept one through nine distinct owned bag-item GUIDs; reject empty, duplicate, oversized and foreign-GUID requests without changing inventory.
- [ ] Require a living player in the machine's map copy and model-box interaction reach.
- [ ] Recognize Scrapping Machine UI-link spawns from active world content.
- [ ] Check Retail catalog identity and the ItemSparse scrappable flag; reject Worn Shortsword25 with Retail `ERR_CANT_BE_SCRAPPED`.
- [ ] Keep inventory and collection unchanged when authentic loot is unavailable; distinguish this content failure from a Retail eligibility rejection.
- [ ] Preserve bind-time collection across database reopen. Do not infer unbound collection from disenchant rules.
- [ ] Destroy eligible bag items and grant authentic scrap loot in the same inventory transaction as applicable appearance receipts. Positive proof must use real item IDs and authentic loot rows.

## How it works

- [Server preflight, Retail sources and blocked execution](../wiki/systems/scrapping.md).
- [Atomic inventory/appearance receipts](../wiki/investigations/appearance-acquisition.md).

## Implementation inventory

- `crates/server/src/scrapping.rs` — internal requests/responses, machine/item preflight, explicit unavailable-loot refusal.
- `crates/server/src/game_objects.rs` — load and mark Scrapping Machine UI links.
- `crates/server/src/main.rs` — install server preflight plugin.

## Tests asserting this spec

- `crates/server/src/scrapping_tests.rs` — real IDs154840,168970,25; TDB spawn501905; internal request/error, selection/reach and database-reopen boundaries. No successful scrap-loot test yet.

## Known gaps (current cycle)

- [ ] Obtain authentic scrap loot and its item/machine selection rules. No source found in inspected local DB2/listfile, world.db or TDB1210 world SQL; never substitute disenchant loot or guessed materials.
- [ ] Implement destruction/rewards and atomic collection receipts once that source is established.
- [ ] Provide a reachable Skyborne machine spawn; active content currently has none.
- [ ] Add protocol requests/responses, interaction role/open-close lifecycle and owner updates in lead-assigned protocol/client slots.

## Out of scope

- Protocol and engine changes in this server slot, per task assignment.
- Timerunning-specific reward policies: sharing Retail's scrap UI link does not establish BFA loot semantics.

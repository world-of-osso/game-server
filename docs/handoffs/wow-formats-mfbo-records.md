# MFBO signed-grid consolidation

Approved scope: shared minimum-grid projection in `wow-adt`, replacing only server `ground.rs::load_terrain`'s inline signed-grid loop. Client full bounds reuse the same grid decoder. Keep first-MFBO server versus last-MFBO client, exact error labels/order, unused tails, plane geometry/float order/coordinates/defaults/falling/MH2O untouched. No BLP/assets/deployment.

## Before/after contract

Test-only `ground::mfbo_receipts_tests::` invokes actual filesystem `load_terrain` and the unchanged full client loader. Required roots779167/778027/776382 and existing texture-shadow companions779170/778030/776385 are present; using the client's existing companions adds no prerequisite to the new record projection.51 real-root-derived synthetic cases cover absence, every0–35B short record, exact36B, unused tails, signed extrema, distinguishable duplicate records and combined-invalid root/MCVT order.

`WOW_FORMATS_MFBO_MODE=capture|replay`, `WOW_FORMATS_MFBO_OUTPUT=<empty dir>`, replay `WOW_FORMATS_MFBO_REFERENCE=<old dir>` and `GAME_SERVER_GROUND_DIR=<engine data>` select receipt I/O only. Replay reads frozen input copies/XY bits, never regenerates coordinates. Default mode writes no persistent receipts.

12B magic/version/count envelopes. MFSR: fdid u32/presence u8/9i16 minima(23B/record;3real roots81B). MFCR: fdid/presence/18i16 max-then-min(41B;3roots135B). MFBE holds the exact length-prefixed old error. MFBR stores selected raw payload presence/length/bytes independently. MFBQ input is mapID+WoWXY float bits12B/record; MFBO query outcome appends explicit-tagged Option<f32>,17B/record. None payload is zeroed; no min-height sentinel or float tolerance.

Queries call actual `GroundMap::min_height` on a legitimate warm cache populated with actual file-loader terrain+MFBO. It has no lazy neighbour/WMO source because those collision faces do not determine minimum planes; inside/outside/unknown-map outcomes remain real method results. Eight triangle interiors, nine vertices, diagonals and ±1/512 edge probes are frozen once. Retained real-map/fall tests separately cover the full canonical cache boundary. Full inventories/counts/sizes/SHA-256/bytes and unchanged input hashes gate acceptance; repeat current terrain/MH2O receipts and Stockade4LOS+4NAV as pin guards, not substitute MFBO proof.

## Ledger

Baseline pending. Servercd4553fd5,enginec177af21d,library2a91b224,protocolf9a91753bbbc15ad7ff7392955b52c4da989d22e. Test-only harness precedes production edits. Library targeted RED→GREEN/default/no-default/check/fmt/push first, then both pins and paired consumer gates.

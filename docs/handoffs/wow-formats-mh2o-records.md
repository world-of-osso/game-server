# MH2O raw-record consolidation

Approved scope: shared12B chunk,24B instance,16B attribute projections and `wow-chunk` scalar reads. Client/server keep their differing bitmap and vertex layouts, LiquidObject interpretation, validation/error precedence and sampling/status policy. No mode flag, fallback, reconciliation, BLP or deployment.

## Frozen proof

Before production edits, `cargo test -p los --features bake --test mh2o_receipts` exercises both current consumers on required ADTs778027/778232/776382,25 concrete synthetic payloads and actual `Baker::collect_water` outputs. `WOW_FORMATS_MH2O_MODE=capture|replay`, `WOW_FORMATS_MH2O_OUTPUT=<empty dir>`, optional `WOW_FORMATS_MH2O_REFERENCE=<old dir>`, and `GAME_SERVER_GROUND_DIR=<engine data>` select test-only receipt I/O. Replay loads frozen synthetic inputs/query bits; it never regenerates queries.

Little-endian12B magic/version/count envelopes: WMSL server chunks (index/count/deep16B), layers(type/min bits/rectangle/exists/height-count22B +4B/height); WMCL client chunks(index/count/attributes-tag/fishable/deep25B), layers retain all type/object/min/max/rectangle/mask fields and length-prefixed heights/UVs/depth/raw-object bytes; WMER exact existing errors. WMRW inventories accessed source records on successful inputs. WMQI frozen chunk-index/XY-bit records12B; WMQO same inputs+tagged12B liquid outcomes24B. WMBN is24B per actual lower/upper bound pair. Client and server receipts compare only against their OWN baseline, never each other.

The existing terrain-height receipt harness also includes776382 for actual terrain/depth/area/Bevy query outcomes. Fresh Stockade4LOS+4NAV guards pin changes but does not prove water decoding. Require matching complete inventories/counts/sizes/SHA-256/actual bytes and unchanged source hashes. No missing-asset skips or zero-test probes count.

## Ledger

Baseline test-only source7064701bb: MH2O receipts2/0 (both consumers,25 synthetic cases), terrain_height23/0 and fresh Stockade4LOS+4NAV0. Complete180-file inventory28,034,481B/116 source hashes at `/home/osso/.worktrees/logs/wow_formats-phase5-reference-receipt.json`. Real server layers42/39/256; water queries6427/5968/39169; actual water bounds1181/1470/16384 records. Each ADT256 height chunks154636B and135428 frozen terrain/liquid queries4739992B. No production edits preceded capture. Server baseba74ef1df; engine basecc4858b97; protocolf9a91753bbbc15ad7ff7392955b52c4da989d22e; libraryed180ce. Test-only fixtures precede all production edits.

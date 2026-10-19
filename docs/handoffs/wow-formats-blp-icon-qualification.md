# BLP icon qualification — before-only

Approved scope is observation only. No production logic, assets, dependencies, pins or fifth crate change. The engine's BLP normalization must not become the server's behavior.

`site_snapshot_catalog.rs::icon` prefers an existing bundle entry, then decodes/re-encodes local PNG, then directly loads/converts local BLP using image-blp1.2 and encodes PNG using image0.24.9. Absent FDID or absent files returns None. BLP errors are prefixed `icon {id}:`; the engine instead strips mip flags and normalizes palette/alpha/BC5. Each consumer must retain its own outputs, errors and selection precedence.

The only change to `site_snapshot.rs` is an include inside its existing cfg(test) module. `site_snapshot_blp_qualification.rs` calls the actual `catalog::icon` function with temporary symlinks to frozen inputs. It saves complete PNG output and decoded RGBA for comparison, exact errors and panic messages. Five separate boundary observations cover absent FDID, missing files, cached bundle, PNG preference over invalid BLP, and invalid preferred PNG refusal. No production path is replaced or copied.

```text
WOW_BLP_INPUTS=/home/osso/.worktrees/logs/wow_formats-phase7/inputs.json
WOW_BLP_ICON_OUTPUT=/home/osso/.worktrees/logs/wow_formats-phase7/icon
agent-run wow_formats cargo test --locked -p server --bin game-server-site-export site_snapshot::tests::blp_qualification::capture_icon_before_only -- --ignored --exact
```

Run sequentially with engine qualification after normal capacity ADD. Inputs/complete receipts and source/hash manifests live under `/home/osso/.worktrees/logs/wow_formats-phase7*`; no authored assets are copied/modified. Observer is ignored by default and requires explicit paths/new output directory. Outcomes are before-only, not a claim of decoder correctness or cross-consumer equivalence.

## Captured results

Observer1/0 at formatter commitf6a571dbc.202 common inputs plus5 source-selection cases yield32success/166error/7panic/2None. The9 authored cases yield7success/2BC5errors. NoneFDID,missingfile,bundlehit,PNGpreference and invalidpreferredPNG refusal are all asserted against the actual icon function. Inventory64files4,123,240B includes full PNG and RGBA payloads, exact errors/panics and metadata; no output is approximated.

`/home/osso/.worktrees/logs/wow_formats-phase7/before-receipt.json` contains complete inventory hashes, input/source audit and cross-consumer differences. All202 input hashes plus5 production-decoder/lockfile hashes remain unchanged. Engine CPU/GPU independently record202 cases each; comparing CPU/icon reveals16 divergences, including authored zero-alpha/palette textures and BC5 refusals. Shared malformed cases retain7 icon panics rather than being silently converted to errors.

Recommend no fifth crate or icon migration: common parsing is already the shared image-blp1.2 dependency. The engine's extra normalization/BC5/Godot upload behavior stays local, while this icon function stays byte-for-byte unchanged. The engine [qualification ledger](../../../../game-engine/docs/handoffs/wow-formats-blp-qualification.md) describes the smallest-boundary proposal. No extraction is authorized by these results.

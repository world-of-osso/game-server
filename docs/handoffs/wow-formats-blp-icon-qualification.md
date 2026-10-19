# BLP icon qualification — before-only

Approved scope is observation only. No production logic, assets, dependencies, pins or fifth crate change. The engine's BLP normalization must not become the server's behavior.

`site_snapshot_catalog.rs::icon` prefers an existing bundle entry, then decodes/re-encodes local PNG, then directly loads/converts local BLP using image-blp1.2 and encodes PNG using image0.24.9. Absent FDID or absent files returns None. BLP errors are prefixed `icon {id}:`; the engine instead strips mip flags and normalizes palette/alpha/BC5. Each consumer must retain its own outputs, errors and selection precedence.

The only change to `site_snapshot.rs` is an include inside its existing cfg(test) module. `site_snapshot_blp_qualification.rs` calls the actual `catalog::icon` function with temporary symlinks to frozen inputs. It saves complete PNG output and decoded RGBA for comparison, exact errors and panic messages. Five separate boundary observations cover absent FDID, missing files, cached bundle, PNG preference over invalid BLP, and invalid preferred PNG refusal. No production path is replaced or copied.

```text
WOW_BLP_INPUTS=/home/osso/.worktrees/logs/wow_formats-phase7/inputs.json
WOW_BLP_ICON_OUTPUT=/home/osso/.worktrees/logs/wow_formats-phase7/icon
agent-run wow_formats cargo test --locked -p server --bin game-server-site-export site_snapshot::tests::blp_qualification::capture_icon_before_only -- --ignored --exact
```

Run sequentially with engine qualification after normal capacity ADD. Inputs/complete receipts and source/hash manifests live under `/home/osso/.worktrees/logs/wow_formats-phase7*`; no authored assets are copied/modified. Observer is ignored by default and requires explicit paths/new output directory. Outcomes are before-only, not a claim of decoder correctness or cross-consumer equivalence. Results/split approval pending capture.

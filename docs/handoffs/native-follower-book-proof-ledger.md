# Native follower starter book — active

## Goal

Ordinary NPC followers and the native kit exporter must consume one literal `starter_purchases` / `build_config` computation for the requested NPC specialization at levels1–90. Populate `KnownSpells` and `TraitLoadout` from that result; controller action aliases must not grant spells. No `Player` entity or component, player specialization fallback, classifier, variant resolver, canonical DB edit, or unrelated combat changes.

Completion requires observed behavioral RED then GREEN for Frenzy exclusivity at54–57/90 and native traits/no-player identity; `followers::` controls; Stockade control and per-follower casts/DPS comparison against unchanged initialization; source export equality; final check/fmt. Parent owns integration/publication.

## Proof ledger

| Scope | Revision | Command / artifact | Result |
|---|---|---|---|
| Native knowledge/traits RED fixtures | `5ed7886e1` | `follower_native_book_ --ignored --nocapture --test-threads=1` | Not observed yet |
| Stockade observation only | `c84d47567` | Existing `stockade_holy_healer_leaves_the_entrance_with_the_native_party` writes `native-book-stockade-combat.json` | Not run yet |
| RED helper | Rust scope `c84d47567` | `E/native_book_red_v1.pyrun.py`, `E/native-book-red-v1-*` | Formal ADD admission pending; no feature result |

`E` = engine `data/diagnostics/dps-parity-2026-10-21`. Prepared read-only world snapshot SHA-256 `464fbb4629af22f1d32ebdd5ad35b6f5800147a4eea1aa4755d32dedb99ee3c7`.

The unrelated selected-trait resolver RED-v2 terminated before Cargo tests because `--locked` was supplied twice. That is a harness argument failure, **not behavioral RED**. Return to classifier branch and correct the invocation before resolver production changes.

## Baseline observation limits

Stockade uses its existing native200sim_s seed6 movement control and unchanged route/casts/resources. Added receipt reads actual `CastSuccess` and `Damage` logs for each follower, including zero-output followers. DPS divides directly attributed damage by full elapsed simulated time; pet damage is not reattributed. This is a paired regression observation, not an isolated causal decomposition or full dungeon/APL parity proof.

# Native follower book authority

## Active goal and boundaries

Lead-approved separate tip from master7639103489: initialize ordinary followers' KnownSpells and TraitLoadout through the same native `starter_purchases`/`build_config` computation as the source exporter. Controller action rows and canonical override-family aliases must not grant unselected native talents. Only identity/profile data participates in computation; never insert or spawn a Player entity. Keep classifier and named trait-family resolver changes off this tip.

Required proof: RED for274837/1243807 overlap across the source window; unchanged positive native learn/override boundaries54–57/90; `followers::` suite and Stockade control at the tip; per-follower casts/damage/DPS versus same-master baseline. Use master+RED-only binary for the baseline ledger, then the same input/seed/control after GREEN. Parent owns publication/landing; feature refspec explicitly authorized `dps-parity-native-book:dps-parity-native-book`.

## Exact observed source boundary

- Ordinary initializer currently learns every eligible `follower_spec_kit` row, with no TraitLoadout. Exact-reference fixtures later replace that with a validated selected book; these are different code paths today.
- Native source exporter: `followers/kit_source_tests.rs::follower_kit_export_native_source`. It validates all eleven requested NPC specs at levels1–90 and handles source-neutral race0 directly. Do not replace that with player level/spec resolution or a grant-only fallback.
- Feral274837 is learned at55/56; Frantic1243807 replaces it at57/90. Native TraitDefinition138989 explicitly has OverridesSpellID274837. A controller canonical-family row spanning that window is not a second native spell grant.

## Proof ledger

- RED authored: `followers/native_book_tests.rs`; an in-memory controller family projection expands only the sourced base action interval, then the real initializer must not learn both variants. Separate control requires native TraitLoadout without any added Player entity.
- Native runtime RED not yet observed. No production book implementation before RED.
- Master baseline:7639103489fb8b822864e164c25211cfb18a9e88; output directory and immutable binary receipt will be sealed after the first build.
- Final followers/Stockade proof and per-follower behavior delta: pending implementation.

## Ret takeover2026-10-23

Lead assigned native-book helper to Ret14752. Ported ignored REDs1131e1685, Stockade observation03699ef4a; Ret requested-spec/no-kit/selected-talent/no-Player case pushed ef331486f. No production helper yet.

| Attempt | Source/proof | Result |
|---|---|---|
| v1 | Ret engine diagnostics `native-book-red-status.json` |35 capacity HOLD checks exhausted; no build |
| v2 | `native-book-red-v2.proof.json` | FormalADD, helper compile425s, SHED RAM<6GiB; own slice stopped/verified inactive; no tests, **not RED** |
| v3 | `native-book-red-v3-status.json` |35 capacity HOLD checks exhausted, -j1; no build |
| v4 | `native-book-red-v4-status.json` | Bounded -j1 admission worker pending; production Rust scope remains ef331486f |

Artifacts live under engine `data/diagnostics/ret-parity-2026-10-23/`. Moved the allocated ended-owner target into game-server-ret-parity/target, without copying it. It contains only a3.4GiB partial dependency build and no game_server executable; native compiler is Arch1.99.0, unlike the older1.98.1 docs. Parent confirmed no genuinely warm1.99 ended-owner cache is available.

| Priority attempt | Proof | Result |
|---|---|---|
| v5 priority | `native-book-red-v5.proof.json` | ADD24.5GiBfree, -j1 compile1577s, later SHED5.1GiBfree; no tests, **not RED** |
| v6 priority | `native-book-red-v6.proof.json` | ADD22.1GiBfree, compile1032s through server/core; later SHED5.5GiBfree, disk425GiB; no tests, **not RED** |

Both used explicitly authorized `CAPACITY_ID=lead_ret_14752`, obeyed SHED and stopped only the owned slice; verified inactive/empty ControlGroup. Dependencies are now warm. Further identical opt2 attempts paused pending a reserved compile window or a lower-resource targeted test profile; no production helper has been written.

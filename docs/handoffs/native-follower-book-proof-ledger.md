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

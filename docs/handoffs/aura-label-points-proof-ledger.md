# Native aura label-point recalculation

Verified source2026-10-23; four genuine native runtime REDs confirmed atcf496f61b. Generic implementation GREEN/lead gates pending. Native70448/TC a352b1fa precede SimC76f8118. Contract: [auras](../specs/auras.md#label-point-modifiers-on-existing-auras).

## Root boundary

Native `spell_mods` already matches aura218/219 by SpellLabel, and `effects::modded_base_points` applies Points/PointsIndex0–4. Aura construction snapshots that result. Adding/removing a later point modifier currently does not recalculate an existing passive's stored amount.

TC `SpellAuraEffects.cpp1194–1249` applies/removes a spell modifier before recalculating affected self-owned passive/permanent auras (or explicitly updating temporary auras). Charged modifiers do not recalculate passive amounts. Points targets all effects; PointsIndexN targets N. The triggering effect is excluded from its own recalculation. Preserve aura identity, lifetime, stacks and periodic phase; this is not a fresh application.

## Native cases

| Case | Source data | Required behavior / proof |
|---|---|---|
| Efficient Training450989 | Native effects0/1=20; Windwalker1258122 effects15/16 aura219, label5146, ops3/12, flat20 |20 without modifier;40 with modifier in either apply order; restore20 on modifier removal. Native runtime RED: stored20/20 versus40/40; live CalcValue40/40 |
| Second Sunrise431474, Ret control | Native effects0/1=15/30; Retribution1258011 effects19/20 aura219, label4716, flat5/20 |15/30→20/50 on late modifier, restore on removal, same aura instance/lifetime. Native runtime RED: stored15/30 versus20/50; live CalcValue20/50 |
| Flames of Xoroth429657, percent control | Native effects0/1=4; Demonology137044 effect12 aura218/op8, label7018,−1% |4→3.96 on both effects with late modifier; removal restores4. Runtime RED: stored4/4 versus3.96/3.96 |
| Modifier-first/remove | Same unchanged native Monk definitions, ordinary KnownSpells passive sync |40 at construction; source removal restores20 without reapplying target. Runtime RED: stored40/40 after removal versus20/20 |

`auras/point_mod_tests.rs` changes native knowledge and runs actual passive synchronization; no invented spell rows, coefficient changes, direct effect-amount writes or buff/resource grants. It separately checks live CalcValue sees the modifier, distinguishing stale stored aura state from bad label matching.

## Runtime RED and implementation gate

`cf496f61b`, native build exit0/zero warnings18m40; four cases fail at the expected stale-amount assertion in10.52s. Proof/log: engine `data/diagnostics/ww-parity-2026-10-23/aura-label-points-red.{build.log,build-proof.json,output.log,proof.json}`. Binary SHA and exact command are in the proof. Runtime command from this checkout:

```text
DPS_PARITY_OUTPUT=/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/ww-parity-2026-10-23/native-before-ww-policy
DPS_PARITY_WORLD_DB=/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/ww-parity-2026-10-23/native-before-ww-policy/world.ww-frozen.db
DPS_PARITY_ACTOR=follower TMPDIR=/var/tmp/osso
agent-run ww-monk-parity python3 scripts/dps_parity_native.py target/debug/deps/game_server-49562a93a94e52d1 native_label_points --ignored --nocapture --test-threads=1
```

`point_mods` hooks apply/refresh/remove and stack loss, reusing existing CalcValue and label matching. Only effect amounts change; native dependent point-modifier chains propagate with TC's original triggering-effect exclusion. Guard/source coverage is not interchangeable with runtime coverage: frozen native data has no charged218/219 point-source row or self-label source selecting its own effect, so neither guard is claimed as a native lifecycle test. Four selected cases do not qualify every affected producer or300s parity. GREEN pending.

## Cross-spec source census

Eight explicit specs:70/71/72/253/266/269/577/1467. Source pool is selected/reference spells plus native level90 passive-kit/spec spells; target set is selected/reference census spells. Counts are candidate source-data impact, not runtime producer qualification.

| Spec | Label-point target aura effect slots | Selected traits with TraitDefinitionEffectPoints rows |
|---|---:|---:|
|70|4|10|
|71|0|11|
|72|0|11|
|253|1|11|
|266|4|11|
|269|10|9|
|577|0|10|
|1467|8|9|

27 target aura effect slots (per-profile sum); separate unimported TraitDefinitionEffectPoints affects82 selected traits/165 rows, including71 multi-rank traits and68 allocated above rank1. That import/CalcValue work belongs to14752, not this fix.

Efficient Training is rank1/max1, definition129904, with no TraitDefinitionEffectPoints row. SimC oracle lines1100–1101 explicitly show1258122 adding20 to its raw20; neither rank2 nor a hotfix explains40. Generated Live hotfix arrays contain4 spell/10 effect/0 power rows, none450989 and zero intersections with these eight selected/reference censuses. This is not a count of the entire Blizzard DBCache or manual C++ hotfix registrations; hotfix-authority decisions remain separate and no data changed.

Evidence under engine `data/diagnostics/ww-parity-2026-10-23/`: `trait-points-label-eight-profile-source-census.json`, `hotfix-eight-profile-source-census.json`. Oracle: `data/diagnostics/simc-spell-oracle-2026-10-23/269/tiger_palm.{json,txt,proof.json}`. [WW context](ww-source-census.md).

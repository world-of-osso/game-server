# Assassination DPS parity

Spec259 work targets the [ST/3T parity contract](../../specs/assassination-parity.md), not a new production rotation. Diagnostics: `game-engine/data/diagnostics/assassination-parity-2026-10-23/`.

## Current coverage

|Boundary|Observed proof|Still missing|
|---|---|---|
|Pinned SimC reference|ST186209.2049;3T265841.4526; exact binary/profile hashes, native target-level93 and no-raid flags|ST command exit unavailable after a post-run receipt-helper error; original complete raw report recovered without rerunning; explicitly null exit|
|Source census|144 action/spell identities,77 trait allocations,49 literal APL rows; native effects/aura/ProcOptions/PPM, signed TDB links and actual TC definitions|Inventory is not runtime qualification|
|Existing native Assisted player baseline|Both ST/3T stop at9.8s/197decisions on rule413/condition12 values[3,10,0]|No300s DPS or channel deltas; literal driver must replace the blocked controller, not bypass it|
|Improved Garrote snapshot|2native controlsPASS/1RED:45.15 instead of67.725 for the native50% bonus|Correction GREEN pending|
|Improved Garrote earned lifecycle|Native Stealth cast succeeds, missing392403RED; existing no-talent stealth controlPASS|Correction GREEN pending|

## Root cause and implementation

TC `spell_rog_improved_garrote` attaches to1784 (signed TDB script table confirms binding) and requires native381632. Stealth applies392403/removes392401; removing stealth reverses the pair. `spell_rog_improved_garrote_damage` snapshots effect1's percentage at703 effect0 application, preferring392401 over392403. Existing server stealth children were present but neither the bonus lifecycle nor snapshot existed. The correction uses native triggered spells and the existing periodic-amount snapshot boundary; no AP coefficient or native period change.

Native Garrote AP coefficient0.4515 agrees with the single-spell SimC oracle. Shared259 oracle receipts were regenerated with inherited `optimal_raid=0`; invalid raid-buffed attempts are archived separately and excluded. Compare actual transient buff/stat factors, not unaligned raw hit values.

## Shared ownership

Use shared literal APL machinery; player/target/composite execution remains shared-owner work. `apply_poison` must preserve both learned2823 and381664 under selected381801; do not silently replace the composite with one spell. The approved `ROGUE_ENERGY_REGEN_COMBINED_SIMC_APL_ESTIMATE` is decision-only; SimC sc_rogue.cpp9894/9897 explicitly estimates90% Envenom uptime for Dashing Scoundrel. No native regeneration grants follow from that estimate.

## Sources

- Pinned TC a352b1fa `src/server/scripts/Spells/spell_rogue.cpp:534–618` — bonus lifecycle and snapshot.
- Native70448 SQLite rows392401/392403 effect1=50 and703 AP coefficient0.4515; source snapshot SHA51455391e7d4962589cc66d239bf77b2dbf3f4372ec8daee80c092f1e7989d9b.
- Pinned SimC76f8118 `profiles/MID2/MID2_Rogue_Assassination.simc`, `engine/class_modules/sc_rogue.cpp:9838–9906`.
- Diagnostic `assassination-source-census-v1.json`, `native-existing-master-{1,3}.proof.json`, `garrote-snapshot-red` and `garrote-lifecycle-red` logs/proofs.

## See also

- [Shared native APL facts](../systems/simc-apl.md).
- [General parity measurement](../../specs/dps-parity.md).

# Assassination DPS parity

Spec259 work targets the [ST/3T parity contract](../../specs/assassination-parity.md), not a new production rotation. Diagnostics: `game-engine/data/diagnostics/assassination-parity-2026-10-23/`.

## Current coverage

|Boundary|Observed proof|Still missing|
|---|---|---|
|Pinned SimC reference|ST186209.2049;3T265841.4526; exact binary/profile hashes, native target-level93 and no-raid flags|ST command exit unavailable after a post-run receipt-helper error; original complete raw report recovered without rerunning; explicitly null exit|
|Source census|144 action/spell identities,77 trait allocations,49 literal APL rows; native effects/aura/ProcOptions/PPM, signed TDB links and actual TC definitions|Inventory is not runtime qualification|
|Existing native Assisted player baseline|Both ST/3T stop at9.8s/197decisions on rule413/condition12 values[3,10,0]|No300s DPS or channel deltas; literal driver must replace the blocked controller, not bypass it|
|Improved Garrote snapshot|2native controlsPASS/1RED:45.15 instead of67.725; corrected source3d9b2e3f0 passes all4Garrote cases in the8-case native29.67s composite|No full BiS DPS effect attributed yet|
|Improved Garrote earned lifecycle|Native Stealth succeeds/missing392403RED; corrected native lifecycle and no-talent controlsGREEN in the same8-case composite; Envenom/Rupture/Mutilate retainersPASS|Vanish/other stealth-like lifecycles not certified by this1784-specific slice|
|Deadly Poison own repeat|Current-input native1controlPASS/1RED13.34s; first/foreign applications verified; same-caster repeat lacks113780|Correction committed; GREEN unobserved because admission attempts launched no tests|
|Literal precombat coatings|Pinned default selects2823,3408,381664,381637; native cast-time row16 is1500ms for each|Source inventory only; composite/cast-completion boundary still unqualified|

## Root cause and implementation

TC `spell_rog_improved_garrote` attaches to1784 (signed TDB script table confirms binding) and requires native381632. Stealth applies392403/removes392401; removing stealth reverses the pair. `spell_rog_improved_garrote_damage` snapshots effect1's percentage at703 effect0 application, preferring392401 over392403. Existing server stealth children were present but neither the bonus lifecycle nor snapshot existed. The correction uses native triggered spells and the existing periodic-amount snapshot boundary; no AP coefficient or native period change.

Native Garrote AP coefficient0.4515 agrees with the single-spell SimC oracle. Shared259 oracle receipts were regenerated with inherited `optimal_raid=0`; invalid raid-buffed attempts are archived separately and excluded. Compare actual transient buff/stat factors, not unaligned raw hit values.

## Deadly Poison repeat boundary

TC `spell_rog_deadly_poison` (384–404) calls113780 when the same caster already has2818 on its target, before the new aura applies. Native70448 has speed0, so the hook runs once at the existing per-target aura-resolution boundary before refresh; no missile timing is substituted. Do not route this through an after-apply hook, which would treat every first application as a repeat.

The first new probe failed before test logic because its canonical backup preceded publication of `trait_definition_effect_points`; that is schema-only evidence, not RED. A fresh current-canonical `.backup` (dd1de231…) includes the published table. The same compiled source29fc88a then observes1controlPASS/1genuineRED13.34s: first/foreign applications do not deal instant damage, but this caster's repeat also incorrectly has zero113780 hits. The correction adds only the TC-owned repeat child. GREEN pending; native poison-parent dummy4 dispatch and full poison DPS remain unqualified.

## Shared ownership

Native Player dispatch/target-aware Prepared landed as pd1035 and is included by merge29fc88a. Remaining composite/cycle/precombat and target-fact extensions belong to the shared literal machinery; do not fork a259 selector.

Pinned `apply_poison` selects four coatings in constructor order: Deadly2823, Crippling3408, Dragon-Tempered Blades Amplifying381664 and Atrophic381637. The profile selects381801 and381637, not Numbing Poison. SimC's action selects background drivers and its execute method only marks the action executed. Native `SpellMisc.CastingTimeIndex=16` resolves to Base/Minimum1500ms for all four. The current synchronous WW precombat loop does not advance time: shared integration must define native completion before snapshot/combat rather than assume instantaneous casts or omit nonlethal coatings. This records a source boundary, not an implemented precombat policy.

The approved `ROGUE_ENERGY_REGEN_COMBINED_SIMC_APL_ESTIMATE` is decision-only; SimC sc_rogue.cpp9894/9897 explicitly estimates90% Envenom uptime for Dashing Scoundrel. It must consume the shared potential regeneration fact, not the native0-at-cap cache. Exact cap/depleted fixtures have been supplied to the shared owners; no shared regeneration change or resource grant is implemented by this slice.

## Sources

- Pinned TC a352b1fa `src/server/scripts/Spells/spell_rogue.cpp:534–618` — bonus lifecycle and snapshot;384–404 — owned Deadly Poison repeat child.
- Native70448 SQLite rows392401/392403 effect1=50 and703 AP coefficient0.4515; source snapshot SHA51455391e7d4962589cc66d239bf77b2dbf3f4372ec8daee80c092f1e7989d9b.
- Pinned SimC76f8118 `profiles/MID2/MID2_Rogue_Assassination.simc`, `engine/class_modules/sc_rogue.cpp:3164–3330` — default four-coating selection;9838–9906 — combined estimate.
- Current private snapshot dd1de231… native `spell_misc`/`spell_cast_times` row16; diagnostic `apply-poison-source-packet-v1.json`, `literal-option-footprint-v1.json` and `energy-potential-counterexample-packet.md` — source/fixture proposals, not runtime proof.
- Diagnostic `assassination-source-census-v1.json`, `native-existing-master-{1,3}.proof.json`, `garrote-snapshot-red` and `garrote-lifecycle-red` logs/proofs.

## See also

- [Shared native APL facts](../systems/simc-apl.md).
- [General parity measurement](../../specs/dps-parity.md).

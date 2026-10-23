# Assassination DPS parity

Qualify native spec259 against pinned SimC76f8118 BiS gear/talents:300s single-target31146 and the prescribed three-target add waves, within ±10% in both. Native70448 and TC a352b1fa remain mechanics authority; [investigation and evidence](../wiki/investigations/assassination-dps-parity.md).

## What it must do

- [ ] Consume the literal pinned APL through shared `simc_apl`, `simc_apl_native` and `dps_parity/literal_rotation`; do not substitute AssistedCombat or a local priority list.
- [ ] Selected Improved Garrote makes actual Stealth1784 apply392403; leaving stealth replaces it with392401. No talent means no invented bonus.
- [ ] Native Garrote703 snapshots one native effect1 bonus from392401/392403, preferring the former when both exist. Later removal does not change already-earned ticks; foreign-unit buffs do not contribute.
- [ ] `ROGUE_ENERGY_REGEN_COMBINED_SIMC_APL_ESTIMATE` is a decision-only SimC fact over native regen/owned poisoned bleed tick state. The sourced90% Dashing Scoundrel estimate never grants Energy or changes real regeneration.
- [ ] ST and3T receipts expose gear/stat/action/channel differences and actual target damage before claiming the ±10% band.

## How it works

- [Shared literal evaluator](../wiki/systems/simc-apl.md).
- [Source census and current proof boundary](../wiki/investigations/assassination-dps-parity.md).

## Implementation inventory

- `crates/server/src/auras/improved_garrote.rs` — sourced stealth bonus lifecycle and periodic snapshot.
- `crates/server/src/auras/scripts.rs` — existing native stealth/periodic hook integration.
- `crates/server/src/dungeon_simulator/dps_parity/` — shared native measurement machinery.

## Tests asserting this spec

- `crates/server/src/spell_cast/assassination_native_tests.rs`.
- `crates/server/src/spell_cast/rogue_stealth_tests.rs` — existing native no-talent stealth lifecycle control.

## Known gaps (current cycle)

- [ ] Garrote correction GREEN qualification pending; source-backed RED retained.
- [ ] Assassination adapter, shared player/target/composite support, native ST/3T channel baseline and band qualification.
- [ ] Poison/hero/talent/trinket channels not certified by source inventory alone.

## Out of scope

- AssistedCombat condition12 reinterpretation, coefficient tuning to a desired total, decompiled sources, native trait-legality bypasses and canonical database mutation.

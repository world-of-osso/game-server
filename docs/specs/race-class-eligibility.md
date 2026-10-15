# Installed race/class eligibility

Retail creation eligibility and race-mask identity follow the installed DB2 build, not a copied race/class matrix. Source: `crates/server/src/race_class.rs`; [runtime and proof boundaries](../wiki/systems/race-class-eligibility.md).

## What it must do

- [ ] Admit installed CharBaseInfo pairs for supported gameplay classes1–13 using playable ChrRaces metadata; do not enable unconverted Adventurer14.
- [ ] Derive RaceMasks bits from installed ChrRaces.PlayableRaceBit rather than race IDs or the retired Retail table.
- [ ] Accept Dwarf3/Shaman7 through ordinary admin/client validation, authentic starting kit/location and character persistence.
- [ ] Fail explicitly on missing, corrupt or empty required source instead of using a static Retail permission matrix.
- [ ] Preserve the separately authored Forever95/96 class/bit contract, existing unsupported-pair rejection, start levels and visage policy.

## How it works

- [Race/class source loading](../wiki/systems/race-class-eligibility.md)
- [New-character kits and authentic starts](../wiki/systems/new-character-kit.md)
- [Forever race contract](skyborne-races.md)

## Implementation inventory

- `crates/server/src/race_class.rs` — installed eligibility, source race bits and preserved policy consumers.
- `crates/server/src/persistence_accounts_tests.rs` — actual admin creation with native kit and persisted Dwarf/Shaman.
- Engine `scripts/desktop-server/test-data.txt` — required CSV inputs for container-hosted server tests.

## Tests asserting this spec

- `race_class::tests::race_source_installed_char_base_info_pairs_are_admitted`
- `race_class::tests::race_source_missing_or_empty_installed_tables_do_not_use_static_defaults`
- Existing `race_class::tests` mask, unsupported-pair, start-level and Forever controls.
- `persistence_accounts_tests::race_source_admin_dwarf_shaman_uses_authentic_kit_and_persists`

## Known gaps (current cycle)

- [ ] Run scoped Rust gates and new private admin creation proof; implementation is not production deployment.
- [ ] Eligibility does not certify every new pair's intro, asset closure, spell kit or full class rotation.

## Out of scope

Stat/spell tuning, start-level/visage redesign, new classes, Forever metadata regeneration and realm5000 operations. Existing source-qualified Forever95/96 policy is preserved rather than replaced by Retail NPC metadata.

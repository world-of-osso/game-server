# Native item sets

Retail equipment set bonuses use native ItemSet/ItemSetSpell data and TC a352b1fa eligibility rather than synthetic stats. [Source and proof ledger](../handoffs/native-item-set-proof-ledger.md).

## What it must do

- [x] Normal DB2 import preserves native set identities, thresholds, specialization, subtree and build receipts without replacing unrelated data.
- [ ] Original equipped item identities determine set counts; stat-allocation redirects do not change set membership.
- [ ] Apply the eligible native set spell when its piece threshold is reached; remove it below threshold; re-equipping restores it without duplicate auras.
- [ ] Specialization changes replace only set-owned spells. Preserve ordinary item, enchant and learned passive auras.
- [ ] Honor native inactive flags, skill requirements and selected hero-subtree restrictions.
- [ ] APL set facts use native membership/threshold eligibility and the explicit SIMC_TIER_ALIAS_ITEMSET_BINDING policy; unknown tier operands error.

## How it works

- [Native source, ownership and proof](../handoffs/native-item-set-proof-ledger.md)

## Implementation inventory

- `scripts/db2.py`, `scripts/db2_schema.sql`: normal native table import.
- `crates/server/src/item_templates.rs`: native item/set catalog load.
- `crates/server/src/equipment_effects/item_sets.rs`: count/eligibility and aura lifecycle.
- `crates/server/src/equipment_effects.rs`, `crates/server/src/auras/mod.rs`: ordinary aura application and fixed-update integration.
- `crates/server/src/class_progression.rs`: active selected subtree lookup.

## Tests asserting this spec

- `scripts/tests/test_db2_item_sets.py`: native Ret and Elemental catalog output.
- `crates/server/src/equipment_effects.rs`: ignored real Ret2062 threshold and specialization lifecycle tests.

## Known gaps (current cycle)

- [ ] Native runtime GREEN and parent integration gate.
- [ ] Generic set APL operands and alias binding.
- [ ] Actual tier child proc/effect qualification and resulting DPS parity.

## Out of scope

- Canonical DB publication is parent-owned; all local evidence uses private copies.
- Heirloom level/bonus-curve retirement and non-Retail set namespaces remain unqualified by this parity slice.

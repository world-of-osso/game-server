# Midnight equipped item damage

Verified: 2026-10-26.

## Native ownership and amounts

Call the Swarm1310082 belongs to ring281029. Void Eruption1310208 comes from bonus14049→ItemEffect238161. Current64/62/263 profiles equip both. Native damage leaves1310088/1310209 have zero points/coefficients; each driver effect0 supplies a class−9 coefficient and an item-scaled budget. The [shared item calculator](../investigations/item-healing-scaling.md) owns budget/rounding semantics; no replacement coefficient is introduced here.

| Driver | Delivery | Native budget observed on the qualified profiles |
|---|---|---|
| Call the Swarm1310082 |1310083 native missile→1310088 Acidbrood |6884 |
| Void Eruption1310208 |1310209 native school damage |5163 |

Pinned SimC76f8118 `unique_gear_midnight.cpp5679–5748` assigns driver effect1 average to the damage payload, applies the role expression and assumes full unsplit AoE. Its default bug mode intentionally omits the inactive Venomcursed15% damage bonus. [Native role expressions](zathatek-weapon.md) use the existing shared parser, not a new specialization table.

## Implementation boundary

`server-spell-scripts/src/modules/midnight_item_damage.rs` replaces only the two damage leaves' effect0 point inputs with native driver budgets at the original casting-item level and role. The ordinary school-damage handler retains native targeting, crit and downstream damage stages. Acidbrood keeps its existing native missile/trigger path. A separate Void callback supplies its missing action while the generic proc manager retains chance, RPPM and cooldown ownership. Ret's stat-item module is unchanged.

The generic aura-proc trigger now passes `aura.cast_item_level` into its child. Previously it called the no-item overload, so the originating item context was lost before the missile chain. No new context field or alternate scaling path is added.

## Native qualification

Test7a87cbe6f runs ordinary300s literal combat with actually equipped profiles, no forced proc rolls. The equipment-owned driver amount is first checked against native item-level scaling and the role expression. Genuine REDs:64`29dc9db2d3` records24Acidbrood hits/zero positive;62`1bbd0b166a`29/zero;263`5eb40449b6`27/zero. Void records no hits in all three. Both native driver budgets are positive before combat. Production implementation is committed before renewed runtime qualification; GREEN and numerical acceptance remain pending.

Private inputs/artifacts: `parity-64/item-damage-gates/{64,62,263}/`. [Frost parity](../investigations/frost-mage-dps-parity.md) owns the latest total-DPS state; these item fixes must not be represented as an isolated DPS gain without a qualified measurement.

## Sources

- SimC76f8118 `engine/player/unique_gear_midnight.cpp5679–5748`, `unique_gear.cpp5021–5065`.
- Native70448 `spell_effect`, `item_effect`, `item_x_item_effect`, `item_bonus` and description variables; shared data remains read-only.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/midnight_item_damage_tests.rs`.

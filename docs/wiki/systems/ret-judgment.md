# Ret Judgment target uses

Ret's197277 target debuff is a pool of independently expiring uses, not a damage multiplier per stack.

## Native relationship

`ret_judgment.rs` grants the native initial uses after successful Judgment/HoW-family impacts with selected Greater Judgment. Selected Lingering Radiance grants one use when owned Dawnlight ends naturally. Native data supplies amount, class mask, duration, cap and initial uses; SimC maps initial stacks to `ProcCharges`.

A successful affected impact consumes one owned use. Tempest and Sunrise Storm receive the bonus without consuming uses. The registry's `NonStackingIncomingAuras` metadata keeps incoming damage magnitude independent of the remaining use count. Other auras retain their existing stack scaling.

## Qualification

Native RED6a9c30b6f8 observed zero target uses after an ordinary Judgment. GREENe4384f8211 is pending, including two real Judgment casts, native Verdict damage/consumption and the consumed-oldest-deadline boundary. Full Ret gates and three-seed numerical acceptance remain open.

## Sources

- [Contract](../../specs/dps-parity.md#ret-judgment-target-uses).
- [Implementation](../../../crates/server-spell-scripts/src/modules/ret_judgment.rs).
- [Native test](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/ret_judgment_tests.rs).
- Pinned SimC76f8118: `sc_paladin.cpp1436–1441/1804–1812/3057–3065/3936`, `sc_paladin.hpp1237–1247`, `spell_data.hpp564`.

## See Also

- [[spell-script-crate]] — downstream relationship registration.
- [[auras]] — native application and stack lifetime.

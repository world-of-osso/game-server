# Zatha'tek weapon proc

Weapon271093's native1298023 DUMMY driver has no trigger edge. The downstream module supplies pinned SimC76f8118's missing1305395 damage and1305391 recipient relationship; native catalogs retain item scaling, proc rates, role multipliers, stack cap and lifetime.

## Runtime

The accepted positive outgoing proc uses the equipped aura's already-scaled effect0 budget. Its triggered1305395 retains item level and normal damage/critical resolution. Native description variables supply the role multiplier. The callback excludes its own action rather than rejecting every item proc.

Before damage, the payload reads only its caster's existing Tear stacks on that recipient. After impact, the module applies the native Tear definition. **The percentage comes from1298023's non-aura effect1.**1305391effect0 is a separate item-scaled value; it is not a percentage and remains unchanged. Native5d9da179b2/00e01dcf80 exposed the mistaken effect0 interpretation through paired actual damage, not a coefficient adjustment.

Verified2026-10-26: all139pinned profile manifests contain this weapon only for259/261. Both are positive gates;260is the unequipped negative control. Producer tests start real equipped swings before instant native payload controls isolate recipient amplification without clock/stat drift.

## Sources

- [Contract](../../specs/zathatek-breath.md).
- [Module](../../../crates/server-spell-scripts/src/modules/zathatek_breath.rs).
- [Native cases](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/zathatek_tests.rs).
- Pinned `unique_gear_midnight.cpp5233–5280`, `unique_gear.cpp5021–5065`, and native70448 spell/effect/description data.

## See Also

- [[auras]] — native application, stacking and proc admission.
- [[character-stats]] — native equipped item context and role data.

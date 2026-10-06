# Item Healing Scaling

Verified 2026-10-06 against world.db and server regression tests. Algari Healing Potion was consumed in combat without healing on server a53edf9.

## Root cause

`spell_cast/effects.rs` excluded spells with Attributes_11 bit 0x4 (`SCALES_WITH_ITEM_LEVEL`) from the coefficient branch of `calc_base_value`. It returned `EffectBasePointsF`, which is zero for these consumables. `inventory::use_item` also discarded the casting item's scaling metadata. SpellScaling and ExpectedStat were not missing: this effect needs RandPropPoints at the item's effective level.

Instrumentation reproduced an injured level-90 warrior at 18444/100000 health: item 211878 was consumed; spell 431416 effect 0 (HEAL 10) resolved to the caster with amount 0 and overheal 0. Its index-1 DUMMY effect has base points 50 and is not the heal. This excludes incorrect targeting, effect selection, and encounter clamping.

## Amount and fix

Algari ranks 211878/211879/211880 share spell 431416, coefficient 5450 and scaling class -9. ItemSparse levels 265/275/285 become 56/58/60 through the existing Midnight item-squish curve 92181. RandPropPoints DamageSecondaryF is 8.54874134064/9.14560699463/9.79184818268; float multiplication and rounding yield **46591/49844/53366 health**, before healing modifiers and overheal. Character level does not select this budget.

The item-use path retains the effective item level on a per-cast spell clone. Cast bars retain that clone; missiles already retain their spell; triggered spells inherit its item level. Aura application snapshots the clone's amounts rather than reloading the context-free catalog spell. The shared amount calculator uses DamageReplaceStatF for class -8, DamageSecondaryF for -9, and SuperiorF_0 (rare chest budget) for other nonzero item-scaling classes. Positive amounts below one become one before rounding. Spells without a casting item use level 1, matching TrinityCore's defined effectiveItemLevel.

## Regression coverage

`inventory::item_use_tests::potionfix_*` exercises the real read-only spell/item catalog, HostClient UseItem delivery, 50-ms server ticks, temporary character inventory persistence, consumption, health, and combat-log target/amount/overheal.

- Algari rank 1: level-90 warrior in combat; exactly 46591 health, one potion consumed.
- Algari ranks 2/3: levels 71/80; 49844/53366 health, proving item rather than player-level scaling.
- Minor Healing Potion 118 at level 10: spell 439, class -9; 121 health. Healing Tonic 109223 at level 40: spell 156438, rare chest budget; 3 health.
- Tough Jerky 117: spell 433 regenerates its scaled food amount while seated. Elixir of Lion's Strength 2454: spell 2367 grants 4 strength.
- Antiseptic Bandage 111603 at level 40: spell 161255 retains a one-point periodic heal after consumption.

Representative tests do not establish full support for every consumable script. Existing class -6/-7 item-level multipliers and target-level scaling remain outside this fix.

## Sources

- [effect amount calculator](../../../crates/server/src/spell_cast/effects.rs)
- [item use](../../../crates/server/src/inventory.rs)
- [item scaling tables and squish](../../../crates/server/src/item_stats.rs)
- [behavioral tests](../../../crates/server/src/item_use_tests.rs)
- TrinityCore a352b1fa `SpellInfo.cpp`, `SpellEffectInfo::CalcBaseValue`, lines 632–685: item-level RandPropPoints branch and minimum/rounding rules.
- Live evidence: game-engine `data/diagnostics/stockaderun-2026-10-06/potion-target-evidence.json`, `server2.log`, `client6.log`, and the a53edf9 source dumps. Shared data remained read-only.

## See Also

- [[spell-casting]] — effect resolution and cast lifecycle
- [[inventory]] — item-use persistence
- [[auras]] — periodic amount snapshots

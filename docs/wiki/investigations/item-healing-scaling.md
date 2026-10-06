# Item Healing Scaling

Verified 2026-10-06. Algari Healing Potion was consumed without healing on server a53edf9.

## Root cause

At a53edf9, `spell_cast/effects.rs:249` excluded Attributes_11 bit 0x4 (`SCALES_WITH_ITEM_LEVEL`) from coefficient scaling; line 263 returned zero BasePoints. `inventory.rs:305,331` also discarded the item's scaling metadata.

Instrumented server reproduction: an injured level-90 warrior at 18444/100000 health consumed item 211878. Spell 431416's index-0 HEAL resolved to the caster with amount 0 and overheal 0. Its index-1 DUMMY's base points 50 are unrelated. This rules out wrong target, wrong index, and encounter clamping. The missing path is **item-level RandPropPoints**, not ExpectedStat or gtSpellScaling.

## Fix and amounts

Item use retains the effective item level on a per-cast spell clone through cast bars, missiles, triggers, and aura amount snapshots. The shared calculator uses DamageReplaceStatF for class -8, DamageSecondaryF for -9, and SuperiorF_0 for other nonzero item-scaling classes. Positive sub-one amounts become one; rounding follows spell attributes.

Algari ranks 211878/211879/211880 share coefficient 5450 and class -9. Existing squish curve 92181 maps ItemSparse levels 265/275/285 to 56/58/60. DamageSecondaryF 8.54874134064/9.14560699463/9.79184818268 yields **46591/49844/53366 health** before healing modifiers and overheal.

## Proof and catalog limits

`inventory::item_use_tests::potionfix_*` uses read-only world.db, real UseItem delivery, 50-ms ticks, temporary persisted inventory, and health/combat-log assertions:

- Algari: warrior levels 90/71/80; exact amounts and one item consumed.
- Minor Healing Potion 118 at level 10: 121 health. Healing Potion 929 at level 20: 1 health.
- Dwarven Mild 422 at level 10: three seated ticks restore 91 whole health, versus 90 without food. Minor Agility Elixir 2457: 7 agility.
- Netherweave Bandage 21990 at level 40: one health per periodic tick after consumption.
- Cast-bar/missile tests modify Algari's spell **in memory** to test context lifetime, not claim Algari itself has cast time or travel.

Read-only pattern scan: 496 item-scaled healing/food/elixir effects across 475 usable catalog items; all effective item levels have RandPropPoints rows (425 positive, 2 negative modifiers, 69 zero source budgets).

Healing Tonic 109223 and Antiseptic Bandage 111603 exist in DB2 but not `content_item`; they cannot currently be used. RandPropPoints levels **1–3** have zero floating budgets, leaving Tough Jerky 117 and Lion's Strength 2454 with source-derived zero amounts. No invented fallback or data edits. Class -6/-7 item multipliers and target-level scaling remain outside this fix.

## Sources

- [amount calculator](../../../crates/server/src/spell_cast/effects.rs), [item use](../../../crates/server/src/inventory.rs), [item budgets/squish](../../../crates/server/src/item_stats.rs), [tests](../../../crates/server/src/item_use_tests.rs)
- TrinityCore a352b1fa `SpellInfo.cpp:632–685`, `SpellEffectInfo::CalcBaseValue`: item-level budgets and rounding.
- game-engine `data/diagnostics/stockaderun-2026-10-06/`: `potion-target-evidence.json`, server/client logs, a53edf9 source dumps. Shared data remained read-only.

## See Also

- [[spell-casting]] — cast lifecycle
- [[inventory]] — item persistence
- [[auras]] — periodic snapshots

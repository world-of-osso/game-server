# Arcane Mage parity

Spec62 follows the pinned SimC behavior and native-value rules in [DPS parity](dps-parity.md). Native execution lives in `crates/server-combat/src/spell_cast/scripts/arcane*.rs`; the literal controller lives in `crates/server-simulation/src/dungeon_simulator/dps_parity/arcane_rotation.rs`.

## What it must do

- [x] Arcane Surge snapshots current mana before payment and scales its damage from 1× at zero mana to its native effect 2 multiplier at full mana, increased by selected Mana Bomb (`sc_mage.cpp3334–3342`).

- [x] The selected Arcane Familiar is granted by an actual Arcane Intellect cast; its native aura owns the maximum Mana change. Its active native aura triggers Arcane Assault at the source hasted three-second cadence. Selected Energized Familiar adds its authored Assaults during Surge and rolls its authored Mana return per Assault. Selected Attuned Familiar rolls its authored chance to conjure an owned Splinter on each Assault.
- [x] Selected Enlightened samples the caster's Mana fraction on the native one-second aura clock and scales Arcane damage by native1217242 effect2 (`sc_mage.cpp6392–6399/6793–6798`). Its separate Mana-regeneration effect is not yet qualified.
- [x] Eligible Blast, Barrage, Missiles, Orb and Explosion executions roll Clearcasting through accumulated PRD. `sc_mage.cpp6626–6631` uses13 in the coefficient calculation, not as an actual attempt cap. Surge grants Clearcasting without a roll.
- [x] Improved Clearcasting stacks are independent uses sharing one proc charge. Missiles spends one stack; stack count does not multiply its cost reduction.
- [x] Actual hostile Missiles ticks execute native7268 once and generate earned MID2 Cumulative Power and selected Salvo, Focusing Crystal and High Voltage. Its secondary damage requires an AoE talent or earned Overpowered Missiles. A fresh channel rounds its modified duration to whole modified tick periods, in addition to its initial tick (`sc_mage.cpp3270–3296`).
- [x] Barrage rolls Orb Barrage before spending charges, rolls Prismatic from earned Salvo and consumes Salvo outside Arcane Soul. Polished Focus uses the pre-Orb stack count. Filler consumes Cumulative Power; Prismatic consumes its enabling aura.
- [x] Touch's caster-owned210824 mark accumulates actual owned spell damage, excluding aura87 vulnerability. Expiry or replacement discharges the native321507 fraction through210833. Aegwynn grants Clearcasting. Clearcasting grants selected Brainstorm and rolls Overpowered Missiles, banking an in-channel proc until the old channel ends.
- [x] Consumed Salvo, Cumulative Power, Intuition and Overpowered damage modifiers survive projectile flight.
- [x] Barrage freezes charge-based recipient count before Orb Barrage and uses native Arcing Cleave scaling. Missiles has one full primary and separate secondary recipients. Prismatic splash excludes its primary. Orb delivers a bolt to each source-selected enemy in range.
- [x] Arcane Touch, Surge, Barrage and Orb producers use the shared Spellslinger path with native counts, payload, doubling and stagger. Infused Splinters rolls native Salvo from actual Arcane Splinter impacts.
- [x] Configure Clearcasting, Salvo and Overpowered reaction clocks before acquisition. Predict eligible guaranteed acquisitions, not random ones (`sc_mage.cpp1860,7254–7255,7388–7389`).
- [x] Potion operands use shared configured native potion facts, not a local name matcher.
- [x] Actual Barrage hits feed selected Overflowing Energy: noncritical hits earn native stacks and critical hits clear them after the hit batch. Other Arcane spells do not change that buff (`sc_mage.cpp2693/1897–1898`).

## How it works

- [Literal APL execution](../wiki/systems/simc-apl.md)
- [Native spell scripts](../wiki/systems/spell-script-crate.md)

## Implementation inventory

- `crates/server-combat/src/spell_cast/scripts/arcane.rs`: Clearcasting, Missiles, Salvo and execution producers.
- `crates/server-combat/src/spell_cast/scripts/arcane_area.rs`: distinct native area recipients.
- `crates/server-combat/src/spell_cast/scripts/arcane_touch.rs`: owned Touch accumulation and discharge.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/arcane_rotation.rs`: pinned literal controller.

## Tests asserting this spec

- `crates/server-simulation/src/dungeon_simulator/dps_parity/arcane_tests.rs`

## Follower L90 measurement

- Snapshot v3, map2813, 120 seconds, seeds 70448 and 1–19: native 43,917 DPS, SimC 42,797 DPS; mean paired difference +2.74% ±4.28 percentage points (95% paired t interval). The paired mean meets the ±5% gate; the interval extends beyond it. Native source `2157a62ae2`, test binary SHA-256 `74ae5122b980d4930dbf8da6ccf546906ef2ac348ec4c341dcccf3c3cbfa1920`; SimC `76f8118`, saved gear and temporary enchants disabled.
- Each native fight cast Time Warp once, with 33.29% uptime; each reference activated Bloodlust once, with 33.33% uptime. The 20-seed means for Barrage damage were 1,342,857 native and 1,338,831 SimC; Missiles were 1,218,160 and 1,208,447; Touch discharge was 394,834 and 387,696. Surge uptime was 25% on both sides. Prismatic Bolt damage was 902,078 versus 818,270; this aggregate difference alone does not establish a defect. Earlier per-attempt Augury and Familiar proc comparisons did not establish a rate defect. No new mechanic defect was proven in this measurement.

## Known gaps (current cycle)

- [x] Blast and Barrage apply one native charge damage modifier frozen before charge gain/spend. Savant augments only its per-charge term; Prodigious multiplies both that mastery contribution (`sc_mage.cpp2076–2083`) and the 36032 base points (SimC affecting-aura debug: Barrage 90→108, Blast 60→72).
- [x] Ordinary Blast admission/payment and the actual cast bar apply native36032 cost/time effects once through the ordinary aura folds. Raw resource count is not a second multiplier (`sc_mage.cpp2091–2100/2850–2855`).
- [x] Paid Blast at capped charges preserves the charge bonus for the next paid cast. Barrage spends the charges after launch; unrelated modified spells do not consume its native resource aura (`sc_mage.cpp2734–2738/2858–2865`).
- [ ] Reach ±10% of pinned ST and3T DPS.
- [x] Qualify shared ordinary spell GCD haste across all40 native spec representatives before qualifying channel chaining. Category133 spells without melee/ranged defense or ability/ranged-slot attributes use ordinary spell haste before aura417, within the native750–1500ms bounds (TC a352b1fa `Spell.cpp9209–9224`; SimC76f8118 `action.cpp648–666,1285–1315`).

## Out of scope

- Continuous retail Orb movement geometry is not qualified by discrete bolt delivery tests.
- Shared Controlled Instincts, Spellfrost Teachings and Augury remain shared Spellslinger work.

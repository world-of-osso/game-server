# Arcane Mage parity

Spec62 follows the pinned SimC behavior and native-value rules in [DPS parity](dps-parity.md). Native execution lives in `crates/server-combat/src/spell_cast/scripts/arcane*.rs`; the literal controller lives in `crates/server-simulation/src/dungeon_simulator/dps_parity/arcane_rotation.rs`.

## What it must do

- [x] Eligible Blast, Barrage, Missiles, Orb and Explosion executions roll Clearcasting through accumulated PRD. `sc_mage.cpp6626–6631` uses13 in the coefficient calculation, not as an actual attempt cap. Surge grants Clearcasting without a roll.
- [x] Improved Clearcasting stacks are independent uses sharing one proc charge. Missiles spends one stack; stack count does not multiply its cost reduction.
- [x] Actual hostile Missiles ticks execute native7268 once and generate earned MID2 Cumulative Power and selected Salvo, Focusing Crystal and High Voltage. Its secondary damage requires an AoE talent or earned Overpowered Missiles.
- [x] Barrage rolls Orb Barrage before spending charges, rolls Prismatic from earned Salvo and consumes Salvo outside Arcane Soul. Polished Focus uses the pre-Orb stack count. Filler consumes Cumulative Power; Prismatic consumes its enabling aura.
- [x] Touch's caster-owned210824 mark accumulates actual owned spell damage, excluding aura87 vulnerability. Expiry or replacement discharges the native321507 fraction through210833. Aegwynn grants Clearcasting. Clearcasting grants selected Brainstorm and rolls Overpowered Missiles, banking an in-channel proc until the old channel ends.
- [x] Consumed Salvo, Cumulative Power, Intuition and Overpowered damage modifiers survive projectile flight.
- [x] Barrage freezes charge-based recipient count before Orb Barrage and uses native Arcing Cleave scaling. Missiles has one full primary and separate secondary recipients. Prismatic splash excludes its primary. Orb delivers a bolt to each source-selected enemy in range.
- [x] Arcane Touch, Surge, Barrage and Orb producers use the shared Spellslinger path with native counts, payload, doubling and stagger. Infused Splinters rolls native Salvo from actual Arcane Splinter impacts.
- [x] Configure Clearcasting, Salvo and Overpowered reaction clocks before acquisition. Predict eligible guaranteed acquisitions, not random ones (`sc_mage.cpp1860,7254–7255,7388–7389`).
- [x] Potion operands use shared configured native potion facts, not a local name matcher.

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

## Known gaps (current cycle)

- [x] Blast and Barrage apply one native charge damage modifier frozen before charge gain/spend. Savant augments only its per-charge term; Prodigious multiplies only that mastery contribution (`sc_mage.cpp2076–2083`).
- [ ] Reach ±10% of pinned ST and3T DPS.
- [ ] Qualify shared ordinary spell GCD haste across all40 native spec representatives before qualifying channel chaining. Category133 spells without melee/ranged defense or ability/ranged-slot attributes use ordinary spell haste before aura417, within the native750–1500ms bounds (TC a352b1fa `Spell.cpp9209–9224`; SimC76f8118 `action.cpp648–666,1285–1315`).

## Out of scope

- Continuous retail Orb movement geometry is not qualified by discrete bolt delivery tests.
- Shared Controlled Instincts, Spellfrost Teachings and Augury remain shared Spellslinger work.

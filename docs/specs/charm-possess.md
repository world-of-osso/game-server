# Charm and possession

Generic aura2/6 control uses native70448 spells and TrinityCore a352b1fa `Unit::SetCharmedBy`/`RemoveCharmedBy`, `AuraEffect::HandleModPossess`/`HandleModCharm`, `CharmInfo`, `PossessedAI` and `SimpleCharmedPlayerAI`. [Runtime](../wiki/systems/charm-possess.md).

## What it must do

- [x] Native605/205364/1098/30923 assign one charmer, adopt its faction, stop the prior cast/combat/threat, reject a second charmer, and restore identity on aura removal (native aura-boundary3GREEN).
- [ ] Ordinary charm follows/attacks for its charmer through pet commands/AI; possession only attacks on command and never AI-chases.
- [ ] Possession routes authenticated PlayerInput to the controlled unit, validates its movement epoch/budget, freezes the player's own body, and restores control on removal.
- [ ] Native duration, channel cancellation, damage interrupt flags, death and lost-caster lifetime govern removal; never infer break-on-damage from spell names.
- [ ] Existing PetSpells/PetAction show and command charm; cancellation releases control rather than deleting the creature or its pet stable record.

## How it works

- [Charm runtime and source qualifications](../wiki/systems/charm-possess.md).

## Implementation inventory

- `crates/server/src/auras/charm.rs`: original-identity snapshot, UnitCharm/charmer link, apply/remove lifecycle, movement authority.
- `crates/server/src/pet_control.rs`: existing pet commands and AI accept the charmer's controlled unit.
- `crates/server/src/networking{,_movement}.rs`: possessed movement uses existing authoritative movement pipeline.
- `crates/server/src/{combat,creature_aggro,creature_ai,scripted_ai/mod,spell_cast/mod}.rs`: pause original AI and body's voluntary actions.

## Tests asserting this spec

- `crates/server/src/auras/charm_tests.rs`: native605 faction/combat/control/cancel;205364 follow/duration;1098/30923 ownership/refusal. Ignored RED pushed e2f48c726;0PASS/3FAIL on missing faction consumer. At0568cd86e, unchanged3GREEN/0FAIL32.74s;tests enabled.

## Known gaps (current cycle)

- [ ] Targeted GREEN and real client movement/control proof pending.
- [ ] Load NPC spell slots from canonical `tdb_creature_template_spell` (TDB1210 CreatureID/Index/Spell/VerifiedBuild), preserving source order and TC passive/charmed restrictions. No invented loadouts for creatures without rows.
- [ ] Maker native30923 scheduling not yet enabled; generic aura lifecycle first.

## Out of scope

- Hex51514: native effects56/60/260/321, not aura2/6. No name-derived charm handling.
- Class-specific charmed-player rotations: generic charmed AI attacks for its owner; no fabricated class spell selection.

# Charm and possession

Generic aura2/6 control uses native70448 spells and TrinityCore a352b1fa `Unit::SetCharmedBy`/`RemoveCharmedBy`, `AuraEffect::HandleModPossess`/`HandleModCharm`, `CharmInfo`, `PossessedAI` and `SimpleCharmedPlayerAI`. [Runtime](../wiki/systems/charm-possess.md).

## What it must do

- [x] Native605/205364/1098/30923 assign one charmer, adopt its faction, stop the prior cast/combat/threat, reject a second charmer, and restore identity on aura removal (native aura-boundary3GREEN).
- [x] Ordinary charm follows/attacks for its charmer through pet commands/AI; possession only attacks on command and never AI-chases. Legal duel/arena permissions follow the controller; its controlled unit cannot kill its duel opponent. A revoked connection cannot stop charmer AI attacks.
- [x] Possession routes authenticated PlayerInput to the controlled unit, validates its movement epoch/budget, freezes the player's own body, and restores control on removal.
- [x] Native duration, channel cancellation, damage interrupt flags, death and lost-caster lifetime govern removal; never infer break-on-damage from spell names.
- [x] Existing PetSpells/PetAction show and command charm; cancellation releases control rather than deleting the creature or its pet stable record.

## How it works

- [Charm runtime and source qualifications](../wiki/systems/charm-possess.md).

## Implementation inventory

- `crates/server/src/auras/charm.rs`: original-identity snapshot, UnitCharm/charmer link, apply/remove lifecycle, movement authority.
- `crates/server/src/pet_control.rs`: existing pet commands and AI accept the charmer's controlled unit.
- `crates/server/src/networking{,_movement}.rs`: possessed movement uses existing authoritative movement pipeline.
- `crates/server/src/{combat,creature_aggro,creature_ai,scripted_ai/mod,spell_cast/mod}.rs`: pause original AI and body's voluntary actions.

## Tests asserting this spec

- `crates/server/src/auras/charm_tests.rs`: native605 faction/combat/control/cancel;205364 follow/duration;1098/30923 ownership/refusal. Ignored RED pushed e2f48c726;0PASS/3FAIL on missing faction consumer. At0568cd86e, unchanged3GREEN/0FAIL32.74s;tests enabled.
- `crates/server/src/auras/charm_control_tests.rs` and `scripted_ai/tests/the_maker.rs`: pushed ignored qualification4b1f5f944 gives2PASS/3FAIL19.48s: actual205364 pet melee and native205364/31865 damage flags PASS;1098 genuinely casts on nativeImp416 but3110 bar absent,605 idle speed0,Maker unscheduled are RED.

## Proof and remaining gaps

- [x] Atb93d9ea2f,9 native lifecycle/control/Maker tests GREEN34.41s. The additional duel permission/spell safety checks pass; its late melee assertion crossed native605's30s duration. Atde5bbb5c6 the corrected observation stops at defeat:1GREEN16.77s. All10 qualifications enabled.
- [x] Loaded canonical `tdb_creature_template_spell` (TDB1210 CreatureID/Index/Spell/VerifiedBuild); nativeImp416 duplicate3110 yields one action. Passive/charmed restrictions and slot order are source-qualified implementation, not separately tested branches. Creatures without rows retain a per-creature named source gap; no invented loadouts.
- [x] Maker native30923 source20–30s/repeat20–30s, multiple-player threat gate, actual charmed-player party damage, and solo exclusion pass in both difficulties.
- [ ] Real rendered client movement/control proof remains pending in both skins.

## Out of scope

- Hex51514: native effects56/60/260/321, not aura2/6. No name-derived charm handling.
- Class-specific charmed-player rotations: generic charmed AI attacks for its owner; no fabricated class spell selection.

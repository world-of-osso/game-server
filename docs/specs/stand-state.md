# Stand state

Players sit, sleep, kneel and take chairs; other players see it. Food and drink need the player seated and heal or restore it while seated. How it works: [stand state](../wiki/systems/stand-state.md).

References: TrinityCore a352b1fa `Unit::SetStandState`/`IsSitState`/`IsStandState` (Unit.cpp:10787-10814), `HandleStandStateChangeOpcode` (MiscHandler.cpp:401-415), `HandleMovementOpcode` (MovementHandler.cpp:589-590), `Unit::DealDamage` (Unit.cpp:1185-1186), `Spell::DoProcessTargetContainer` (Spell.cpp:3004-3005), `Unit::_ApplyAura` (Unit.cpp:3584-3586), `RollMeleeOutcomeAgainst` (Unit.cpp:2422-2424), `Player::RegenerateHealth` (Player.cpp:1742-1801), `GameObject::Use` chair (GameObject.cpp:2780-2852), `Player::ProcessItemCast`/`CastItemUseSpell`, `SpellInfo::_LoadSpellSpecific` food/drink.

## What it must do

- [x] Players carry a replicated stand state; a request may set Stand, Sit, Sleep or Kneel only.
- [x] Moving, a non-periodic hit, a hostile harmful spell, casting a spell that does not allow sitting, death and stuns stand the player up.
- [x] Standing up ends Standing-interrupt auras; such an aura sits a standing player.
- [x] Not standing: health regen ×1.5, every non-missed melee swing crits, auras do not proc unless allowed.
- [x] Health regen auras 84, 88, 116, 161 and gear health regen follow `Player::RegenerateHealth`.
- [x] SpellScaling effect amounts (gtSpellScaling) for spells not scaled by item level.
- [x] Using bread or water casts its food or drink, starts the item's 1 s category cooldown, uses one up; food cannot be eaten in combat; a new food replaces the old.
- [x] Chairs seat the user on the nearest free seat at the chair's height.
- [ ] Food/drink visual kits, logout sit, barber chairs, MOD_POWER_REGEN 85, item-level (ATTR11) scaling.

## Tests

- `stand_state_tests.rs`, `spell_cast/regen_tests.rs`, `item_use_tests.rs` (spell harness, real 50 ms ticks, world.db spells and items).
- `combat_tests/outcomes.rs` `a_creature_swing_crits_a_sitting_player_and_stands_it_up`, `melee_tests.rs` `a_swing_at_a_sitting_player_crits_unless_it_misses`.
- `chair_tests.rs` (Northshire bench 151953, Goldshire chair 177499), `item_templates_tests.rs` (bread on-use data).
- `networking_tests/stand_state.rs`: over UDP another client sees a player sit, kneel and stand.
- shared-protocol `protocol_spell_tests.rs` `player_stand_state_follows_trinitycore_sit_and_stand_predicates_and_replicates`.

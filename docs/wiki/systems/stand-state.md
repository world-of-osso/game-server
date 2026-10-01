# Stand state

A player's `UnitData::StandState` replicates as `PlayerStandState` (shared-protocol; creatures keep theirs in `UnitPose`). Code: `crates/server/src/stand_state.rs`, `chair.rs`, item use in `inventory.rs`. Spec: [stand state](../../specs/stand-state.md). All rules cite TrinityCore a352b1fa.

## Setting it

- `StandStateIntent` (`CMSG_STAND_STATE_CHANGE`, `HandleStandStateChangeOpcode`): Stand, Sit, Sleep or Kneel; anything else is dropped. The client sends it for X (`SITORSTAND`) and for /sit, /sleep and /kneel (TrinityCore's text emote handler leaves those to the client). Read every frame in `Update`: unread Lightyear messages do not outlive their frame.
- `set_stand_state` writes the component and marks replication; standing up removes auras with the Standing interrupt flag 0x40000 (food, drink).
- An aura with that flag applied to a standing player sits it (`Unit::_ApplyAura`); a stun stands it up (`SetStunned`).
- Chairs (`GAMEOBJECT_TYPE_CHAIR`, `GameObject::Use`): Data0 seats 1 yd × size apart across the chair's facing, the nearest free seat within 90 yd (a seat is taken while its last user sits on a chair within 0.1 yd); the user is moved there (`teleport::teleported_in`, new movement epoch), faces the chair's way and takes SitLow/Medium/HighChair by Data1.

## Standing up

| Cause | Where | TrinityCore |
|---|---|---|
| Moving or jumping (MOVEMENTFLAG_MASK_MOVING bits of `PlayerMotion`) | `stand_up_moving_players`, FixedUpdate after inputs | MovementHandler.cpp:589-590 stands sit states; sleep/kneel are the client's own request, here the server owns movement |
| Any damage but a periodic tick | `spell_cast::effects::deal_damage`, `combat::deal_direct_damage`, every melee swing (missed too) | Unit.cpp:1185-1186 |
| A harmful spell of a hostile caster | spell hit loop | Spell.cpp:3004-3005, unless ATTR3_DO_NOT_TRIGGER_TARGET_STAND |
| Casting a spell without ATTR0_ALLOW_WHILE_SITTING | `start_cast` | client-side in Retail; TrinityCore never checks the attribute |
| Death | player death transition | Unit.cpp:9218 |

## While not standing

- Health regen ×1.5 (`Player::RegenerateHealth`); see [[spell-casting]].
- Every melee swing that does not miss crits when the crit chance is above 0 (`RollMeleeOutcomeAgainst`, Unit.cpp:2422-2424).
- Auras do not proc without ATTR4_ALLOW_PROC_WHILE_SITTING (`Aura::GetProcEffectMask`).

## Food and drink

- 12.x starter-zone food and water: Tough Hunk of Bread 4540 → Food 1291822 (OBS_MOD_HEALTH 3% of max per second, 20 s), Refreshing Spring Water 159 → Drink 1291950 (OBS_MOD_POWER 1% of max mana per second, first tick on apply by ATTR5_EXTRA_INITIAL_PERIOD). Older foods use MOD_REGEN 84 with SpellScaling amounts (Conjured Pumpernickel → 1127).
- Food replaces food and drink replaces drink (`SPELL_SPECIFIC_FOOD/DRINK`, generic family + Standing flag); a drink stays beside a food.
- Using the item (`process_use_requests`): the first on-use `ItemEffect` (questsrv 22cf305) is cast without a spellbook check; peaceful-only spells (ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL) are refused in combat with `EQUIP_ERR_NOT_IN_COMBAT`; the effect's category goes on its `CategoryCoolDownMSec` (food 11 / drink 59, 1 s); a negative `Charges` uses one up.
- Needs world.db `item_effect` / `item_x_item_effect` (ItemEffect import, startgear/questsrv branches).

## Gaps

- Item spells with a cast time are consumed at cast start, and their ATTR11 item-level scaling is not modelled.
- No food/drink visual kits (`SPELL_VISUAL_KIT_FOOD`/`DRINK` heartbeat), no logout sit, no barber chair, no chair `triggeredEvent`.
- MOD_POWER_REGEN 85 (legacy drinks like 430) has no handler.
- Turning is not a moving flag (`PlayerInput` carries none).

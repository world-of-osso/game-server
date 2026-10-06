# Player motion

Other clients animate a player from its replicated `PlayerMotion`: Retail movement bits from the newest applied `PlayerInput` and authoritative airborne state, plus a project-only jump-origin marker. How it works: [networking](../wiki/systems/networking.md).

References:
- TrinityCore `MovementInfo.h` `MovementFlags`: FORWARD 0x1, BACKWARD 0x2, STRAFE_LEFT 0x4, STRAFE_RIGHT 0x8, WALKING 0x100, FALLING 0x800, SWIMMING 0x100000. `PlayerMotion` keeps these bit values.
- TrinityCore `MovementHandler.cpp` `HandleMovementOpcode`: the server stores the client's `MovementInfo` (`mover->m_movementInfo = movementInfo`) and rebroadcasts it as `SMSG_MOVE_UPDATE` (`SendMessageToSet`, 546-571); the flags travel with every movement update.
- Retail FALLING (0x800) and FALLING_FAR (0x1000) describe falling state; `MovementHandler.cpp` handles `CMSG_MOVE_JUMP` separately. FALLING_FAR is not a jump-origin bit. The project's JUMP_STARTED (bit 63 of the existing `u64`) preserves the jump event's meaning across an airborne interval. No field is added; bincode's variable-width jump flags grow from 3 to 9 bytes, while unjumped FALLING stays 3 bytes.

## What it must do

- [x] Every player spawns with `PlayerMotion(0)` (standing) and it replicates to all clients.
- [x] Each applied input sets the flags: its wire `direction` against `facing_yaw` gives FORWARD or BACKWARD (alignment beyond ±0.5) and STRAFE_RIGHT or STRAFE_LEFT (the facing's right `(-cos, sin)` beyond ±0.5); `running` false sets WALKING; `swimming` sets SWIMMING; `jumping` sets FALLING and JUMP_STARTED.
- [x] The grounded stop input clears direction and airborne flags, keeping WALKING and SWIMMING. An already airborne player's input cannot erase its takeoff origin; gravity decides touchdown.
- [x] The component is written only when the flags change, so repeated identical inputs replicate nothing.
- [x] A blocking cast (speed 0) and a dropped input (taxi control, corpse, map transfer) clear the moving flags.
- [x] Gravity sets FALLING for a ledge/terrain walk-off without JUMP_STARTED; jumps retain both until touchdown. Changes replicate even without another input.
- [ ] TURN_LEFT/TURN_RIGHT and ASCENDING/DESCENDING are not sent: the input carries no turn or vertical intent state.
- [ ] A player who dies mid-run and sends no further input keeps its moving flags until its next input.

## Tests

- `crates/server/src/networking_tests/player_motion.rs` (Lightyear receive path into the production `process_player_inputs`): spawn stands; forward, walk, backward, strafe left/right, swim, running jump and jump in place set their flags at a facing off the world axes; the stop clears them keeping walk and swim; repeated inputs leave the component's change tick; a blocking cast and a corpse input stop the player.
- `player_motion_airborne_walkoff_and_touchdown_replicate_without_jump` crosses a loaded chunk edge through the real input receive → height → gravity ordering, stays airborne without inputs and lands on a lower surface. `player_motion_airborne_jump_keeps_origin_until_touchdown` reports a jumping height, retains its origin without further inputs and clears both on landing.
- shared-protocol `src/protocol_player_motion_tests.rs`: TrinityCore bit values, bitcode and bincode round trips, replication registration.

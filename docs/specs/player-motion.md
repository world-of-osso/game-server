# Player motion

Other clients animate a player from its replicated `PlayerMotion`: the Retail `MovementFlags` of its newest applied `PlayerInput`. How it works: [networking](../wiki/systems/networking.md).

References:
- TrinityCore `MovementInfo.h` `MovementFlags`: FORWARD 0x1, BACKWARD 0x2, STRAFE_LEFT 0x4, STRAFE_RIGHT 0x8, WALKING 0x100, FALLING 0x800, SWIMMING 0x100000. `PlayerMotion` keeps these bit values.
- TrinityCore `MovementHandler.cpp` `HandleMovementOpcode`: the server stores the client's `MovementInfo` (`mover->m_movementInfo = movementInfo`) and rebroadcasts it as `SMSG_MOVE_UPDATE` (`SendMessageToSet`, 546-571); the flags travel with every movement update.
- Other clients animate remote units from those flags: wow_client `src/obj/unit.c` `update_animation` / `get_standing_animation` (FORWARD walk or run by WALKING, BACKWARD walk backwards, strafes, swim variants, FALLING jump start/loop, landing on its clear).

## What it must do

- [x] Every player spawns with `PlayerMotion(0)` (standing) and it replicates to all clients.
- [x] Each applied input sets the flags: its wire `direction` against `facing_yaw` gives FORWARD or BACKWARD (alignment beyond ±0.5) and STRAFE_RIGHT or STRAFE_LEFT (the facing's right `(-cos, sin)` beyond ±0.5); `running` false sets WALKING; `swimming` sets SWIMMING; `jumping` sets FALLING.
- [x] The stop input (no direction, no jump) clears FORWARD/BACKWARD/STRAFE_*/FALLING and keeps WALKING and SWIMMING, as Retail's stop keeps the mode flags.
- [x] The component is written only when the flags change, so repeated identical inputs replicate nothing.
- [x] A blocking cast (speed 0) and a dropped input (taxi control, corpse, map transfer) clear the moving flags.
- [ ] TURN_LEFT/TURN_RIGHT, ASCENDING/DESCENDING and FALLING from walking off a ledge (not a jump) are not sent: the input carries no turn or fall state.
- [ ] A player who dies mid-run and sends no further input keeps its moving flags until its next input.

## Tests

- `crates/server/src/networking_tests/player_motion.rs` (Lightyear receive path into the production `process_player_inputs`): spawn stands; forward, walk, backward, strafe left/right, swim, running jump and jump in place set their flags at a facing off the world axes; the stop clears them keeping walk and swim; repeated inputs leave the component's change tick; a blocking cast and a corpse input stop the player.
- shared-protocol `src/protocol_player_motion_tests.rs`: TrinityCore bit values, bitcode and bincode round trips, replication registration.

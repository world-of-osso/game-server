# Player height

The server adopts the height each `PlayerInput.position` reports, as it adopts x/z, within what swimming, the ground, a jump and a fall allow. How it works: [networking](../wiki/systems/networking.md), [swimming and breath](../wiki/systems/swimming-and-breath.md).

References:
- TrinityCore `MovementHandler.cpp` `HandleMovementOpcode` adopts `MovementInfo.pos` (x, y and z) after `VerifyMovementInfo` (flag sanity, `IsPositionValid`, transport offsets) and checks z only against `GetMinHeight` (fall to void, 594-612). TrinityCore core has no height anti-cheat; AzerothCore delegates it to `sScriptMgr->Anticheat*` hooks (`MovementHandler.cpp:518-590`), implemented out of core. The bounds below are this server's own.
- Water: terrain MH2O, sampled as the Godot client samples it (game-engine `adt_tex_water.rs`, `terrain_height_data.rs`); `shared::movement::SWIM_DEPTH` 1.25, `is_swimming`, `swim_top`.

## What it must do

- [x] The server reads each root ADT's MH2O: per chunk its liquid layers (type, vertex heights by vertex format, exists bitmap) and `deep` mask; the surface over a point is the highest layer covering it. On the Godot swimming spot (Azeroth 32,48, WoW -8558, -490) the seabed is 140.04 and the surface 143.99 (liquid type 5, no deep mask).
- [x] A swimming input over water at least `SWIM_DEPTH` deep moves the player toward the reported point with the time the movement bank grants covering both its horizontal distance at its speed and its vertical distance at `SWIM_SPEED` × the aura speed multiplier; its height is then held between the ground under it and `swim_top` (feet `SWIM_DEPTH` under the surface).
- [x] A swimmer has no gravity: idle, it stays at its height; a fall into swimming water ends without fall damage.
- [x] On land the reported height, moved by the same fraction as x/z, is held above the ground under the new x/z probed from the server's height (no report passes through a floor) and below:
  - without a jump, the ground plus `GROUND_SNAP_THRESHOLD` or the server's height, whichever is higher (no rising off the ground);
  - for a jumping input on the ground, the server's height plus the `JUMP_IMPULSE` apex (2.07 yd) plus 0.25;
  - off the ground, the takeoff height plus the highest point of the arc (`JUMP_IMPULSE` for a jump, 0 for walking off) within the movement bank (±0.5 s) of the time since takeoff, plus 0.25.
- [x] A report out of bounds is corrected to the bound, as x/z are shortened to the bank; the server sends no correction.
- [x] Gravity leaves a height an input set that tick to the client; fall damage still lands on the server from the tracked peak. Without inputs gravity pulls the player to the ground as before.
- [x] Without ground data (no `GroundMap`, unloaded tile) the height stays the server's.
- [x] The server samples active WMO MLIQ tiles after the WMO placement transform, including global WMOs. Liquid type follows the WMO root and group fields. The client still swims in terrain water only.
- [ ] The Godot client's pitched forward swim plus Ascend rises up to twice `SWIM_SPEED`; the server allows `SWIM_SPEED`, so such a climb reaches the server slower than the client shows it.
- [ ] A client reporting `jumping` without end can bunny-hop: each landing allows a new jump arc.

## New-character starts

- [x] New characters use normal TDB1210 `playercreateinfo` map, position and orientation keyed by both race and class. Missing rows fail creation explicitly; no Human/default-coordinate fallback.
- [x] Only documented unimplemented-intro overrides (Demon Hunter Mardum, Dracthyr, Kul Tiran, Earthen, Haranir) and user-approved Skyborne95/96 reconstruction remain. Imported NPE coordinates do not enable an Exile's Reach intro.
- [x] Existing saved positions are retained, not reinterpreted or migrated to a race start.
- [x] A new Tauren Warrior at authored map1(-2915.55,-257.347,59.2693) stays supported on real Mulgore terrain through server gravity. An underground admin teleport does not alter ground/step semantics.

## Tests

- `crates/server/src/terrain_height_tests.rs` `swim_spot_liquid_is_the_surface_the_client_floats_under`, `water_surface_height_separate_from_terrain`.
- `crates/server/src/networking_tests/client_height.rs` (Lightyear receive path, fixed-tick input and gravity): ascend with the client and float at the surface, held at the float height, swim-speed × bank limit, seabed floor on real Azeroth 32,48 terrain; the Godot jump arc followed and landed, a report 20 yd up held to a jump, a report under the ground, a hover brought down, a reported 10 yd fall with fall damage on flat ground.
- shared-protocol `movement.rs` `swimming_starts_where_the_water_stands_swim_depth_over_the_feet`.

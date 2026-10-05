# Skyriding

Skyriding (Retail "dragonriding", Flight Style: Skyriding 404464): momentum flight, Skyriding Charges (vigor) and the Skyriding abilities, server-authoritative. Physics and assumed values: shared-protocol `src/skyriding.rs`; server rules: `crates/server/src/skyriding.rs`, `spell_cast`, `networking_movement.rs`; client: game-engine `godot/rust/src/gameplay.rs`. How it works: [mounts](../wiki/systems/mounts.md).

## What it must do

Flight (part 1)
- [x] A rider on the Skyriding capability (MountCapability 494, aura 406095) can skyride (`CAN_ADV_FLY`); losing it ends the skyride.
- [x] The server moves a skyrider no faster than its energy allows (`speed_after`: v² ≤ v0² + 2·g·drop, capped at MaxVel 65) and cuts a climb at v²/2g.

Skyriding Charges (vigor)
- [x] Surge Forward 372608 and Skyward Ascent 372610 each spend one charge of ChargeCategory 2391 (6 charges) and share them.
- [x] Without a charge the cast fails with NoChargesRemain and nothing flaps.
- [x] On the Skyriding aura a charge recovers every 10.35 s (15 s × (1 − 31%)).
- [x] Second Wind 425782 restores one charge.

Abilities
- [x] Mounted players may cast them (ALLOW_WHILE_MOUNTED); Surge Forward and Skyward Ascent need Skyriding Basics 376777 (LEARN_SPELL 195).
- [x] Surge Forward and Skyward Ascent hold a 1.5 s global cooldown.
- [x] Surge Forward flaps the glider along its facing, Skyward Ascent upward, Whirling Surge 361584 along its facing (no charge, 30 s cooldown); the speed an impulse reaches is capped at AddImpulseMaxSpeed 100.
- [x] The server raises the skyrider's energy by a cast flap and follows the faster glider; a flap it never granted is cut short.
- [x] Aerial Halt 403092 (no charge, 10 s cooldown) multiplies air friction by 100 for 0.5 s, stopping the mount.
- [x] The client flaps only on the server's SpellGo of its own cast.
- [x] Switch Flight Style 436854 swaps Flight Style: Steady 404468 and Skyriding 404464; 460002 sets Skyriding, 460003 Steady. The next mount flies the style (Steady: steady flight capability; Skyriding: capability 494, glider, bonus bar 5, vigor).
- [x] A skyrider's takeoff (Space, Space in the air) casts Skyward Ascent through the server: one charge spent, FORCE_CAST Lift Off 404191 on the rider; the client launches on its SpellGo and the server's bound takes off with that flap only (no charge, no launch).
- [x] The style is saved per character and restored when it enters the world (Steady before its first switch).
- [x] REMOVE_AURA_2 203 removes the effect's TriggerSpell auras from its selected target without an unhandled-effect warning; other spells and units retain their auras.
- [x] An explicitly learned Lift Off 374763 cast while skyriding applies its FORCE_CAST 404191 aura for two seconds.

## Assumed values

Unpublished (Retail runs these in server scripts); one block in shared-protocol `src/skyriding.rs`, tune there:

| Constant | Value | Basis |
|---|---|---|
| `LAUNCH_SPEED` (takeoff, Skyward Ascent) | 31.5 yd/s up | Skyward Ascent / Lift Off effect 0 DUMMY 450, read as 450% × 7 yd/s |
| `SURGE_SPEED` (Surge Forward) | 31.5 yd/s along the facing | same reading; its DUMMY has no base points |
| `WHIRLING_SURGE_SPEED` (Whirling Surge) | 31.5 yd/s along the facing | same as Surge Forward; its DUMMY aura has no base points |

## How it works

- [mounts](../wiki/systems/mounts.md) (Skyriding parts 1 and 2)
- game-engine `docs/wiki/systems/mounts.md`

## Implementation inventory

- shared-protocol `src/skyriding.rs`: `Glider` physics, `Glider::cast`, `impulse_bound`, `ability_impulse`, FlightCapability 11, assumed values.
- `crates/server/src/skyriding.rs`: `SkyridingImpulse`, `flap` (AfterCast).
- `crates/server/src/spell_cast/mod.rs`: charges, `restore_charges`, LEARN_SPELL known rule, AfterCast call.
- `crates/server/src/spell_cast/effects.rs`: MODIFY_CHARGES 293, FORCE_CAST 140, REMOVE_AURA_2 203.
- `crates/server/src/spell_cast/scripts.rs`: `after_cast` registry.
- `crates/server/src/networking.rs`, `networking_movement.rs`: energy bound, `MovementClocks::add_impulse`.
- `crates/server/src/auras/mount.rs`: Skyriding capability, flight style (saved per character, `persistence_progression::CHARACTER_FLIGHT_STYLE`).
- `crates/server/src/spell_cast/scripts.rs`: Switch Flight Style 436854 / 460002 / 460003.
- game-engine `godot/rust/src/gameplay.rs`, `combat_visuals.rs`: client glider, SpellGo flap.

## Tests asserting this spec

- `crates/server/src/skyriding_tests.rs`
- `crates/server/src/networking_tests/skyriding.rs`
- `crates/server/src/auras/mount_tests.rs`
- shared-protocol `src/skyriding_tests.rs`
- game-engine `godot/rust/src/gameplay.rs` (`gameplay::tests`), live `godot/tests/skyriding_live.gd`, `godot/tests/skyriding_abilities_live.gd`, `godot/tests/skyriding_style_live.gd`

## Known gaps (current cycle)

- [ ] Lift Off 374763 (standing launch, no charge) is not cast by anything; no DB2 row or UI code inspected names its trigger. Its DUMMY 450 has no TC launch script; the data has no LEAP/JUMP effect. Native trigger, permission to cast without learning, and the exact not-skyriding failure reason need authoritative evidence before implementation. See [investigation](../wiki/systems/mounts.md#lift-off-374763-investigation).

## Out of scope

- Thrill of the Skies 377234 / Ground Skimming 404184: trait-tree talents with unpublished triggers.
- Aerial Halt's reduced gravity (4 s, amount unpublished); Whirling Surge's spiral visual.

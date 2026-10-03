# Mounts

Player mounts cast as spells (Retail "Steady Flight" and Skyriding flight physics), after TrinityCore master a352b1fa. Code: `auras/mount.rs` (aura hooks, capability), `auras/stat_mods.rs` (speeds), `networking_movement.rs` / `networking.rs` (flight movement, gravity). Client side: game-engine `docs/wiki` mounts page.

## Content

### Data

`world.db` gets the Retail DB2 tables `mount`, `mount_capability`, `mount_type_x_capability` and `mount_x_display` (`scripts/db2.py`; run `python3 scripts/db2.py fetch && python3 scripts/db2.py import` to add them to an existing `world.db`). `MountStore` loads them at startup with the aura spell part of the PlayerConditions they name.

### Mounting (SPELL_AURA_MOUNTED 78)

`AuraEffect::HandleAuraMounted`: applying the aura inserts `Mounted { mount_display_id }` (a `MountXDisplay` of the spell's `Mount` row whose condition the rider meets, at random; `DISPLAYID_HIDDEN_MOUNT` 73200 for a self mount; else a model of the creature template in `EffectMiscValue_0`). The effect's amount stores the `MountCapability` the rider qualifies for (`Unit::GetMountCapability`, `MountTypeXCapability` in `OrderIndex` order: riding skill 762 rank ≥ `ReqRidingSkill`, GROUND flag out of water / UNDERWATER while swimming, `ReqMapID`, `ReqSpellAuraID`, `ReqSpellKnownID`, PlayerCondition aura spells). The rider then gets the capability's `ModSpellAuraID` aura. Removing the mount aura removes `Mounted` (Dismount interrupt flag 0x40 auras go too), every other mount aura and that speed aura.

Golden Gryphon 32235: `Mount` 129, type 424, display 17697. Type 424 capabilities: 494 Skyriding (needs Skyriding Basics 376777 known and PlayerCondition 96927 = NOT Flight Style: Steady; aura 406095, FlightCapability 11), 457/456/455 steady flight at riding 375/300/225 (need Flight Style: Steady 404468) → 86461/86460/86459, then 227/226 ground at 150/75 → 86458/86457.

Every player gets Flight Style: Steady 404468 when its `ActiveAuras` is added (TC `Player::_LoadAuras` `AddAura(404468)`).

Riding: Expert Riding 34090's SKILL effect sets Riding to 225; `SkillRaceClassInfo.Flags` `SKILL_FLAG_ALWAYS_MAX_VALUE` (0x10) makes the rank the tier maximum (Player.cpp:3004).

### Speeds and flight

- `AuraStatMods::movement_speed`: under a mount aura, strongest MOD_INCREASE_MOUNTED_SPEED 32 × MOD_MOUNTED_SPEED_ALWAYS 130 (else the unmounted run auras), then slows.
- `AuraStatMods::flight_speed`: under a mount aura, strongest MOD_INCREASE_MOUNTED_FLIGHT_SPEED 207 × MOD_MOUNTED_FLIGHT_SPEED_ALWAYS 208. 86459 gives 3.2 × `FLIGHT_SPEED` 7 = 22.4 yd/s.
- `PlayerMotion::CAN_FLY` while a FLY 201 or 207 aura is on (`Unit::SetCanFly`); losing it clears `FLYING`.
- A `PlayerInput { flying: true }` counts only with `CAN_FLY` (MovementHandler.cpp:128). It sets `FLYING`, moves at the flight speed over the 3D distance, and holds the height only to the ground under it. `apply_terrain_gravity` skips a `FLYING` player and keeps its `FallTracker` grounded where it is, so landing does no damage and a dismount in the air falls from there.

### Skyriding (part 1: physics and server bound)

- Switch a test character (online): `game-server-admin flight-style <name> skyriding` (removes Flight Style: Steady 404468, applies Flight Style: Skyriding 404464, teaches Skyriding Basics 376777); `flight-style <name> steady` switches back. Mount again after switching. Login puts Steady back (TC `Player::_LoadAuras` forces 404468), until the Retail toggle (part 3).
- Skyriding aura 406095: ADV_FLYING 446 → `PlayerMotion::CAN_FLY | CAN_ADV_FLY` (TC `HandleModAdvFlying` → `SetCanFly`, `SetCanAdvFly`); MOD_INCREASE_MOUNTED_SPEED 100%. Losing it drops `ADV_FLYING`.
- A `flying` input from a `CAN_ADV_FLY` player sets `FLYING | ADV_FLYING` and moves under the energy bound of shared-protocol `skyriding` instead of the flight speed: the server tracks each skyrider's most possible speed (`MovementClocks`, starting at the mounted run speed with the 31.5 yd/s launch on top), moves it toward the report at the faster end of the leg (`speed_after`: v² ≤ v0² + 2·g·drop, capped at FlightCapability 11 `MaxVel` 65), cuts a climb at `v²/2g`, and keeps the bank and unreached-report catch-up of every other travel. Drag is not part of the bound (upper bound only). The momentum model and its constants are documented in shared-protocol `src/skyriding.rs`.

### Skyriding (part 2: vigor and abilities)

Code: `skyriding.rs` (flaps), `spell_cast` (charges, `restore_charges`, LEARN_SPELL), `networking.rs` / `networking_movement.rs` (`MovementClocks::add_impulse`). Retail DB2 12.1.0.69933; TC a352b1fa.

| What | Value | Source |
|---|---|---|
| Vigor | Skyriding Charges: `ChargeCategory` 2391 "Skryriding Charges - Core", MaxCharges 6, ChargeRecoveryTime 15000 (no power: SpellPower 372608 is PowerType 25 cost 0) | SpellCategories 138578/138579, SpellCategory 2391 |
| Regen while skyriding | × 0.69 → 10.35 s a charge | 406095 effect 8 CHARGE_RECOVERY_MULTIPLIER 454 -31 misc 2391; TC `SpellHistory::GetChargeRecoveryTime` |
| Surge Forward 372608 | 1 charge, GCD 1.5 s (cat 133), cooldown 1.5 s, cat 2316 1.5 s; effect 0 DUMMY | SpellCooldowns 77887, SpellEffect 997336 |
| Skyward Ascent 372610 | 1 charge, cooldown 1.5 s; effect 0 DUMMY 450 | SpellCooldowns 77888, SpellEffect 997338 |
| Aerial Halt 403092 | no charge, cooldown 10 s, GCD 1.5 s; MOD_ADV_FLYING_AIR_FRICTION 513 10000% for 0.5 s | SpellCooldowns 83556, SpellEffect 1049834, SpellDuration 327 |
| Whirling Surge 361584 | no charge, cat 2193 cooldown 30 s; DUMMY aura 3 s | SpellCooldowns 77889, SpellCategories 138580 |
| Second Wind 425782 | MODIFY_CHARGES 293 +1 on 2391; own charges 3 / 180 s (cat 2195) | SpellEffect 1103859, SpellCategory 2195 |
| Impulse cap | AddImpulseMaxSpeed 100, OverMaxDeceleration 7 | FlightCapability 11 |

- Learning: Skyriding Basics 376777's LEARN_SPELL 195 effects teach Surge Forward and Skyward Ascent (TC `AuraEffect::HandleLearnSpell`): a spell taught by a known (or profession-learned) spell's 195 effect counts as known. Aerial Halt, Whirling Surge and Second Wind come from the Skyriding trait tree in Retail; test characters `learn-spell` them.
- All five have `SPELL_ATTR0_ALLOW_WHILE_MOUNTED` (0x01000000), so `NotMounted` lets them through.
- The flap: Surge Forward / Skyward Ascent effect 0 (DUMMY, the caster) runs `skyriding::surge_forward` / `skyward_ascent` (`spell_cast::scripts`), which leaves a `SkyridingImpulse` of 31.5 yd/s (shared-protocol `ability_impulse`); the next `process_player_inputs` raises the skyrider's energy to `impulse_bound` (v + 31.5, capped at 100) and drops it. A grounded player has no energy yet; its takeoff starts one. The client flaps its `Glider` on the `SpellGo`.
- Second Wind: `EffectModifySpellCharges` → `restore_charges` (TC `RestoreCharge`: the most recent used charge comes back, the oldest keeps recovering), and a `SpellChargesUpdate` named after the last spell that used the category.
- Not modelled: the impulse sizes are assumptions (Retail runs these DUMMY effects in an unpublished server script; TC defines `SMSG_MOVE_ADD_IMPULSE` but never sends it); Thrill of the Skies 377234 (-42%) and Ground Skimming 404184 (-20%), trait-tree talents 383366/404183 whose triggers are server scripts; Dragon's Rest 372606; Whirling Surge movement; Aerial Halt's reduced gravity; the Space launch spending a charge (Retail's takeoff is Skyward Ascent / Lift Off).
- Tests: `skyriding_tests.rs` (charge spent, shared charges, GCD, drain then NoChargesRemain, 10.35 s recovery, Aerial Halt cooldown, Second Wind, needs Skyriding Basics), `networking_tests/skyriding.rs` (a granted Surge Forward is followed within 0.05 yd; the same flap ungranted falls behind). Live: game-engine `godot/tests/skyriding_abilities_live.gd`.

### Casting rules

`validate_start`: `SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL` fails in combat (`AffectingCombat`, mounts, food); a mounted player casts only passive or `SPELL_ATTR0_ALLOW_WHILE_MOUNTED` spells (`NotMounted`). `CancelMountAura` (`CMSG_CANCEL_MOUNT_AURA`) removes the cancellable mount auras. A swimming input still dismounts (a pre-existing rule, now through the mount aura); Retail keeps the mount in water.

### Learning a mount

Retail path: vendors 20510/32216/35101 sell item 25470 (Golden Gryphon, `ItemEffect` 55884 LEARN_SPELL with the learned 32235), and riding trainers teach 33392 → 34092 (Expert Riding). Item learn-spell triggers are not modelled, so test characters use `game-server-admin learn-spell <name> 34090` and `learn-spell <name> 32235` (online character; saved with `PlayerProfessions`).

### Not modelled

Skyriding's 85% speed outside the Dragon Isles (Warcraft Wiki), `AreaTable.MountFlags` (no area table: ground and flying allowed everywhere), `ReqAreaID` capabilities, PlayerCondition ModifierTrees, capability recalculation on water/area/skill change (`UpdateMountCapability`), backward flight speed (`MOVE_FLIGHT_BACK` 4.5), unmounted flight auras (206), auto-dismount before other casts.

## See also

- [[auras]], [[networking]], [[swimming-and-breath]], [[professions]]

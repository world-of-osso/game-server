# Mounts

Player mounts cast as spells (Retail "Steady Flight", not Skyriding), after TrinityCore master a352b1fa. Code: `auras/mount.rs` (aura hooks, capability), `auras/stat_mods.rs` (speeds), `networking_movement.rs` / `networking.rs` (flight movement, gravity). Client side: game-engine `docs/wiki` mounts page.

## Content

### Data

`world.db` gets the Retail DB2 tables `mount`, `mount_capability`, `mount_type_x_capability` and `mount_x_display` (`scripts/db2.py`; run `python3 scripts/db2.py fetch && python3 scripts/db2.py import` to add them to an existing `world.db`). `MountStore` loads them at startup with the aura spell part of the PlayerConditions they name.

### Mounting (SPELL_AURA_MOUNTED 78)

`AuraEffect::HandleAuraMounted`: applying the aura inserts `Mounted { mount_display_id }` (a `MountXDisplay` of the spell's `Mount` row whose condition the rider meets, at random; `DISPLAYID_HIDDEN_MOUNT` 73200 for a self mount; else a model of the creature template in `EffectMiscValue_0`). The effect's amount stores the `MountCapability` the rider qualifies for (`Unit::GetMountCapability`, `MountTypeXCapability` in `OrderIndex` order: riding skill 762 rank ≥ `ReqRidingSkill`, GROUND flag out of water / UNDERWATER while swimming, `ReqMapID`, `ReqSpellAuraID`, `ReqSpellKnownID`, PlayerCondition aura spells). The rider then gets the capability's `ModSpellAuraID` aura. Removing the mount aura removes `Mounted` (Dismount interrupt flag 0x40 auras go too), every other mount aura and that speed aura.

Golden Gryphon 32235: `Mount` 129, type 424, display 17697. Type 424 capabilities: 494 Skyriding (needs 376777, not known), 457/456/455 steady flight at riding 375/300/225 (need Flight Style: Steady 404468) → 86461/86460/86459, then 227/226 ground at 150/75 → 86458/86457.

Every player gets Flight Style: Steady 404468 when its `ActiveAuras` is added (TC `Player::_LoadAuras` `AddAura(404468)`).

Riding: Expert Riding 34090's SKILL effect sets Riding to 225; `SkillRaceClassInfo.Flags` `SKILL_FLAG_ALWAYS_MAX_VALUE` (0x10) makes the rank the tier maximum (Player.cpp:3004).

### Speeds and flight

- `AuraStatMods::movement_speed`: under a mount aura, strongest MOD_INCREASE_MOUNTED_SPEED 32 × MOD_MOUNTED_SPEED_ALWAYS 130 (else the unmounted run auras), then slows.
- `AuraStatMods::flight_speed`: under a mount aura, strongest MOD_INCREASE_MOUNTED_FLIGHT_SPEED 207 × MOD_MOUNTED_FLIGHT_SPEED_ALWAYS 208. 86459 gives 3.2 × `FLIGHT_SPEED` 7 = 22.4 yd/s.
- `PlayerMotion::CAN_FLY` while a FLY 201 or 207 aura is on (`Unit::SetCanFly`); losing it clears `FLYING`.
- A `PlayerInput { flying: true }` counts only with `CAN_FLY` (MovementHandler.cpp:128). It sets `FLYING`, moves at the flight speed over the 3D distance, and holds the height only to the ground under it. `apply_terrain_gravity` skips a `FLYING` player and keeps its `FallTracker` grounded where it is, so landing does no damage and a dismount in the air falls from there.

### Casting rules

`validate_start`: `SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL` fails in combat (`AffectingCombat`, mounts, food); a mounted player casts only passive or `SPELL_ATTR0_ALLOW_WHILE_MOUNTED` spells (`NotMounted`). `CancelMountAura` (`CMSG_CANCEL_MOUNT_AURA`) removes the cancellable mount auras. A swimming input still dismounts (a pre-existing rule, now through the mount aura); Retail keeps the mount in water.

### Learning a mount

Retail path: vendors 20510/32216/35101 sell item 25470 (Golden Gryphon, `ItemEffect` 55884 LEARN_SPELL with the learned 32235), and riding trainers teach 33392 → 34092 (Expert Riding). Item learn-spell triggers are not modelled, so test characters use `game-server-admin learn-spell <name> 34090` and `learn-spell <name> 32235` (online character; saved with `PlayerProfessions`).

### Not modelled

`AreaTable.MountFlags` (no area table: ground and flying allowed everywhere), `ReqAreaID` capabilities, PlayerCondition ModifierTrees, capability recalculation on water/area/skill change (`UpdateMountCapability`), backward flight speed (`MOVE_FLIGHT_BACK` 4.5), unmounted flight auras (206), auto-dismount before other casts.

## See also

- [[auras]], [[networking]], [[swimming-and-breath]], [[professions]]

# Mounts

Player mounts cast as spells (Retail "Steady Flight" and Skyriding flight physics), after TrinityCore master a352b1fa. Code: `auras/mount.rs` (aura hooks, capability), `auras/stat_mods.rs` (speeds), `networking_movement.rs` / `networking.rs` (flight movement, gravity). Client side: game-engine `docs/wiki` mounts page.

## Content

### Data

`world.db` gets the Retail DB2 tables `mount`, `mount_capability`, `mount_type_x_capability` and `mount_x_display` (`scripts/db2.py`; run `python3 scripts/db2.py fetch && python3 scripts/db2.py import` to add them to an existing `world.db`). `MountStore` loads them at startup with the aura spell part of the PlayerConditions they name.

### Mounting (SPELL_AURA_MOUNTED 78)

`AuraEffect::HandleAuraMounted`: applying the aura inserts `Mounted { mount_display_id }` (a `MountXDisplay` of the spell's `Mount` row whose condition the rider meets, at random; `DISPLAYID_HIDDEN_MOUNT` 73200 for a self mount; else a model of the creature template in `EffectMiscValue_0`). The effect's amount stores the `MountCapability` the rider qualifies for (`Unit::GetMountCapability`, `MountTypeXCapability` in `OrderIndex` order: riding skill 762 rank ≥ `ReqRidingSkill`, GROUND flag out of water / UNDERWATER while swimming, `ReqMapID`, `ReqSpellAuraID`, `ReqSpellKnownID`, PlayerCondition aura spells). The rider then gets the capability's `ModSpellAuraID` aura. Removing the mount aura removes `Mounted` (Dismount interrupt flag 0x40 auras go too), every other mount aura and that speed aura.

Golden Gryphon 32235: `Mount` 129, type 424, display 17697. Type 424 capabilities: 494 Skyriding (needs Skyriding Basics 376777 known and PlayerCondition 96927 = NOT Flight Style: Steady; aura 406095, FlightCapability 11), 457/456/455 steady flight at riding 375/300/225 (need Flight Style: Steady 404468) → 86461/86460/86459, then 227/226 ground at 150/75 → 86458/86457.

Every player enters the world with its saved Flight Style aura (redb `character_flight_style`: Steady 404468 or Skyriding 404464), Steady before it first switched. TC `Player::_LoadAuras` always adds 404468 ("TODO: finish dragonriding"); Retail keeps the style across logins.

Switch Flight Style (part 3): its spells (5 s cast, SpellCastTimes 6) carry only DUMMY 0 on the caster, so spell scripts (`spell_cast/scripts.rs`, `OnEffectHitTarget` like a TC `SpellEffectFn`) switch: 436854 (the Riding skill line's ability, spelldesc `$?A404468[Steady Flight][Skyriding]`) swaps the two, 460002 (icon ability_dragonriding_dynamicflight01) puts the player on Skyriding, 460003 (ability_dragonriding_staticflight01) on Steady. The switch saves the style; the next mount picks its capability (both style auras are DUMMY auras, which TC `UpdateMountCapability` does not follow). No DB2 row teaches 436854 (`SkillLineAbility` AcquireMethod 0): `learn-spell <name> 436854`.

Riding: Expert Riding 34090's SKILL effect sets Riding to 225; `SkillRaceClassInfo.Flags` `SKILL_FLAG_ALWAYS_MAX_VALUE` (0x10) makes the rank the tier maximum (Player.cpp:3004).

### Speeds and flight

- `AuraStatMods::movement_speed`: under a mount aura, strongest MOD_INCREASE_MOUNTED_SPEED 32 × MOD_MOUNTED_SPEED_ALWAYS 130 (else the unmounted run auras), then slows.
- `AuraStatMods::flight_speed`: under a mount aura, strongest MOD_INCREASE_MOUNTED_FLIGHT_SPEED 207 × MOD_MOUNTED_FLIGHT_SPEED_ALWAYS 208. 86459 gives 3.2 × `FLIGHT_SPEED` 7 = 22.4 yd/s.
- `PlayerMotion::CAN_FLY` while a FLY 201 or 207 aura is on (`Unit::SetCanFly`); losing it clears `FLYING`.
- A `PlayerInput { flying: true }` counts only with `CAN_FLY` (MovementHandler.cpp:128). It sets `FLYING`, moves at the flight speed over the 3D distance, and holds the height only to the ground under it. `apply_terrain_gravity` skips a `FLYING` player and keeps its `FallTracker` grounded where it is, so landing does no damage and a dismount in the air falls from there. An input's flags are its state at the position it reports (TC `UpdatePosition(movementInfo.pos)`): a `FLYING` player flies to a `flying: false` report and drops `FLYING` only on reaching it, so a server catching up after a client stall flies the rest of the way and lands where the client landed (`a_stalled_flyer_landing_ahead_of_the_server_lands_without_falling`, skyriding too). Live 2026-10-03 (UDP 5193, `skyriding_live.gd` with `SKY_STALL_FPS=1`): the client landed with the server 8.7 yd up and 19 yd behind, still flying; the server landed on the same spot with health 292 → 292 (engine `data/diagnostics/landfall-2026-10-03/`).

### Skyriding (part 1: physics and server bound)

- Switch a test character (online): `game-server-admin flight-style <name> skyriding` (removes Flight Style: Steady 404468, applies Flight Style: Skyriding 404464, teaches Skyriding Basics 376777); `flight-style <name> steady` switches back. Mount again after switching. The style is saved for the next login.
- Skyriding aura 406095: ADV_FLYING 446 → `PlayerMotion::CAN_FLY | CAN_ADV_FLY` (TC `HandleModAdvFlying` → `SetCanFly`, `SetCanAdvFly`); MOD_INCREASE_MOUNTED_SPEED 100%. Losing it drops `ADV_FLYING`.
- A `flying` input from a `CAN_ADV_FLY` player sets `FLYING | ADV_FLYING` and moves under the energy bound of shared-protocol `skyriding` instead of the flight speed: the server tracks each skyrider's most possible speed (`MovementClocks`, starting at the mounted run speed with the flaps cast on the ground on top: the takeoff's Skyward Ascent gives the 31.5 yd/s launch; without it the takeoff has the run speed only), moves it toward the report at the faster end of the leg (`speed_after`: v² ≤ v0² + 2·g·drop, capped at FlightCapability 11 `MaxVel` 65), cuts a climb at `v²/2g`, and keeps the bank and unreached-report catch-up of every other travel. Drag is not part of the bound (upper bound only). The momentum model and its constants are documented in shared-protocol `src/skyriding.rs`.

### Skyriding (part 2: vigor and abilities)

Code: `skyriding.rs` (flaps), `spell_cast` (charges, `restore_charges`, LEARN_SPELL), `networking.rs` / `networking_movement.rs` (`MovementClocks::add_impulse`). Retail DB2 12.1.0.69933; TC a352b1fa.

| What | Value | Source |
|---|---|---|
| Vigor | Skyriding Charges: `ChargeCategory` 2391 "Skryriding Charges - Core", MaxCharges 6, ChargeRecoveryTime 15000 (no power: SpellPower 372608 is PowerType 25 cost 0) | SpellCategories 138578/138579, SpellCategory 2391 |
| Regen while skyriding | × 0.69 → 10.35 s a charge | 406095 effect 8 CHARGE_RECOVERY_MULTIPLIER 454 -31 misc 2391; TC `SpellHistory::GetChargeRecoveryTime` |
| Surge Forward 372608 | 1 charge, GCD 1.5 s (cat 133), cooldown 1.5 s, cat 2316 1.5 s; effect 0 DUMMY | SpellCooldowns 77887, SpellEffect 997336 |
| Skyward Ascent 372610 | 1 charge, cooldown 1.5 s; effect 0 DUMMY 450 | SpellCooldowns 77888, SpellEffect 997338 |
| Aerial Halt 403092 | no charge, cooldown 10 s, GCD 1.5 s; MOD_ADV_FLYING_AIR_FRICTION 513 10000% for 0.5 s | SpellCooldowns 83556, SpellEffect 1049834, SpellDuration 327 |
| Whirling Surge 361584 | no charge, cat 2193 cooldown 30 s; DUMMY aura 3 s; forward impulse (assumed) | SpellCooldowns 77889, SpellCategories 138580 |
| Second Wind 425782 | MODIFY_CHARGES 293 +1 on 2391; own charges 3 / 180 s (cat 2195) | SpellEffect 1103859, SpellCategory 2195 |
| Impulse cap | AddImpulseMaxSpeed 100, OverMaxDeceleration 7 | FlightCapability 11 |

- Learning: Skyriding Basics 376777's LEARN_SPELL 195 effects teach Surge Forward and Skyward Ascent (TC `AuraEffect::HandleLearnSpell`): a spell taught by a known (or profession-learned) spell's 195 effect counts as known. Aerial Halt, Whirling Surge and Second Wind come from the Skyriding trait tree in Retail; test characters `learn-spell` them.
- All five have `SPELL_ATTR0_ALLOW_WHILE_MOUNTED` (0x01000000), so `NotMounted` lets them through.
- The flap: after a Surge Forward, Skyward Ascent or Whirling Surge cast, `skyriding::flap` (`spell_cast::scripts::after_cast`) leaves a `SkyridingImpulse` of the ability's assumed speed (shared-protocol `ability_impulse`; values in [the spec](../../specs/skyriding.md)); the next `process_player_inputs` raises the skyrider's energy to `impulse_bound` (v + 31.5, capped at 100) and drops it. A grounded player has no energy yet: its flap is kept as the launch its takeoff starts with. The client flaps its `Glider` on the `SpellGo`.
- The takeoff (part 4): Retail's takeoff is JUMP, JUMP (Blizzard_Tutorials_RPE.lua:391-395). A skyrider's Space in the air sends Skyward Ascent 372610 instead of launching (game-engine `gameplay.rs` `take_takeoff_request`); the cast spends a Skyriding Charge, its effect 1 FORCE_CAST 140 makes the rider cast Lift Off 404191 (DUMMY aura, 2 s) on itself (TC `Spell::EffectForceCast`, SpellEffects.cpp:791-849; `cast_triggered` ignores the forced cast's costs, which TC keeps; Lift Off has none), and its `SpellGo` launches the client's glider. Without a charge the cast fails and the jump lands. Effects 2/3 (REMOVE_AURA_2 203 of the tutorial highlight auras 1259215/1259214) are not modelled. Lift Off 374763 ("When standing, launch upward", no ChargeCategory) also force-casts 404191; no DB2 row or UI code says what casts it.
- Second Wind: `EffectModifySpellCharges` → `restore_charges` (TC `RestoreCharge`: the most recent used charge comes back, the oldest keeps recovering), and a `SpellChargesUpdate` named after the last spell that used the category.
- Not modelled: exact impulse sizes (assumed; Retail runs these DUMMY effects in an unpublished server script; TC defines `SMSG_MOVE_ADD_IMPULSE` but never sends it); Thrill of the Skies 377234 (-42%) and Ground Skimming 404184 (-20%), trait-tree talents 383366/404183 whose triggers are server scripts; Dragon's Rest 372606; Whirling Surge's spiral; Aerial Halt's reduced gravity; the Space launch spending a charge (Retail's takeoff is Skyward Ascent / Lift Off).
- Tests: `skyriding_tests.rs` (charge spent, Whirling Surge, shared charges, GCD, drain then NoChargesRemain, 10.35 s recovery, Aerial Halt cooldown, Second Wind, needs Skyriding Basics), `networking_tests/skyriding.rs` (a granted Surge Forward is followed within 0.05 yd; the same flap ungranted falls behind). Live: game-engine `godot/tests/skyriding_abilities_live.gd`.

### Skyriding (part 3: the Skyriding bar)

- Gaining ADV_FLYING (`auras::mount::reconcile_can_fly`) runs `skyriding::fill_skyriding_bar`: each Skyriding ability the rider can cast (`spell_cast::caster_knows_spell`) goes on bonus bar 5 (slots 120..124: Surge Forward, Skyward Ascent, Aerial Halt, Whirling Surge, Second Wind) when its slot is empty and it is not on the bar yet, and the owner gets the `ActionBarSnapshot` (`class_progression::place_actions`). Bonus bar 5 is what the client's main bar shows while skyriding (game-engine mounts page).
- Not persisted: the login prune keeps only class progression spells, so the slots are refilled on every mount. The bar order is assumed (no DB2 table or TC code lists it).
- Action slots are 0..179 (TC Player.h `MAX_ACTION_BUTTONS` 180); `SetActionButton` rejects 180+.
- `SpellChargesUpdate.category` is the `ChargeCategory` (TC `SMSG_SET_SPELL_CHARGES` Category), so the client tracks Skyriding Charges 2391 whichever ability spent them.
- Test: `skyriding_tests::a_skyriding_mount_fills_bonus_bar_5_with_the_known_abilities`.

### Casting rules

`validate_start`: `SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL` fails in combat (`AffectingCombat`, mounts, food); a mounted player casts only passive or `SPELL_ATTR0_ALLOW_WHILE_MOUNTED` spells (`NotMounted`). `CancelMountAura` (`CMSG_CANCEL_MOUNT_AURA`) removes the cancellable mount auras. A swimming input still dismounts (a pre-existing rule, now through the mount aura); Retail keeps the mount in water.

### Learning a mount

Retail path: vendors 20510/32216/35101 sell item 25470 (Golden Gryphon, `ItemEffect` 55884 LEARN_SPELL with the learned 32235), and riding trainers teach 33392 → 34092 (Expert Riding). Item learn-spell triggers are not modelled, so test characters use `game-server-admin learn-spell <name> 34090` and `learn-spell <name> 32235` (online character; saved with `PlayerProfessions`).

### Not modelled

Skyriding's 85% speed outside the Dragon Isles (Warcraft Wiki), `AreaTable.MountFlags` (no area table: ground and flying allowed everywhere), `ReqAreaID` capabilities, PlayerCondition ModifierTrees, capability recalculation on water/area/skill change (`UpdateMountCapability`), backward flight speed (`MOVE_FLIGHT_BACK` 4.5), unmounted flight auras (206), auto-dismount before other casts.

## See also

- [[auras]], [[networking]], [[swimming-and-breath]], [[professions]]

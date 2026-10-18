# Mounts

Player mounts cast as spells (Retail "Steady Flight" and Skyriding flight physics), after TrinityCore master a352b1fa. Code: `auras/mount.rs` (aura hooks, capability), `auras/stat_mods.rs` (speeds), `networking_movement.rs` / `networking.rs` (flight movement, gravity). Client side: game-engine `docs/wiki` mounts page.

## Content

### Mount Journal

Contract: [Mount Journal authority](../../specs/mount-journal.md). `mount_journal.rs` reads authentic Mount/ItemEffect/creature/vehicle/capability joins; `persistence_mounts.rs` stores Mount ID→favourite outside frozen character serialization. Direct learn items and inventory consumption commit atomically. Existing profession/class learned source spells add ownership; cast validation consults the account collection without injecting fake class spells.

One exclusive Update step initializes accounts, publishes context changes, then handles favourite/summon/random/dismiss requests. Auth can apply deferred InWorld/PlayerLink before or after that step, but no journal reader runs before initialization. Both the original PostUpdate publication crash and an early world-entry request crash were reproduced by file-backed/HostClient regressions; initialization/publication/request orchestration now shares that boundary. HostClient delivers in Last, so tests observe the initial update before next-frame clearing.

Summon uses `cast_for_player`, and dismount uses `cancel_mount_aura`; existing riding/area/flight rules remain authoritative. Random candidates are owned usable favourites. Authored visibility flags and capability/positive-swim/FlightCapability/non-control passenger-seat categories are transmitted to the native journal; these categories are data-derived, not a claim of unpublished client classification parity.

Verified2026-10-12 after merging master99f61aec: 173 targeted `mount`/`merchant::`/`auction` server tests pass; private UDP5901 native runs pass in both skins for paid Katie Hunter item5655→Mount18, alternate-character persistence and two-account AH listing/buyout/mail/UseItem of unbound item44413→Mount275. World.db is read-only; existing1694 mount rows need no new import.

### Vendor acquisition

`merchant/mount_costs.rs` loads pinned build69933 ItemExtendedCost rows selected by TDB1210 teaching-mount offers. The catalog replaces matching imported mount rows with exact TDB offers, preserves ordinary offers and checks the catalog slot on purchase. Cost Money overrides item BuyPrice. Item/currency amounts multiply by purchase count; `_100_Scaler` currency precision divides raw counts by100 exactly. Missing metadata disables the whole affected cost, never substitutes IDs or invents names. [Data provenance and reproduction](../../mount-vendor-costs.md).

Detached inventory/wallet copies stage payment before delivery. One redb write commits gold, consumed items, delivered items and currency; only then does the server replace PlayerCurrencies and enqueue gold/bag/currency updates. Mount ItemSparse reputation requirements use CharacterReputation standing for listing usability and purchase rejection. Existing faction filtering remains. No serialized character fields, global currency definitions or reputation discounts change.

Targeted HostClient tests: eight `merchant_mount_cost_tests.rs` tests passed at e94f73a3; original gold-only purchase→learn/no-transmog passed at bf0ad0c5. [Proof and unrelated fixture gap](../../mount-vendor-costs.md#development-proof). Native client and integration acceptance belong to the main Mount Journal goal.

### Data

`world.db` gets the Retail DB2 tables `mount`, `mount_capability`, `mount_type_x_capability` and `mount_x_display` (`scripts/db2.py`; run `python3 scripts/db2.py fetch && python3 scripts/db2.py import` to add them to an existing `world.db`). `MountStore` loads them at startup with the aura spell part of the PlayerConditions they name.

### IoC Demolisher seats — implementation2026-10-15

`vehicle/isle.rs` handles only source Demolisher34775 on map628. Its normal66245 control aura236 selects the first free authored Vehicle509 seat; driver seat5766 is not excluded by the passenger-mount non-control-seat rule. A server-only NPC occupancy component binds each rider to its exact control-aura instance. Existing VehiclePassenger carries the NPC's server identity and source seat; rider movement control and source VehicleSeat/M2 bind-pose placement follow the seat. Aura removal, exit and reconciliation release occupancy; player-owned mount boarding remains a separate unchanged request path.

`isle_vehicle_models.sql` selects the authored display/VehicleId/M2 for the catalog. Source M2123367 exists locally. The deploy-CLI regression reproduces its omission while retaining the passenger mount (`isle-vehicle-deploy-closure-red1.log`,8926e11c); ground_files.py now reads both shared SQL catalogs. Shared semantic epoch2 distinguishes NPC-driver identities from old mount-only consumers. Driver boarding GREEN at e1a2dc8f; source pose, normal wire exit/reentry/Leave/NewWorld/ACK plus vehicle/rider lethal-damage release controls pass3/3 at56c27190 (`isle-vehicle-seat-lifecycle2.log`). Deploy present/missing/housing controls pass4/4 at210ed823. Merged serverd414bb03/protocol5ce52e0 passes19 foundation cases (four real-channel Demolisher lifetimes/occupancy, twelve passenger mounts, source faction, two aura controls), eight bomb/AV/arena controls and runtime check; protocol semantic-mismatch/matching handshake2 passes. Client scene/UDP gates are recorded in the [client attachment proof](../../../../game-engine/docs/wiki/systems/battlegrounds.md#ioc-npc-attachment--implementation2026-10-15). Actual model attachment, movement, action bar/fire and both-skin vehicle proof remain pending.

Multiple occupants expose a separate stacking RED at5b2a0569: the second friendly caster takes driver0 rather than free seat2 because generic beneficial-aura replacement removes the first caster's236 aura. TC `SpellAuras.cpp` CanStackWith (1621–1632 in the pinned source) keeps independent vehicle-control auras while seats are available. Stacking now retains each caster's236 aura; the vehicle handler still owns free-seat/overfill admission and removal of rejected applications. The same real-channel occupancy/overfill/reuse test passes at31939668 (`isle-vehicle-occupancy-green1.log`,18.90s): seats0/2/3 retain separate caster auras, a fourth rider is refused without displacing them, and normal exit frees seat2 for reuse. Generic236 coverage outside the implemented vehicle context is not claimed. [IoC faction prerequisites](factions.md#isle-per-spawn-script-factions--implementation2026-10-15) retain source metadata, not fixture-only overrides.

### Vehicle movement metadata — implementation2026-10-15

The prior server catalog imported only Vehicle.ID/SeatID_0..7; normal Demolisher509 driving also needs the installed Flags and TurnSpeed. Current local CSV values1594726007/1.04719758 were dropped by the importer; the existing-seat-only upgrade regression reproduces missing Flags (`isle-vehicle-driving-metadata-red1.log`,b7aef39f). The importer now projects both fields, clears old rows inside its transaction before adding required columns without synthetic defaults, then inserts every authored value. Fresh imports and repeat imports use the same columns. Fresh/seat-only-upgrade/repeat/unrelated-data and missing-TurnSpeed preflight tests pass3/3 at39f42801 (`isle-vehicle-driving-metadata-green1.log`). Runtime driving and canonical publication remain unproved.

Lead-only publication after the importer gate: `python3 scripts/import-vehicles.py ../game-engine/data/db2/active data/world.db`. Agents use disposable databases; no canonical write accompanies this implementation. The movement consumer still needs its own input/control and native proof.

### Demolisher spellclick publication — implementation2026-10-15

Normal promotion omitted authored spellclicks. The first34775-only fix was rejected by lead13:17: promotion must copy all source spellclick rows for every entry spawned on the promoted maps. An authentic map628+559 process RED (`isle-general-spellclick-publication-red1.log`,d3414920) expects nine source rows across seven NPC entries but gets only34775. `promote_battlegrounds.py` now deletes/reinserts the complete target-entry set from `tdb_npc_spellclick_spells` in the population transaction, preserves other entries and rolls back when the source table is absent. Empty per-entry source sets remove stale rows; no NPC-specific defaults or row requirement. Generalized GREEN4/4 atd0c7a255 (`isle-general-spellclick-publication-green1.log`,168.538s,0skips): source rows including two34776 spells, stale-row replacement, unrelated-entry preservation, repeat, missing-source-table rollback, existing maps and open-world rejection. No canonical write. The diagnostic TDB-only runtime-fixture catalog must retire once publication is proved.

Process tests require `WORLD_POPULATION_SOURCE=/absolute/path/to/authentic/world.db`; without it they skip. Run `env WORLD_POPULATION_SOURCE=/absolute/path/to/authentic/world.db python3 -m unittest scripts.tests.test_battleground_population`. Lead-only apply: `python3 scripts/promote_battlegrounds.py --db data/world.db --maps 628 --report data/diagnostics/isle-spellclick-publication.json`.

### Skyriding native prerequisite fixture — 2026-10-18

Surge Forward372608, Skyward Ascent372610, Aerial Halt403092 and Whirling Surge361584 require native372773 Dragonrider Energy (`SpellAuraRestrictions.CasterAuraSpell`). Mount capability aura406095 supplies it through effect1 LINKED285; the existing [linked-aura handler](auras.md) casts that child. The skyrider fixture loaded406095 but omitted372773 from its explicit SpellStore. Strict required-aura admission in9129a861b exposed that missing fixture input when stack9f19bd86e landed through mergeae9a7a53c: its exact parent8707ac0ea passes21/21 skyriding cases, while the merge fails10/21 with `SPELL_FAILED_CASTER_AURASTATE`. Dormant-aura candidatefc0372d93 independently passes21/21. All runs use identical owned world/protocol inputs; the captured-destination fix is later and not causal.

The fixture now loads the real child and asserts normal mount completion produced its active aura. It never inserts that aura or relaxes production admission. Existing charge, GCD, cooldown, takeoff, tutorial-removal and movement assertions are unchanged. At1342a9fb0 all21 skyriding cases plus unchanged full Akunda pass22/22,0failed,0ignored in24.84s/176192ms; server/all-target check passes12.123s without warnings, and fmt passes. Only native-child loading and its active-aura assertion changed; all production files remain untouched. Receipts: `data/class-completeness6-2026-10-12/skyriding-regression-proof-ledger.json`. This is fixture qualification, not a new live/Retail parity claim.

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
- The takeoff (part 4): Retail's takeoff is JUMP, JUMP (Blizzard_Tutorials_RPE.lua:391-395). A skyrider's Space in the air sends Skyward Ascent 372610 instead of launching (game-engine `gameplay.rs` `take_takeoff_request`); the cast spends a Skyriding Charge, its effect 1 FORCE_CAST 140 makes the rider cast Lift Off 404191 (DUMMY aura, 2 s) on itself (TC `Spell::EffectForceCast`, SpellEffects.cpp:791-849; `cast_triggered` ignores the forced cast's costs, which TC keeps; Lift Off has none), and its `SpellGo` launches the client's glider. Without a charge the cast fails and the jump lands. Effects 2/3 (REMOVE_AURA_2 203) remove the tutorial highlight auras 1259215/1259214 from the rider. Surge Forward removes the same two; Whirling Surge removes 1259208. Lift Off 374763 ("When standing, launch upward", no ChargeCategory) also force-casts 404191; its native trigger remains unverified (investigation below).
- Second Wind: `EffectModifySpellCharges` → `restore_charges` (TC `RestoreCharge`: the most recent used charge comes back, the oldest keeps recovering), and a `SpellChargesUpdate` named after the last spell that used the category.
- Not modelled: exact impulse sizes (assumed; Retail runs these DUMMY effects in an unpublished server script; TC defines `SMSG_MOVE_ADD_IMPULSE` but never sends it); Thrill of the Skies 377234 (-42%) and Ground Skimming 404184 (-20%), trait-tree talents 383366/404183 whose triggers are server scripts; Dragon's Rest 372606; Whirling Surge's spiral; Aerial Halt's reduced gravity; the Space launch spending a charge (Retail's takeoff is Skyward Ascent / Lift Off).
- Tests: `skyriding_tests.rs` (charge spent, Whirling Surge, shared charges, GCD, drain then NoChargesRemain, 10.35 s recovery, Aerial Halt cooldown, Second Wind, needs Skyriding Basics), `networking_tests/skyriding.rs` (a granted Surge Forward is followed within 0.05 yd; the same flap ungranted falls behind). Live: game-engine `godot/tests/skyriding_abilities_live.gd`.

### Effect 203: tutorial highlight removal

TC a352b1fa `SharedDefines.h:1549` names 203 `SPELL_EFFECT_REMOVE_AURA_2`; `SpellEffects.cpp:295` dispatches `Spell::EffectRemoveAura`, not `EffectNULL`. The handler at lines 5154–5163 removes all `TriggerSpell` auras from the selected unit, without a caster filter. `spell_cast/effects.rs::remove_aura` uses the normal aura removal lifecycle (AuraRemoved log, scripts, mount reconciliation, stat/replicated aura updates); the registry prevents the unhandled warning.

Real `world.db` effects: Surge Forward 372608 indices 1/2 remove 1259214/1259215; Skyward Ascent 372610 indices 2/3 remove 1259215/1259214; Whirling Surge 361584 index 2 removes 1259208. These are Highlight Surge Forward / Skyward Ascent / Whirling Surge (DNT). The real-data regression applies all three plus an unrelated Lift Off aura to both rider and another unit, casts each ability, and checks only the selected highlights disappear, with no effect-203 warning.

### Lift Off 374763 investigation

Verified 2026-10-05 against DB2 12.1.0.69933 and TC a352b1fa:

- `SpellEffect` 1000784: effect index 0, DUMMY 3, BasePoints 450, implicit targets 1/1 (caster). No LEAP/JUMP/knockback effect.
- `SpellEffect` 1051597: index 1, FORCE_CAST 140, TriggerSpell 404191, target 1 (caster). The existing handler applies 404191; row 1051600 is APPLY_AURA 6 / DUMMY 4, SpellDuration 39 = 2000 ms. A real-data cast test explicitly learns 374763, mounts in Skyriding, and checks that aura is applied and expires.
- No `spell_effect.EffectTriggerSpell=374763`, LEARN_SPELL aura MiscValue 374763, or `tdb_conditions.SourceEntry=374763` row exists in this imported database. No 374763 rule occurs in the inspected TC Spell.cpp, spell_generic.cpp, Player.cpp or MovementHandler.cpp. No imported spell data specifies a skyriding-only failure reason. `request_cast` requires a known spell; no default grant for 374763 is implemented.
- Retail UI `Blizzard_Tutorials_RPE.lua:391–395` teaches double JUMP for takeoff, and line 439 teaches JUMP to fly higher. It does not name 374763. The current Godot path casts 372610 on the second Space, then launches on its SpellGo; 372610 independently FORCE_CASTs 404191. Sharing the resulting aura does not prove 372610 triggers 374763.
- **Blocked:** identifying 374763's native request/trigger and exact unmounted/non-skyriding failure. No automatic grant, guessed mount failure, fabricated LEAP effect, or new impulse formula was added. Client/wire work remains outside this server slot. A proposed direct 374763 client action would need existing SpellCastIntent delivery and launch-on-SpellGo handling, but its activation condition and permission cannot yet be specified as Retail parity.

Sources: [TC effect names](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Miscellaneous/SharedDefines.h#L1549), [TC dispatch](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellEffects.cpp#L295), [TC removal](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Spells/SpellEffects.cpp#L5154); imported `world.db` rows above; local Retail UI and existing server/client source paths above. Trigger/failure absence is bounded to these inspected sources, not proof that Retail has no rule.

#### Follow-up: bar and casting-restriction data

Verified 2026-10-05 against full Wago CSV exports pinned to **12.1.0.69933**, not just the imported `world.db` subset:

| Source | Exact row / observation | What it establishes |
|---|---|---|
| `OverrideSpellData` | ID **2106**: Spells 0/1/2/3/9 = **372608 / 372610 / 361584 / 374990 / 377042**, Flags 5; none of 3050 rows contains 374763 | This Skyriding override contains Skyward Ascent, not Lift Off 374763 |
| `SpellEffect` | ID **1052661**, spell **404849**, aura **260** (`OVERRIDE_SPELLS`), MiscValue 2106, MiscValueB 7; spells 404751/421621/427422/427423 also reference 2106 | An actual aura-to-override-data link, not an assumed slot assignment |
| `SpellShapeshiftForm` | ID **47**, Dragon Isles Drake: BonusActionBar **4**, presets **372608 / 372610 / 361584 / 403092**; no form preset contains 374763 | Another Skyriding bar source, again without 374763; not evidence for replacing the current generic bonus bar |
| `MountCapability` → `FlightCapability` | Capability **494**: ModSpellAuraID **406095**, FlightCapabilityID **11**, ReqSpellKnownID **376777**, PlayerConditionID **96927**. FlightCapability **11**: SpellID **372610** | The movement capability names Skyward Ascent as its native spell; it does not name 374763 |
| `SpellActionBarPref` | ID **343**, SpellID **372610**, PreferredActionBarMask **1024**; no 374763 row | Bar preference for 372610, not 374763 |
| `SpellAuraRestrictions` / `SpellCastingRequirements` | No SpellID **374763** row in either full export (10632 / 28152 rows) | No mounted/grounded prerequisite or failure reason can be read from these tables for 374763 |

`SpellMisc` **633542** for 374763 has Attributes_0 **0x2d800000**, including **ALLOW_WHILE_MOUNTED** (permission while mounted, not a requirement to be mounted). Attributes_2 **0x30484005** includes **NOT_AN_ACTION** (0x10000000); Attributes_3 **0x10130200** includes **IGNORE_CASTER_AND_TARGET_RESTRICTIONS** (0x10000000). Names are from TC **a352b1fa** `SharedDefines.h`. These flags do not establish a player action-bar button, a ground check, or an unmounted failure reason. They reinforce the need to identify the native/scripted activation before granting the spell or adding checks.

Retail UI `AddOns/Blizzard_Tutorials/Blizzard_Tutorials_RPE.lua:391–395` binds takeoff instructions to **JUMP, JUMP**; `:439` binds Fly Higher to **JUMP**; `:446–452` resolves Surge Forward through the bonus-bar action button. `AddOns/Blizzard_ActionBarController/ActionBarController.lua:154–163` selects vehicle, override, temporary shapeshift or bonus pages through `C_ActionBar`; `AddOns/Blizzard_OverrideActionBar/OverrideActionBar.lua:151` uses `GetOverrideBarIndex`. No inspected Lua source names 374763. The UI source and FlightCapability agree on double-jump/Skyward Ascent; neither proves 374763's trigger.

Existing client transport already carries arbitrary spell IDs: `godot/rust/src/account.rs:465` accepts `SpellCastIntent`; `godot/rust/src/ipc/world.rs:220` sends the requested spell ID; `godot/rust/src/spells.rs:418` casts an action slot's `ActionRef::Spell`. **No wire addition is needed.** The missing piece is not serialization: it is the verified action/grant and rejection semantics. No production code or behavior test was changed in this follow-up; adding a no-charge action-bar button or guessing `OnlyMounted` would not be established Retail parity.

Pinned export sources: [OverrideSpellData](https://wago.tools/db2/OverrideSpellData/csv?build=12.1.0.69933), [FlightCapability](https://wago.tools/db2/FlightCapability/csv?build=12.1.0.69933), [SpellActionBarPref](https://wago.tools/db2/SpellActionBarPref/csv?build=12.1.0.69933), [SpellAuraRestrictions](https://wago.tools/db2/SpellAuraRestrictions/csv?build=12.1.0.69933), [SpellCastingRequirements](https://wago.tools/db2/SpellCastingRequirements/csv?build=12.1.0.69933); existing local `data/db2/12.1.0.69933/{MountCapability,SpellShapeshiftForm}.csv`, `world.db`, and Retail UI paths above; [TC attribute definitions](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Miscellaneous/SharedDefines.h).

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

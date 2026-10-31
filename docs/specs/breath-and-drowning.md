# Breath and drowning

Breath and fatigue mirror timers and their environmental damage, TrinityCore master rules (`Player::HandleDrowning`, Player.cpp:725-853; `getMaxTimer` 680-704; `UpdateLiquidMirrorTimerFlagsOnPositionChange` 26779-26823; `GridMap::GetLiquidStatus`, GridMap.cpp:661-690). Server: `crates/server/src/mirror_timers.rs`, `crates/server/src/liquid.rs`. Protocol: shared-protocol `protocol/mirror_timer_messages.rs`. How it works: [swimming and breath](../wiki/systems/swimming-and-breath.md).

## What it must do

- [x] Owner-only `MirrorTimerStart { timer, value_ms, max_value_ms, scale, paused, spell_id }`, `MirrorTimerPause { timer, paused }`, `MirrorTimerStop { timer }` on the reliable ordered `MirrorTimerChannel` (`SMSG_START/PAUSE/STOP_MIRROR_TIMER`); `timer` is `MirrorTimerType`: fatigue 0, breath 1, feign death (`FIRE_TIMER`) 2; `scale` is timer milliseconds per elapsed millisecond.
- [x] The head is under water when the terrain liquid surface is more than `DEFAULT_COLLISION_HEIGHT` (2.03128, Object.h:78) over the feet (`LIQUID_MAP_UNDER_WATER`); a Godot swimmer floating at the surface (feet 1.25 under it) breathes.
- [x] Head under: the breath timer starts full, 180 000 ms (`3 * MINUTE`) × `MOD_WATER_BREATHING` 155, sent once with scale -1, and drains by the tick time without resending.
- [x] Head out: the timer refills 10 × the tick time, sent once with scale 10, and stops (`MirrorTimerStop`) when full. Going back under while refilling resends it draining from its value.
- [x] Below 0 the timer gains 1 000 ms back and deals `DAMAGE_DROWNING`: 20% of maximum health plus `urand(0, 10^max(0, floor(log10(damage)) - 1))`, every second while out of breath; a lethal hit is a player death.
- [x] `WATER_BREATHING` 82 (Unending Breath 5697, Water Breathing 7178) disables the breath timer; gaining it under water stops a running one (`UpdateMirrorTimers`).
- [x] Deep water (the chunk's MH2O `deep` mask) with any liquid status runs the fatigue timer the same way: 60 000 ms, `DAMAGE_EXHAUSTED` 20% every second at 0, refill ×10 out of it.
- [x] Death (health 0 or not `Alive`) stops every running timer, as `KillPlayer`/`BuildPlayerRepop` call `StopMirrorTimers`; a dead player gets no timers. A player the server flies (taxi) is out of the water.
- [x] The server sends no `MirrorTimerPause` and no feign-death timer: TrinityCore sends neither.
- [ ] The timers count server simulation time (the 50 ms fixed step), TrinityCore counts real elapsed time (`Player::Update(p_time)`). When outer updates overrun Bevy's 250 ms virtual-time clamp ([tick loop design](../wiki/design/tick-loop-design.md#catch-up-and-overrun-policy)) the server falls behind the wall clock and so behind the client's bar, which counts real time: on :5000 on 2026-09-29 00:33:38-00:39:04 the server ran 4 000 ticks (200 s) in 326 s, so breath had not run out when the client's bar reached 0.
- [ ] Fatigue in TrinityCore needs the liquid's `LiquidType.SoundBank` to be ocean (map_extractor System.cpp:690); the server has no LiquidType data, so any liquid with the `deep` mask fatigues.
- [ ] Lava and slime damage (`FIRE_TIMER`, `DAMAGE_LAVA`) need `LiquidType.SoundBank`; not implemented. WMO liquid has no MH2O deep mask.
- [ ] Collision height is TrinityCore's default for every race; `CreatureModelData.CollisionHeight` × scale of the player's display is not used.
- [ ] A ghost's fatigue does not repop it at its graveyard; GM breathing disable, `SPELL_AURA_FORCE_BREATH_BAR`, `MOD_ENVIRONMENTAL_DAMAGE_TAKEN` and `SMSG_ENVIRONMENTAL_DAMAGE_LOG` are not implemented.

## Tests

- `crates/server/src/mirror_timers_tests.rs`: start full and drain silently, refill ×10 and stop at 3 s, re-dive resend, drowning every second from 180 s, damage formula, water breathing, breath multiplier, fatigue after 60 s, death stops; on real Azeroth 32,48 liquid: head-under threshold at the swim spot, a seabed player through the fixed-tick system and Lightyear host delivery gets the breath start and one drowning hit at 180 s and a refill on surfacing, death stops the bar, a floating swimmer gets none.
- `crates/server/src/auth_character_tests.rs` `a_selected_player_on_the_lake_bed_drowns_every_second_after_180_s`: a player entered through character select, real `GroundMap` and `update_mirror_timers`, hits of 58-59 at ticks 3601/3621/3641/3661.
- `crates/server/src/auras/tests.rs` `water_breathing_aura_marks_the_target_water_breathing` (world.db spell 5697).
- shared-protocol `protocol_mirror_timer_tests.rs`: wire round trips, type values, registration.

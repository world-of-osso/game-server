# Damage meter

Server side of Retail's built-in damage meter (`Blizzard_DamageMeter`, `C_DamageMeter`, 12.x). Server: `crates/server/src/damage_meter.rs`. Protocol: shared-protocol `protocol/damage_meter_messages.rs`. Client window: game-engine `docs/specs/damage-meter.md`.

Retail's UI reads only aggregated sessions (`C_DamageMeter.GetCombatSessionFromType`/`FromID`, DamageMeterSessionWindow.lua:561-579) and the `DAMAGE_METER_*` events (DamageMeterSessionWindow.lua:93-97); it never reads combat log events, and every session getter is `SecretWhenInCombat` (DamageMeterDocumentation.lua:23,41,59,77). The aggregation is therefore not client Lua. Whether Retail computes it on the server or in the client engine is not visible in these files; here the server computes it, because a client does not receive its group members' spell damage (`log_event` sends combat log lines only to the source's and target's owners, spell_cast/mod.rs `log_event`).

## What it must do

- [x] Every `CombatLogKind::Damage` line with a positive amount by a player to another unit (`spell_cast::log_event`: spells, periodic ticks, melee swings, `.damage`) counts on the meter of that player and of every online member of its group (`GroupRegistry::group_members`). Healing, misses, self-damage and creature damage do not count.
- [x] Damage is kept per source (unit, name, class) and per spell id (0 = melee).
- [x] A meter's `Current` session starts when its owner or any online group member is in combat (`CombatStatus`), or at the first counted damage, and ends when none of them is; each start is a new session, numbered from 1 (`DamageMeterAvailableCombatSession.sessionID`). After it ends it stays `Current` until the next begins.
- [x] `Overall` sums every session since the owner entered the world; its duration is the sum of the sessions' combat time, including the running one.
- [x] A source's and a spell's `amount_per_second` is its damage over its session's duration (`GetSessionDurationSeconds`), 0 at duration 0.
- [x] Sources rank by damage (ties by name), spells by damage; `total_amount` is the session sum; `is_local_player` marks the recipient.
- [x] The owner gets `DamageMeterSnapshot { current, overall }` on `CombatChannel` when a session starts or ends, and at most every 1 s while one runs or has new damage; nothing while idle.
- [ ] Pets and guardians (`RedirectSourceToOwner`), healing, absorbs, interrupts, dispels, damage taken, deaths and the other `DamageMeterType`s are not tracked.
- [ ] Only the current and overall sessions are kept; Retail lists past sessions (`GetAvailableCombatSessions`) and can reset them (`ResetAllCombatSessions`).
- [ ] Combat ends 5 s after the last hit (the server's `COMBAT_GRACE_SECS`), so a session's duration includes that grace and its DPS is lower than over the active fighting time.
- [ ] Retail's session boundaries for a group (who must be in combat, whether a far-away member's combat counts) are not documented in the files; here any online member in combat keeps the session running.

## Tests

- `crates/server/src/damage_meter_tests.rs`: session duration and DPS over combat time with per-spell totals; a new session per combat and overall summing damage and durations (also while running); damage out of combat opens a session and sources rank by damage; the real Frostbolt 116 path (level 10 mage harness, world.db): its 228597 damage line equals current and overall, the session ends with combat (5-6.5 s), nothing is sent while idle, a second Frostbolt is session 2 and overall is both; a group member's damage and combat reach every member's meter but not an outsider's, heals and self-damage do not count, and the mage's session ends with the group's combat.
- shared-protocol `protocol_damage_meter_tests.rs`: wire round trip, registration.

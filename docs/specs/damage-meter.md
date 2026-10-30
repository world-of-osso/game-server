# Damage meter

Server side of Retail's built-in damage meter (`Blizzard_DamageMeter`, `C_DamageMeter`, 12.x). Server: `crates/server/src/damage_meter.rs`. Protocol: shared-protocol `protocol/damage_meter_messages.rs`. Client window: game-engine `docs/specs/damage-meter.md`.

Retail's UI reads only aggregated sessions (`C_DamageMeter.GetCombatSessionFromType`/`FromID`, DamageMeterSessionWindow.lua:561-579) and the `DAMAGE_METER_*` events (DamageMeterSessionWindow.lua:93-97); it never reads combat log events, and every session getter is `SecretWhenInCombat` (DamageMeterDocumentation.lua:23,41,59,77). The aggregation is therefore not client Lua. Whether Retail computes it on the server or in the client engine is not visible in these files; here the server computes it, because a client does not receive its group members' spell damage (`log_event` sends combat log lines only to the source's and target's owners, spell_cast/mod.rs `log_event`).

## What it must do

- [x] Every `CombatLogKind::Damage` line with a positive amount by a player to another unit (`spell_cast::log_event`: spells, periodic ticks, melee swings, `.damage`) counts on the meter of that player and of every online member of its group (`GroupRegistry::group_members`). Misses, self-damage and creature damage do not count as damage done; healing is a separate category.
- [x] Damage is kept per source (unit, name, class) and per spell id (0 = melee).
- [x] A meter's `Current` session starts when its owner or any online group member is in combat (`CombatStatus`), or at the first counted category event, and ends when none of them is; each start is a new session, numbered from 1 (`DamageMeterAvailableCombatSession.sessionID`). After it ends it stays `Current` until the next begins.
- [x] Combat follows TrinityCore's `CombatManager` (`combat_refs.rs`, game-server wiki `systems/combat.md`): a fight with a creature that has a threat list lasts until it dies or evades, so a lull does not split the session and a kill ends it at once; a fight with a training dummy ends 5 s after the attacker's last hit (TC `npc_training_dummy`), so a gap over 5 s there starts a new session; auto-attack alone keeps nobody in combat.
- Retail's DPS denominator is not documented. `GetSessionDurationSeconds` "Returns the amount of time a combat session has lasted" (DamageMeterDocumentation.lua), the session timer runs only while `UnitAffectingCombat("player")` (DamageMeterSessionWindow.lua:915-926), and sessions are named `DAMAGE_METER_COMBAT_NUMBER` (DamageMeterSessionWindow.lua:421). Nothing names an activity time or first-to-last-hit span, so the duration here is combat time.
- [x] `Overall` sums every session since the owner entered the world; its duration is the sum of the sessions' combat time, including the running one.
- [x] A source's and a spell's `amount_per_second` is its damage over its session's duration (`GetSessionDurationSeconds`), 0 at duration 0.
- [x] Sources rank by damage (ties by name), spells by damage; `total_amount` is the session sum; `is_local_player` marks the recipient.
- [x] The owner gets `DamageMeterSnapshot { current, overall }` on `CombatChannel` when a session starts or ends, and at most every 1 s while one runs or has new damage; nothing while idle.
- Hunter pet actions redirect to their owner. Guardians, damage taken totals and other `DamageMeterType`s remain outside this change.
- [ ] `FOLLOWER_DAMAGE_METER_EXCLUDED`: follower NPC damage is omitted because sources must be players. Retail includes follower NPCs in the group meter.
- [ ] Only the current and overall sessions are kept; Retail lists past sessions (`GetAvailableCombatSessions`) and can reset them (`ResetAllCombatSessions`).
- [ ] A session against a training dummy includes the 5 s after the last hit before the dummy's combat ends (as in TC). Combat starts when a harmful spell launches (Spell.cpp:8538-8541), and a missile lands after its flight, so a one-shot kill lasts the flight (0.57 s for Frostbolt at 20 yd).
- [ ] Retail's session boundaries for a group (who must be in combat, whether a far-away member's combat counts) are not documented in the files; here any online member in combat keeps the session running.

## Group category snapshots

- [x] Every online party/raid member receives every member's damage, effective healing, overhealing, consumed absorbs, successful interrupts, dispels and deaths in Current and Overall. Zero-valued members remain present. An unrelated idle player receives nothing.
- [x] Healing is `max(amount, 0) - clamp(overflow, 0, amount)`; overhealing is the clamped overflow. Absorbs count shield points consumed, credited to the shield caster, not the victim's combat-log `absorbed` field.
- [x] Each death freezes the last eight events targeting that member, chronological and excluding the Death line itself. Only the latest two death recaps per member per session are exported; death totals retain all deaths. Logged deaths and their accepted player-death transitions count once.
- [x] Each source exports successful interrupt and dispel counts by `(spell_id, affected_spell_id)`: the casting spell and interrupted spell/removed aura from `CombatLogEvent.extra_spell_id`. Missing affected identity stays absent. Current and Overall independently retain the first 16 distinct pairs per action category, increment retained pairs after reaching the cap, and sort exported detail by descending count then spell IDs. Category totals include events whose pairs were omitted. Both categories reach every online group recipient; no owner-only combat-log counting.
- [x] Exports contain at most 40 sources per session, 64 damage spells and 16 pairs per action category per source, and 48 UTF-8 name bytes. Current plus Overall with two eight-event recaps per source fits 256 KiB in standard bincode, before transport framing. Category totals are never truncated when spell/recap detail is capped.
- Snapshot delivery retains the existing one-second cadence and immediate session-boundary sends. Solo damage sessions, DPS and owner combat-log delivery remain unchanged; the added categories are authoritative for solo players too.

### Retail reference

Paths relative to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- `Blizzard_DamageMeter/DamageMeterSessionWindow.lua:1-4`: damage, healing/HPS, interrupts, dispels and deaths category menu.
- `Blizzard_DamageMeter/DamageMeterSessionWindow.lua:15-19,37-44`: healing, absorbs, interrupts, dispels and deaths types.
- `Blizzard_DamageMeter/DamageMeterSessionWindow.lua:561-579`: aggregated `C_DamageMeter` session getters; `:967-970`: death recap action.
- `Blizzard_DamageMeter/DamageMeterSourceWindow.lua:156-191`: category-specific source getter and `combatSpells` detail rows; `Blizzard_DamageMeter/DamageMeterEntry.lua:617-628,706-727`: spell identity, names and target-unit formatting.
- `Blizzard_APIDocumentationGenerated/DamageMeterDocumentation.lua:183-191,214-243`: `combatSpells`, spell ID, total and target details. The public API does not expose a separate interrupted-spell field or define how two spell identities relate; the requested cast/affected pair here preserves the existing authoritative combat-log identities rather than inventing API behavior.
- `Blizzard_APIDocumentationGenerated/DamageMeterConstantsDocumentation.lua:68-71,83-90,102-111`: SpellCasted/SpellAffected display enum, storage and display categories. Overheal is a server detail, not a separate Retail menu category.

### Client migration (client remains unchanged here)

Read `DamageMeterSource.interrupt_spells` and `dispel_spells` for each member's action breakdown, resolving casting and affected spell names locally. Read `DamageMeterSource.healing_done`, `overhealing`, `absorbs`, `interrupts`, `dispels`, `deaths` and `death_recaps` instead of incrementing these categories from owner-scoped `CombatLogEvent`. Recap entries expose timestamp, source/target, spell, amount, overflow and mitigation fields. Continue using `total_amount`, `amount_per_second` and `spells` for damage. A healing-and-absorbs view sums `healing_done + absorbs`; HPS divides by session duration. Do not add owner combat-log counts on top of snapshot totals. Deploy matching protocol consumers together: the extended serialized source is not wire-compatible with old builds.

## Tests

Targeted action-breakdown proof, 2026-10-06: server `4788713` (implementation `7f22fee`) passed all 20 `meter`-filtered tests; protocol `8147de9` passed all four `damage_meter`-filtered tests, including round trip and the maximum snapshot. Maximum encoded payload: 252,761 bytes, below 262,144 bytes (256 KiB), before framing. Both runs exited 0 without warnings through `build-lock.sh`, `agent-run meterspells`, and the local server helper; protocol tests used their own `../shared-protocol/Cargo.toml` manifest. Logs: `/tmp/claude/meterspells-resume-server.out` and `/tmp/claude/meterspells-resume-protocol.out`. No live server was started.

Targeted local proof at server `24822cc` / protocol `a4bee6a`: all 19 server `damage_meter` tests pass without warnings. Protocol's four `damage_meter`-filtered tests passed at `a6e871f`; `a4bee6a` changes only a module comment. Both runs used the build lock, `agent-run groupmeter` and the local Docker helper; no live server was started.

New behavioral assertions (`crates/server/src/damage_meter_tests.rs`):

- `groupmeter_three_players_receive_exact_categories_and_capped_recaps`: three authenticated HostClient connections receive exact healer/tank/DPS Current and Overall totals; outsider receives nothing; three deaths retain the latest two eight-event recaps.
- `groupmeter_healing_is_visible_to_a_non_casting_group_member`, `groupmeter_raid_receives_healing_without_counting_creature_damage`: party/raid healing and zero damage from creature hits.
- `groupmeter_first_absorb_exports_the_whole_roster`, `groupmeter_real_shield_consumption_credits_the_caster_group`: absorb-only roster and real Power Word: Shield consumption; mitigation is not counted twice. First-absorb RED exported one source instead of two before roster seeding was fixed.
- `groupmeter_categories_reset_current_but_preserve_overall`, `groupmeter_unlogged_death_transition_counts_once`: segment category reset and missing/logged Death transition accounting.
- `damage_meter_action_spell_caps_preserve_totals_and_existing_pairs`: both action categories retain 16 distinct cast/affected pairs, increment retained pairs after the cap, preserve full counts for omitted pairs, clear Current detail at the next session, and preserve Overall detail.
- `groupmeter_export_caps_detail_without_losing_totals`: spell-detail and UTF-8 name caps retain full damage totals. Protocol `damage_meter_snapshot_round_trips_on_the_wire` includes every new field; `damage_meter_maximum_snapshot_fits_payload_bound` round-trips a maximum-width 40-source, two-session fixture below the advertised byte bound. Registration and existing threat round-trip also pass.


- `crates/server/src/damage_meter_tests.rs`: session duration and DPS over combat time with per-spell totals; a new session per combat and overall summing damage and durations (also while running); damage out of combat opens a session and sources rank by damage; the real Frostbolt 116 path (level 10 mage harness, world.db): its 228597 damage line equals current and overall, the session ends with combat (5-6.5 s), nothing is sent while idle, a second Frostbolt is session 2 and overall is both; a group member's damage and combat reach every member's meter but not an outsider's, heals and self-damage do not count as damage done, and the mage's session ends with the group's combat; a group member's fight with a creature keeps one session for both through an 8 s lull and ends it when the creature's combat stops; auto-attack from out of range does not stretch a dummy session past 5-6.5 s; a kill ends the session at the kill (3.2-3.4 s from the first launch for two Frostbolts 3.15 s apart); a one-shot Frostbolt kill at 20 yd lasts its 0.5-0.65 s flight; an 11 s gap between Frostbolts on a creature is one session; on a dummy, hits 3.15 s and 3.9 s apart are one session and a 6.15 s gap starts session 2.
- `crates/server/src/combat_refs_tests.rs`: combat references (see game-server wiki `systems/combat.md`).
- shared-protocol `protocol_damage_meter_tests.rs`: wire round trip, registration.

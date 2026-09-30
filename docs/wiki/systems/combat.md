# Combat

Combat logic is split across two crates: `shared/src/formulas/` holds pure combat math (melee hit table, stat conversions, spell formulas), and `server/src/combat.rs` runs the live ECS systems (active auto-attack, direct NPC/player lethal-transition emission, respawn).

## Content

## Server Systems (`CombatPlugin`)

Runs in `FixedUpdate`. Systems in order:

1. `creature_attack_system` and `process_attack_messages` — start and stop auto-attack (see [Starting and stopping auto-attack](#starting-and-stopping-auto-attack))
2. `auto_attack_system` — processes only entities with `AutoAttack`, swings at `AutoAttack.victim`, ticks the main- and off-hand swing timers, checks melee range (5 yards), and swings: players their weapon through the hit table and armor ([[player-damage]]), creatures `BASE_DAMAGE` or their ExpectedStat damage through armor; then per-target damage and health multipliers ([[level-scaling]]). It emits damage or miss/dodge/parry events and handles lethal NPC transitions
3. `maintain_in_combat_state_system` — expires `CombatState` after 5s of no engagement
4. `respawn_system` — ticks `DeathState.respawn_timer` (default 60s), restores HP and position to `SpawnPoint`, re-adds `Replicate` and `NetworkVisibility`

All combat events are broadcast over `CombatChannel` (reliable ordered) and logged to a server-side `CombatLog` ring buffer (capacity 10,000 entries). Melee damage, NPC death, and NPC respawn events retain this broadcast and logging path.

## Starting and stopping auto-attack

Selecting a unit never attacks it. TrinityCore `HandleSetSelectionOpcode` (MiscHandler.cpp:396-399) only sets the selection, and `SetTarget` here only writes `CombatTarget`. `AutoAttack { victim }` is TrinityCore's `m_attacking` with `UNIT_STATE_MELEE_ATTACKING`. A player's victim is separate from its selection: `Player::SetTarget` is a no-op (Player.h:1920). Component presence means the unit is attacking.

- **Players** start with `AttackSwing { target }` (`CMSG_ATTACK_SWING`, `HandleAttackSwingOpcode`, CombatHandler.cpp:28-61).
  - An unknown unit, or one that fails `IsValidAttackTarget`, gets `AttackStopped` back. That covers dead units (Object.cpp:2367), uninteractible units (Object.cpp:2374), and faction or duel refusals ([[factions]]).
  - A dead attacker's request fails silently (`Unit::Attack`, Unit.cpp:5876).
  - A new victim keeps the running swing timers. Repeating the current victim changes nothing (Unit.cpp:5906-5931).
  - `AttackStop` (`CMSG_ATTACK_STOP`) runs `Unit::AttackStop`.
- **Creatures** attack through their AI's `CombatTarget` (`Changed<CombatTarget>`, `Without<Player>`). `Unit::Attack` engages the victim for a creature (Unit.cpp:5950-5952), so they enter combat at once.
- **Combat.** A player enters combat with the first swing (`AttackerStateUpdate` → `AtTargetAttacked`, Unit.cpp:2271), not with the request. Auto-attack alone keeps nobody in combat; see Combat State.
- **Stops.** `combat::attack_stop` removes `AutoAttack` and sends `AttackStopped { attacker, victim, now_dead }` (`SendMeleeAttackStop`, Unit.cpp:2550-2556). Every stop goes through it:
  - `CMSG_ATTACK_STOP`
  - `combat_stop` (the unit's and its attackers', `CombatStop` → `RemoveAllAttackers`, Unit.cpp:6029-6044), for death (`setDeathState`, Unit.cpp:9185-9187), evade, map transfer and duel end
  - scripted AI `AttackStop`
  - a completed cast of a `SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT` spell (Polymorph, Gouge; `Spell::finish`, Spell.cpp:4448-4450). Channels stop at cast start, not at channel end.
- **Echoes.** `AttackStart { attacker, victim }` (`SMSG_ATTACK_START`, `SendMeleeAttackStart`) and `AttackStopped` go to every client replicating the attacker (`spell_cast::send_to_set`, TrinityCore `SendMessageToSet`).
- **Selection changes.** The server does not stop auto-attack when the selection changes. The client sends `AttackSwing` for a new attackable selection and `AttackStop` otherwise.

Tests: `combat_tests/attack_requests.rs` (selection neither swings nor enters combat; swing, echo, first-swing combat; stop; switch keeps the timer; friendly, corpse, uninteractible and unknown refusals; victim death; Polymorph).

## Combat State

`combat_refs.rs` is TrinityCore's `CombatManager` (Combat/CombatManager.h:27-46): each pair of units fighting holds a reference, and `CombatStatus` is whether a unit holds any. Swings, spell damage, `.damage`, threat (`add_threat`) and a creature's attack call `set_in_combat_with(a, b)`; `combat_stop` (death, evade, map transfer, duel end) ends every reference of the unit on both sides, so the units it fought leave combat at once unless they fight something else.

- A reference with a creature that has a threat list never times out: the fight lasts until the creature dies or evades, however long the lull.
- Player-versus-player references end 5 s after their last refresh (`PVP_COMBAT_TIMEOUT`, CombatManager.h:84).
- Creatures without a threat list (triggers, critters, training dummies) have no AI that would end their fights, so their references time out after 5 s too, like TC `npc_training_dummy` (npcs_special.cpp:1382-1415).
- Not modelled: suppressed references (vanish, feign death; TC puts a caster in combat at spell launch, Spell.cpp:8538-8541, here at the hit), pets.

Tests: `combat_refs_tests.rs` (lull, two opponents, PvP timeout, threat-less timeout, despawn, dead refused).

## Replicated Unit Target

`UnitTargetPlugin` (`unit_target.rs`, PostUpdate) mirrors each unit's current target into the replicated `UnitTarget(Option<u64>)` (entity bits) for target-of-target frames. Players: from `CombatTarget` (SetTarget). NPCs: from `CreatureState::Pursuing { target }`, which [[creature-aggro]] keeps equal to the creature's own `CombatTarget`. Written only when the value changes.

## NPC Death Transition (`c8762ed`)

NPC death is handled at the sole authoritative NPC damage boundary: `try_swing`, called by `auto_attack_system`. After damage is clamped to zero, a lethal NPC result immediately inserts `DeathState` and removes `Replicate`; the same swing emits both the melee-damage and death `CombatEvent` values. A lethal swing on an NPC or a player also sends the `CombatLogEvent` Death line (after the swing's Damage line) to the killer and victim, built by the same `death_log_event` spell kills use (AzerothCore `Unit::DealDamage` → `Unit::Kill` for both). The former 20 Hz scan over every living NPC was removed.

The transition contract is explicit:

- Nonlethal damage lowers health while leaving the NPC alive and replicated.
- Lethal damage marks the NPC dead, removes replication, and emits a death event.
- An NPC that starts at zero health without an authoritative damage transition is not discovered by a global poll.
- Future NPC damage writers must route through the same authoritative death transition rather than adding another health scan.

Coverage in `combat_tests.rs` includes `nonlethal_auto_attack_keeps_npc_alive_and_replicated`, `lethal_auto_attack_produces_death_state_replication_removal_and_event`, and `zero_health_without_damage_transition_is_not_discovered_by_death_poll`. Respawn semantics remain unchanged: `respawn_system` restores health and spawn position, re-adds replication and visibility, removes `LastDamager`, and emits the existing `Respawn` event.

## Player Death Transition (`18c936b`)

Player-targeted melee damage emits the internal `PlayerDeathTransition` only when `Health` crosses from positive to zero. The lifecycle consumer in `death/transitions.rs` runs after combat and gravity in `FixedUpdate`, preserving zero-health clamping, `LastDamager`, durability loss, corpse/graveyard state, replication dirtying, the routed `"you died"` update, duplicate-death prevention, resurrection, and later second death. The former broad `detect_player_deaths` player scan is gone.

Persisted zero health is queued during character selection so relogging a dead character reconstructs the same lifecycle instead of waiting for a poll. `b4e5cbd` split the consumer module and `2106cd8` split the focused proof into `combat_tests/death.rs`; `1b7c5a4` preserves unchanged gravity health ticks. Stale entities are ignored safely. RED/GREEN artifacts are recorded under `/tmp/claude/game-server/` for idle-scan removal, transition consumption, combat, fall, login, and second-life coverage. No live CPU savings claim is made.

## Melee Hit Table (`shared/src/formulas/melee.rs`)

The server does not use this AzerothCore table: player swings roll TrinityCore master's in `crates/server/src/melee.rs` ([[player-damage]]).

Single-roll resolution matching AzerothCore's `RollMeleeOutcomeAgainst()`. Priority order on a 0–10000 scale:

**Miss → Dodge → Parry → Glancing → Block → Crit → Hit**

Key formulas:
- **Miss**: 3% base + 1% per level the target exceeds the attacker; clamped to 0 minimum
- **Glancing**: only when target is higher level; `(10 + level_diff * 5) * 100`, capped at 40%
- **Glancing damage**: 90% / 80% / 70% at +1 / +2 / +3 levels (caps at 3)
- **Dodge/Parry**: avoidance DR formula `base + (diminishing * cap) / (diminishing + cap * k)` with per-class `k` and `cap` constants from AzerothCore
- **Crit**: rating → percent via level-scaled curve, then Shadowlands secondary stat DR applied
- **Auto-attack damage**: `weapon_roll + (AP / 6) * weapon_speed`; off-hand gets 50% penalty

## Spell Formulas (`shared/src/formulas/spell.rs`)

- **Spell miss**: same base rate as melee (3% at equal level, +1% per level diff), evaluated independently
- **Spell crit multiplier**: 1.5× base (vs. melee 2.0×); additive aura bonuses stack on top
- **Healing**: `apply_heal` returns effective healing + overheal; health capped at max
- **Haste**: `base_cast / (1 + haste_pct)` for cast times, swing timers, and GCD; GCD floor is 0.75s
- **DoT haste**: Pandaria+ model — hasted tick interval, extra ticks added when enough haste accumulates
- **GCD**: 1.5s base for most classes; 1.0s for Rogue (4), Monk (10), Demon Hunter (12)

## Stat Formulas (`shared/src/formulas/mod.rs`)

Players no longer use the HP/mana/rating functions below: [[character-stats]] computes retail stats server-side.


- **Max HP**: `base_hp(class, level)` from AzerothCore table + `hp_from_stamina`
  - Below 70: WotLK model (first 20 stam = 1 HP, rest = 10 HP each)
  - Level 70+: retail model (flat 20 HP per stamina)
- **Max mana**: same dual-model approach with intellect
- **Armor mitigation**: `armor / (armor + K(attacker_level))`; `K` from SimC `armor_constant` table
- **Secondary stat DR**: piecewise-linear curve (SimC DBC curve 21024); affects Crit, Haste, Mastery, Versatility

## Sources

- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — CombatPlugin, auto-attack, lethal-transition emission, NPC death transition, and respawn
- [crates/server/src/combat_tests.rs](../../../crates/server/src/combat_tests.rs) — auto-attack participation, NPC death, and respawn coverage
- [crates/server/src/combat_tests/death.rs](../../../crates/server/src/combat_tests/death.rs) — focused player combat-death proof
- [crates/server/src/death/transitions.rs](../../../crates/server/src/death/transitions.rs) — player death-transition consumer and lifecycle side effects
- `game-server` commits `18c936b`, `b4e5cbd`, `2106cd8` — player death transition, source split, and focused test split
- `game-server` commit `1b7c5a4` — preserve unchanged gravity health ticks
- `game-server` commit `152ccbd` — remove inactive auto-attack components after target loss
- [crates/shared/src/formulas/melee.rs](../../../crates/shared/src/formulas/melee.rs) — hit table, avoidance DR
- [crates/shared/src/formulas/spell.rs](../../../crates/shared/src/formulas/spell.rs) — spell miss, crit, haste, DoT
- [crates/shared/src/formulas/mod.rs](../../../crates/shared/src/formulas/mod.rs) — HP, mana, armor, rating conversion

## See Also

- [[tick-model]] — combat systems run in `FixedUpdate`; no global fixed-phase pipeline is implemented
- [[networking]] — CombatEvent broadcast over CombatChannel
- [[creature-aggro]] — creature aggro/threat sets the creature's CombatTarget; evade stops combat and restores HP
- [[creature-ai]] — chase and leash movement
- [[player-damage]] — weapon swings, hit table, armor and spell amounts
- [[world-data]] — spell_misc/spell_effect tables provide cast time, range, coefficients

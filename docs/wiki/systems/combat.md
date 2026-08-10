# Combat

Combat logic is split across two crates: `shared/src/formulas/` holds pure combat math (melee hit table, stat conversions, spell formulas), and `server/src/combat.rs` runs the live ECS systems (active auto-attack, direct NPC/player lethal-transition emission, respawn).

## Content

## Server Systems (`CombatPlugin`)

Runs in `FixedUpdate`. Systems in order:

1. `start_attack_system` — reacts to `Changed<CombatTarget>`; inserts `AutoAttack` and `CombatState` for a target, and removes `AutoAttack` when the target is cleared
2. `auto_attack_system` — processes only entities with active `AutoAttack`, ticks swing timers, checks melee range (5 yards), applies `BASE_DAMAGE`, emits damage events, and handles lethal NPC transitions
3. `maintain_in_combat_state_system` — expires `CombatState` after 5s of no engagement
4. `respawn_system` — ticks `DeathState.respawn_timer` (default 60s), restores HP and position to `SpawnPoint`, re-adds `Replicate` and `NetworkVisibility`

All combat events are broadcast over `CombatChannel` (reliable ordered) and logged to a server-side `CombatLog` ring buffer (capacity 10,000 entries). Melee damage, NPC death, and NPC respawn events retain this broadcast and logging path.

## Inactive Auto-Attack Participation (`152ccbd`)

Before `152ccbd`, clearing `CombatTarget` retained `AutoAttack { active: false }`. The fixed-step `auto_attack_system` therefore continued validating and iterating those entities forever, even though `ready_auto_attack_target` returned without changing timers, health, combat events, or combat state. `start_attack_system` now removes `AutoAttack` when the target becomes `None`; component presence represents active auto-attack participation.

Target acquisition still inserts an active `AutoAttack` with a zero timer, so the first swing and existing cooldown cadence are unchanged. Clearing a target still leaves `CombatState` available for the existing combat-grace timeout, and active combat, damage, death, respawn, and reliable event behavior remain unchanged.

The RED test at `/tmp/claude/game-server/autoattack-inactive-removal-red.log` failed because the inactive component remained. The focused GREEN test at `/tmp/claude/game-server/autoattack-inactive-removal-green.log` passed after the removal. `cargo fmt` and Rust readability evidence are recorded for the commit. This documents a correctness-preserving participation fix; no live CPU or Character-stage claim is made before runtime measurement.

## NPC Death Transition (`c8762ed`)

NPC death is handled at the sole authoritative NPC damage boundary: `try_swing`, called by `auto_attack_system`. After damage is clamped to zero, a lethal NPC result immediately inserts `DeathState` and removes `Replicate`; the same swing emits both the melee-damage and death `CombatEvent` values. The former 20 Hz scan over every living NPC was removed.

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
- [[creature-ai]] — aggro/threat feeds CombatTarget; leash evade restores HP
- [[world-data]] — spell_misc/spell_effect tables provide cast time, range, coefficients

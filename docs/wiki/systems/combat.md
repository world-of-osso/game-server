# Combat

Combat logic is split across two crates: `shared/src/formulas/` holds pure combat math (melee hit table, stat conversions, spell formulas), and `server/src/combat.rs` runs the live ECS systems (auto-attack, death, respawn).

## Server Systems (`CombatPlugin`)

Runs in `FixedUpdate`. Systems in order:

1. `start_attack_system` — reacts to `Changed<CombatTarget>`; inserts `AutoAttack` and `CombatState`
2. `auto_attack_system` — ticks swing timers, checks melee range (5 yards), applies `BASE_DAMAGE`, emits `CombatEvent`
3. `maintain_in_combat_state_system` — expires `CombatState` after 5s of no engagement
4. `death_system` — detects HP ≤ 0, inserts `DeathState`, removes `Replicate`
5. `respawn_system` — ticks `DeathState.respawn_timer` (default 60s), restores HP and position to `SpawnPoint`, re-adds `Replicate`

All combat events are broadcast over `CombatChannel` (reliable ordered) and logged to a server-side `CombatLog` ring buffer (capacity 10,000 entries).

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

- [crates/server/src/combat.rs](../../../crates/server/src/combat.rs) — CombatPlugin, auto-attack, death/respawn
- [crates/shared/src/formulas/melee.rs](../../../crates/shared/src/formulas/melee.rs) — hit table, avoidance DR
- [crates/shared/src/formulas/spell.rs](../../../crates/shared/src/formulas/spell.rs) — spell miss, crit, haste, DoT
- [crates/shared/src/formulas/mod.rs](../../../crates/shared/src/formulas/mod.rs) — HP, mana, armor, rating conversion

## See Also

- [[tick-model]] — combat systems run at step 4 of the 20Hz pipeline
- [[networking]] — CombatEvent broadcast over CombatChannel
- [[creature-ai]] — aggro/threat feeds CombatTarget; leash evade restores HP
- [[world-data]] — spell_misc/spell_effect tables provide cast time, range, coefficients

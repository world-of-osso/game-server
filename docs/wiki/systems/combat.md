# Combat

## Native per-hand auto-attack overrides

TC `Unit::AttackerStateUpdate` reads the first active Aura361 TriggerSpell for the main hand and first nonzero MiscValue spell for the off hand. The native engine now reads these fields in aura/effect order at the swing snapshot. After existing weapon/timer/target/range/LOS admission it retains native hand clocks and queues that native spell instead of calculating/committing white damage. A refused native child is logged, not replaced by ordinary damage. No Ret spell IDs or recipient substitution occur in generic execution.

Actual selected404542→408385 native damage and no-override white control pass in the [strict Storm ledger](../../handoffs/ret-storm-gap-proof-ledger.md). Native151791/404542 hand-field/application-order controls and retained core melee coverage are tracked separately. The406833 resource-counter relationship is not supplied by the override itself. [Contract](../../specs/dps-parity.md#ret-storm-root-supply-and-secondary-relationships).

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

## Action-permitting channels —2026-10-24

TC a352b1fa `Unit.cpp3107–3120` does not set `UNIT_STATE_CASTING` for channels with `SPELL_ATTR5_ALLOW_ACTIONS_DURING_CHANNEL` (SharedDefines.h621, bit0x1); `DoMeleeAttackIfReady2166–2180` checks that state, not cast-bar presence. The melee snapshot now reads actual `CastState` and native `SpellStore` permissions. Normal casts and nonallowing channels still block, both hand timers keep advancing, and crowd-control/victim-avoidance paths remain unchanged. Native SCK101546 has0x2201; FoF113656 lacks the allow-actions bit. No core spell IDs or cast/timer/stat changes are introduced.

Real SCK with a naturally ready0ns attack timer stays active1.5s yet produces zero white deliveries before the correction: genuine RED plus three controlsPASS11.67s at1d196cead. Vivify's normal-cast control waits for its genuinely earned20s instant-cast proc to expire; it does not remove that aura or change stats. Atdb543b270 the same four native cases pass27.56s; actual SCK white deliveries become1 while the cast remains active, and all three controls retain their behavior. Compiler405.74s/zero warnings; lead broad/plain-bin gate remains separate. [Contract](../../specs/dps-parity.md), [source/proof ledger](../../handoffs/channel-melee-proof-ledger.md).

## Combat State

`combat_refs.rs` is TrinityCore's `CombatManager` (Combat/CombatManager.h:27-46): each pair of units fighting holds a reference, and `CombatStatus` is whether a unit holds any. Swings, spell damage, `.damage`, threat (`add_threat`) and a creature's attack call `set_in_combat_with(a, b)`; `combat_stop` (death, evade, map transfer, duel end) ends every reference of the unit on both sides, so the units it fought leave combat at once unless they fight something else.

- A reference with a creature that has a threat list never times out: the fight lasts until the creature dies or evades, however long the lull.
- References where both units are player-controlled, including pets, end5s after their last refresh (`PVP_COMBAT_TIMEOUT`, CombatManager.h:84; TC a352b1fa CombatManager.cpp:210–216). An ordinary NPC versus a controlled pet is PvE and does not expire during a quiet interval, despite the pet correctly having no threat list.
- Training dummy PvE references are untimed in the generic manager: the [TC script owns each attacker's five-second timer](#training-dummy-combat-exit--2026-10-23). Existing generic five-second timeouts for other uncontrolled threat-less creatures (triggers/critters) are unchanged; they are not certified by the training-dummy script.
- A harmful spell (or a dispel) with initial aggro (`SpellInfo::HasInitialAggro`: none of `SPELL_ATTR1_NO_THREAT`, `SPELL_ATTR2_NO_INITIAL_THREAT`, `SPELL_ATTR4_NO_HARMFUL_THREAT`), or any harmful spell at an engaged unit, puts the caster in combat when it launches, with the reference suppressed on the target's side (`Spell::PreprocessSpellLaunch` → `SetInCombatWith(target, addSecondUnitSuppressed)`, Spell.cpp:8538-8541, CombatManager.cpp:217); the target enters combat when the spell lands (`PreprocessTarget`, Spell.cpp:2763-2764), which refreshes the reference and lifts the suppression (`CombatReference::Refresh`). A Frostbolt's target is out of combat during the flight; Agony engages both in its launch tick, before its first tick. Polymorph has `NO_THREAT`, so it starts no combat with an idle unit.
- MISS/IMMUNE are still selected `TargetInfo` entries: TC Spell.cpp8538 and2763 exclude only EVADE from caster launch/target arrival combat. `PlannedEffect.selected_units` now preserves selection before defense removes effect recipients; missile timing/reachability and combat use those entries, while damage/aura/hit scripts still use successful `units`. Native two-case missing-reference REDs precede the correction; launch/immune/evade3GREEN plus retained missile/reflection9GREEN and Ret funding/TV damage1GREEN at02090739e. [Proof](../../handoffs/ret-paladin-parity-proof-ledger.md#generic-missed-hostile-launch-combat).
- `Unit::DealDamage` starts no combat itself (Unit.cpp:1085-1092): spell damage's combat comes from the spell's launch, and threat adds it for creatures. So periodic ticks refresh no PvP reference and no training dummy's timer.
- Not modelled: the other suppressions (vanish, feign death).

### Training dummy combat exit — 2026-10-23

TC a352b1fa `npcs_special.cpp1382–1421` starts an attacker's timer on combat entry and refreshes it only for non-DOT `DamageTaken`, even though the dummy takes zero health loss. Each expired timer ends only that attacker's existing PvE reference. `NullCreatureAI::EnterEvadeMode` is empty (`PassiveAI.h55–67`); exit does not call `CombatStop`, remove attack intent, or strip foreign auras. Ordinary LeavingCombat interrupts remain authoritative (`Unit.cpp9257–9272`), as does real CreatureAI evade cleanup (`CreatureAI.cpp298–314`).

Native `creature_scripts.rs` previously used AzerothCore-style whole-dummy evade cleanup, while generic reference refreshes also reset the timer for harmless Counterspell. Independent `db17198c4` RED: actual Fire63 BiS/Fireball133 produces Ignite12654 at1.65s with native9s duration/zero interrupt flags/expiry10.65s; combat ends and the aura vanishes at6.9s. Counterspell also incorrectly extends combat. LeavingCombat26662 and ordinary Defias40/Curse702 controls pass (two RED/two PASS,26.87s). At `a3fdbf375`, script timers and reference-only expiry pass20 targeted cases/74.84s: five native cases (including two real Fire players), five creature-script controls, ten combat-reference controls (including threat-less NPC attackers and retained PvP/pets). Current-canonical SQLite backup `25995de4…` reproduces the pre-fix two RED/two controls PASS24.06s, then native five GREEN35.61s; canonical file SHA `7d9106fa…`, full provenance in `havoc-parity-2026-10-23/current-canonical-dummy-db.proof.json`. Compiler0warnings/fmt checkPASS; independent lead gate remains separate. Existing aura/attack systems are preserved. No follower policy, fixture exemption, coefficients or DPS parity claim. [Contract](../../specs/dps-parity.md#training-dummy-combat-exit).

### Attacked controlled-unit owners — 2026-10-22

TC a352b1fa `Unit.cpp:2271,9281–9287` engages the attacked unit and its direct charmer/owner. The engine's represented owner is `UnitSummonedBy`; `combat_refs::engage_attacked_target` now shares that rule across melee health commit, direct damage and native missile impact, before lethal damage can clear the pet's references. A threat-capable owner receives native `AddThreat(0)` (`Unit.cpp:8511–8520`), not transferred pet damage threat; an owner without a threat list receives combat references. Launch-only suppression does not engage the owner.

Pushed ignored REDs `bbda77823`/`96d709f76`, corrected native fixture `bff61bc51`, reproduce missing owner engagement through actual melee/direct133 impact/lethal damage and an NPC↔pet reference expiring during30quiet seconds. Separate production commits `2fbb45574` and `56e758740` give six native GREEN cases, including owner zero threat and both-controlled PvP refresh/expiry. Follower pet coverage, healer policy and pet stat scaling are unchanged. Stockade/Henry/Azure positioning controls are pending; these fixes do not claim every Azure pet death is prevented. [Contract](../../specs/pet-lifecycle.md), behavioral tests `combat_pet_engagement_tests.rs` and `combat_refs_tests.rs`.

Tests: `combat_refs_tests.rs` (lull, two opponents, PvP timeout, threat-less timeout, despawn, dead refused, Agony at launch, Polymorph without initial aggro).

## Group damage meter

`spell_cast::log_event` feeds `damage_meter.rs` before routing the owner-only combat log. Actor category totals fan out to the actor's online group; victim events fan out separately to the victim's group, so PvP need not share a roster. Each recipient's meter owns Current and Overall. Shield consumption credits its aura caster before removal; accepted player-death transitions fill missing Death lines and consume a marker to avoid double-counting logged deaths. Export limits and client migration fields live in the [damage-meter contract](../../specs/damage-meter.md).

Source: `crates/server/src/damage_meter.rs`, `auras/mod.rs::absorb_damage`, `death/transitions.rs::process_player_death_transitions`. Behavioral proof: `damage_meter_tests.rs`.

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
- **Armor mitigation**: `armor / (armor + K(attacker_level))`, capped at85% before damage truncation; native `K` comes from installed ExpectedStat, not a SimC table. [Aura280 follower policy and source-qualified inputs](../investigations/follower-armor-penetration.md).
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

- [Class defensives audit](../../defensives-audit.md) — melee immunity/absorb fixes, dodge and Brew proof; remaining Blur/Turtle limits (2026-10-09)

- [[tick-model]] — combat systems run in `FixedUpdate`; no global fixed-phase pipeline is implemented
- [[networking]] — CombatEvent broadcast over CombatChannel
- [[creature-aggro]] — creature aggro/threat sets the creature's CombatTarget; evade stops combat and restores HP
- [[creature-ai]] — chase and leash movement
- [[player-damage]] — weapon swings, hit table, armor and spell amounts
- [[world-data]] — spell_misc/spell_effect tables provide cast time, range, coefficients

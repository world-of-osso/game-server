# Authoritative spell casting

Server-side spell casting in `crates/server/src/spell_cast/`: validation, cast bars, costs, cooldowns, targets and effect resolution. It replaces the presentation-only cast lifecycle. See [spell casting](../wiki/systems/spell-casting.md).

## What it must do

- [x] Accept spell requests only for the authenticated in-world connection's player; a connection's stop request must not cancel another entity's cast.
- [x] Reject unknown spells, dead casters, school lockouts, cooldown, exhausted charges, moving casters (cast-time/channel spells), wrong target relation, out-of-range (min and max) targets and unaffordable costs, sending `CastFailed` to the owner. Active casts/GCD reject immediately outside the spell queue window; inside it, defer validation as below.
- [x] Instant spells resolve in the receiving tick; cast-time spells resolve on completion; channels resolve at start and tick per aura period.
- [x] Spend every applicable `SpellPower` cost in raw DB2 units (`PowerCostPct` of base mana for mana, of the max pool otherwise); aura-gated rows only while the caster has the aura.
- [x] Start GCD (at cast start), cooldown, category cooldown and charges; send `SpellCooldownUpdate`/`SpellChargesUpdate` to the owner.
- [x] Movement input cancels casts that are not cast-while-moving.
- [x] Resolve effects 2, 6/35/65 (auras), 10, 30, 31, 38, 58, 64 (bounded depth), 68, 121, 136; log unknown effects once per spell and skip them.
- [x] Fail the cast on unsupported implicit targets of handled effects.
- [x] Emit `CombatLogEvent` (CastStart, CastSuccess, Damage, Heal, Energize, Interrupt, Death) to source and target owners.
- [x] Reject player casts of spells not in `KnownSpells` (`NotSupported`, "spell not known"); NPC casters exempt.
- [x] Maintain class power pools with DB2 regen/decay; replicate `UnitPowers` (player `UnitLevel` belongs to class progression; NPC `UnitLevel` from the template).
- [x] Replicate raw whole power, fractional thousandths, signed regen rate, and charged combo-point indices. Resend on whole value, max, rate, display order or charged-point changes, not fraction alone; report zero rate on the tick a pool reaches its limit.
- [x] Replicate indexed rune ready times and full recharge duration, including queued runes' wait. Resend on spend, completion, duration or rune-count changes, not countdown ticks.
- [x] Use contract `CastFailReason` variants directly; `NotSupported` names the cause in `detail`.

## Source interrupt eligibility

Normal and channeled casts use the full TrinityCore `SpellInfo::CanBeInterrupted` rule rather
than unconditional eligibility: attr7 `NO_UI_NOT_INTERRUPTIBLE`, channel Damage/EnteringCombat
flags, player-only DamageCancels, DamageCancels0x400, the interrupter's exact-spell
ALLOW_INTERRUPT_SPELL aura, or SILENCE PreventionType with target MECHANIC_INTERRUPT immunity
and matching PREVENT_INTERRUPT checks. Missing SpellCategories means PreventionType0. Early
positive terms retain TC's bypass precedence; ignore-immunity bypasses only mechanic immunity.
Followers keep ordinary kicks and native spell data. An ineligible cast produces no interrupt
log or school lockout. Source eligibility is distinct from damage pushback and forced CastStop.
Existing kit/test conflicts must be reported before GREEN, not weakened to fit the fix.

## Spell queue window

- [x] Hold one player spell request when a cast or the requested spell's GCD is still active and **both** have at most 750 ms remaining (the hasted-GCD floor; Retail and TrinityCore use 400 ms, widened by user decision 2026-10-15). A newer admitted request replaces the held spell and its target/witness. A newer immediately executable player request supersedes it too; rejected requests do not replace it.
- [x] At admission resolve the spell and require that the authenticated player knows it. Do not check power, health costs, cooldowns, charges, target validity/range, movement or other start-time requirements.
- [x] After the cast finishes and its effects apply, and after the applicable GCD expires, attempt the held spell once through full `validate_start`, including `spell_costs`. Due missile effects run before this attempt; do not wait for future missile impacts. Resource gains can fund a spell that was unaffordable when admitted. A start-time failure emits normal owner `CastFailed` and consumes the slot.
- [x] Explicit stop of an active cast cancels its held request without attempting it. Hostile interruption, movement cancellation and death retain the request until release, then revalidate; school lockout or dead caster fails normally. Never retry a failed held request after resurrection.
- [x] Keep existing `SpellCastIntent`/`CastFailed` contracts; no client or shared-protocol change, queue acknowledgement or replicated pending state.

### Sources and proof limits

- Retail AddOns: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_APIDocumentationGenerated/SpellDocumentation.lua:511-517` exposes `C_Spell.GetSpellQueueWindow`; `Blizzard_DeprecatedSpellScript/Deprecated_SpellScript.lua:10-11` aliases it. **These Lua files contain no `SpellQueueWindow` CVar default literal.** Do not attribute a 400 ms default to them. The original 400 ms policy matched the pinned server reference below; the current 750 ms window is a user decision (2026-10-15) and has no external source.
- [TrinityCore a352b1fa Player.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Player/Player.cpp#L31517-L31559): `RequestSpellCast` replaces pending requests; `SPELL_QUEUE_TIME_WINDOW = 400ms`; admission tests remaining GCD and generic/melee cast time. `CanExecutePendingSpellCastRequest` at lines 31753-31777 waits for casting/GCD before preparing the spell. TrinityCore allows channel handover immediately; this task's specified current-cast completion gate is used for all existing `CastState` kinds instead.
- [TrinityCore a352b1fa SpellHandler.cpp:247-255](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Handlers/SpellHandler.cpp#L247-L255): explicit active-cast cancellation also calls `CancelPendingCastRequest`.
- [TrinityCore a352b1fa Unit.cpp:3158-3188](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Unit/Unit.cpp#L3158-L3188) interrupts only the active spell; `setDeathState` at 9178 onward interrupts active spells without clearing the player's pending request. Retaining pending through hostile interruption/death is an inference from these paths plus `CanExecutePendingSpellCastRequest`, not a live Retail observation.
- Owned `data/world.db`: Onslaught 315720 effect 799630 grants 300 raw Rage; Charge 100 effect 64 grants 200; Slam 1464 `spell_power.ID=127` costs 200 raw Rage; Mortal Strike 12294 row 9599 costs 300. Connection/fixed-tick tests preserve these effect/cost rows. The delayed-builder fixture changes only Onslaught's cast duration to 1500 ms (Retail Onslaught is instant) to prove completion ordering; a separate GCD-only test uses granted Charge's real effects after admission.

## How it works

- [Spell casting](../wiki/systems/spell-casting.md)

## Implementation inventory

- `crates/server/src/spell_cast/mod.rs`: receivers, validation, start/complete/cancel, `SpellHistory`, messages.
- `crates/server/src/spell_cast/targets.rs`: implicit targets, relations, range.
- `crates/server/src/spell_cast/effects.rs`: effect registry, damage/heal paths.
- `crates/server/src/spell_cast/powers.rs`: `PowerTypeTable`, `PowerPools`, regen, `UnitPowers` sync.
- `crates/server/src/spell_cast/runes.rs`: per-rune cooldowns and `UnitRunes` sync.
- `crates/server/src/networking.rs`: `Moving` input marker.
- `crates/server/src/combat.rs`: `enter_combat`, `mark_npc_dead_world`.
- `crates/server/src/spell_info/`: spell data including `spell_category` charges.

## Tests asserting this spec

- `crates/server/src/spell_cast/tests.rs` (real `world.db` spells): instant Mortal Strike cost/GCD/cooldown messages and repeat failures; Fireball cast time and combat log order; movement cancel; Kick interrupt and Fire lockout; OutOfRange `CastFailed`; Flash Heal overheal; Charge energize raw units and charges message; rage decay; bounded trigger chain; unsupported implicit target (`NotSupported`); unknown-spell rejection; warrior `UnitPowers`; Fireball scaling with computed Mage stats; percent mana cost from base mana; Flash Heal at a hostile kobold self-casts.
- `crates/server/src/spell_cast/queue_tests.rs`: builder effects before real rage spender; GCD-only resource gain; deferred NotEnoughResource; immediate outside-window cast/GCD failure; last request wins; explicit stop; hostile interrupt school lock; death failure/no stale resurrection cast; deferred range validation; inclusive exactly-750ms admission using the active cast's native time precision.
- `crates/server/src/zones.rs`: NPC `UnitLevel` and empty `UnitPowers`.
- `crates/server/src/spell_cast/class_resource_tests.rs`: fractional Essence and send cadence, charged-point addition/removal, form rates, same-tick prediction limits, rune spend/wait/completion, haste-scaled duration/rate changes and max-passive rune-count updates.

## Known gaps (current cycle)

- [x] Auras, aura-gated costs, procs, periodic effects: [auras spec](auras.md).
- [ ] Players have no gear or weapon stats (see [character stats](character-stats.md)); weapon hits use a placeholder.
- [ ] Target relations use faction reactions without reputation or PvP flags ([factions wiki](../wiki/systems/factions.md)). No facing check. NPCs do not cast and have no power pools.
- [ ] Combat log reaches only source/target owners, not nearby observers.
- [ ] End-to-end wire replication and client rendering are unverified here.

## Out of scope

- Learned-spell permissions (known-spells task), haste-scaled GCD, ground-targeted spells (target 87). Spell travel time: [spell-casting wiki](../wiki/systems/spell-casting.md) Execute.

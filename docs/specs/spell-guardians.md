# Temporary guardian actors and damage transfer

Temporary summoned actors use `spell_cast/{summon,guardian}.rs`; incoming damage sharing lives in `auras/damage_transfer.rs`. [Runtime and source boundaries](../wiki/systems/spell-casting.md#guardian-actors-and-damage-transfer).

## What it must do

### Actors
- [x] Spawn the native Grimoire Imp111859 effect28 creature416 with SummonProperties3314; cast its archive-bound3110 with native1750ms cast time, not immediately.
- [x] Subsequent casts inherit changed owner selection; absent selection stops new attacks.
- [x] Native27000ms duration and owner death remove the actor without replacing a persistent pet.
- [x] Import exact local TDB creature spell rows atomically; preserve prior bindings on duplicate-key failure and reject absent source data rather than synthesizing bindings.
- [x] Named49206's temporary Gargoyle uses its native pet-title3238 and authentic27829/51963 binding without enabling generic pet-title actors; [batch12 contract](spell-dummy-scripts.md#twelfth-batch-actorresourcesnapshot-consumers).

### Damage sharing
- [x] Native6940 aura81 redirects30% of post-mitigation damage to its caster for spell, periodic and real auto-attack paths; direct damage also shares.
- [x] Native6940 effect2 terminates transfer at20% caster health, not an invented threshold.
- [x] A caster on another map cannot receive shares.
- [x] Recipient absorbs consume redirected damage, immune recipients do not receive shares, and expired transfer auras stop sharing.
- [x] Dead protected units cannot transfer damage; concurrent queued melee swings share one remaining caster-health floor budget.

## How it works
- [Spell casting wiki](../wiki/systems/spell-casting.md#guardian-actors-and-damage-transfer).
- [Coverage and all40-spec table](../spell-guardians-coverage.md).

## Implementation inventory
- `crates/server/src/spell_cast/guardian.rs` — read-only binding loader, actor powers and normal creature casts.
- `crates/server/src/spell_cast/summon.rs` — effect28 creation and actor lifetime.
- `crates/server/src/spell_cast/effects.rs` — inherited existing demon power formula and spell/periodic sharing.
- `crates/server/src/auras/damage_transfer.rs` — school-mask splits and native Sacrifice floor.
- `crates/server/src/combat.rs` — melee splitting before health loss; nonrecursive redirect health application.
- `scripts/import-guardian-spells.py` — archive importer for a prepared database copy.
- `scripts/add_guardian_templates.py` — prepare authentic Darkglare103673 template/model from imported TDB tables on a copy; reject live world.db. Command: `python3 scripts/add_guardian_templates.py --db data/world.db.new`. Production import remains operator-owned.

## Tests asserting this spec
- `crates/server/src/spell_cast/guardian_tests.rs` — authentic effect28/binding actor fixture and real spell/melee/periodic/direct damage paths.
- `scripts/tests/test_guardian_spells.py` — exact rows, missing input and rollback.
- `scripts/tests/test_guardian_templates.py` — authentic Darkglare template/model, idempotent prepared-copy import, missing-model rollback and live-database refusal.

## Known gaps (current cycle)
- [ ] Wild Imp55659, Dreadstalker98035, Darkglare103673, Tyrant135002/250289 and other named guardian rotations require authentic creature-to-spell/cadence/budget data. See coverage receipts; no guessed six-bolt/energy policy.
- [ ] Non-demon guardian power inheritance, movement/AI priorities, untimed-repeat cadence and talent modifiers remain uncertified. [Batch13 source recheck](../spell-dummy13-coverage.md#capabilities--limits) preserves this gap: wolf1500ms template timing does not establish its damage curve. Simultaneous split reservations with recipient absorbs and floor-limit ordering lack complete Retail proof.
- [ ] Root111859 Dummy0 cleanse bridge remains unbound; effect28 fixture is not whole-root execution proof.
- [ ] Blade Flurry secondary geometry, Atonement soft-cap curve, Touch of Karma and Spirit Link scripts remain underived.

## Out of scope
- Live world.db import, UDP5000, master merge, deployment and full rotation/client parity are excluded by this task.

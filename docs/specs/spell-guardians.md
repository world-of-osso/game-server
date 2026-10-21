# Temporary guardian actors and damage transfer

Temporary summoned actors use `spell_cast/{summon,guardian}.rs`; incoming damage sharing lives in `auras/damage_transfer.rs`. [Runtime and source boundaries](../wiki/systems/spell-casting.md#guardian-actors-and-damage-transfer).

## What it must do

### Actors
- [x] Emit one CombatLogKind::Summon per actual spell-created unit: source=summoner, target=unit, spell_id=actual summoning spell. Guardian logs reach the owning player through summon ancestry; quest/PvP/meter credit policies are unchanged. Login/script/journal restoration does not fabricate spell events.
- [x] Spawn the native Grimoire Imp111859 effect28 creature416 with SummonProperties3314; cast its archive-bound3110 with native1750ms cast time, not immediately.
- [x] Subsequent casts inherit changed owner selection; absent selection stops new attacks.
- [x] Native27000ms duration and owner death remove the actor without replacing a persistent pet.
- [x] Import exact local TDB creature spell rows atomically; preserve prior bindings on duplicate-key failure and reject absent source data rather than synthesizing bindings.
- [x] Named49206's temporary Gargoyle uses its native pet-title3238 and authentic27829/51963 binding without enabling generic pet-title actors; [batch12 contract](spell-dummy-scripts.md#twelfth-batch-actorresourcesnapshot-consumers).

### Feral Spirit owner inheritance
- [x] Consume authenticated29264 owner melee AP×0.31 (integer AP and bonus SP), level×3–5 weapon range and template BaseAttackTime. Subsequent swings/spells read live owner power; other families keep their separate formulas. Intrinsic wolf base AP is explicitly unsupported (`FERAL_SPIRIT_INTRINSIC_AP_SOURCE`): authentic archive has no29264 pet_levelstats; TC's fake Strength22 must not be substituted.
- [x] Actual51533→228562 creates two29264 wolves that deal positive damage to their owner's training-dummy victim without fabricated wolf AP; native owner-power changes affect subsequent damage. Source classification and stat/attack consumption must be proved separately from Summon event delivery.

### Source-backed Darkglare attack
- [x] Native205180 effect28's103673 actor casts205231, explicitly named in the local root description, with native2000ms cast time and20000ms actor lifetime; an empty archive binding table must not disable this named binding.
- [x] Reuse existing demon SP contribution live: changed owner spell power affects subsequent Eye Beams. This is not new Retail inheritance-percentage proof.
- [x] Retarget from owner selection, stop initiating without selection, and expire without occupying/replacing a persistent pet.
- [ ] Root's DoT extension, Presence, per-DoT bonus and Dummy/aura4 consumers remain unconverted; direct effect28 tests are not whole-root proof.

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
- [x] Source-qualified native104317/55659 Wild Imp binds104318; `WARLOCK_WILD_IMP_OWNER_SP_SIMC` takes constructor702–705 SP1.2 from pinned SimC76f8118, retaining native104318 coefficient and other demons' AP/SP/health policies. Authenticated template/model preparation is copy-only; canonical publication is separate. Exact Wild Imps retire when native next-bolt Energy cost exceeds remaining Energy, preserving their final in-flight bolt and existing independent expiry/owner cleanup.
- [x] Natural Energy-exhausted Wild Imps roll Demoniac only when426115 is actually learned: native267102 effect0 nominal chance, pinned SimC76f8118 capped accumulated PRD21, counter resets on success. Earn264173 through ordinary triggered spell application, not fabricated aura state.
- [ ] HoG→Wild Imp lifecycle/energy exhaustion/Core, Dreadstalker98035, Tyrant135002/250289 and other named guardian rotations require authentic cadence/budget/routing data and behavioral proof. Darkglare's base Eye Beam binding is derived above; its modifiers remain open. See coverage receipts; no guessed six-bolt/energy policy.
- [ ] Other non-demon guardian AP/SP inheritance, owner-to-pet/guardian stamina share and other melee weapon-damage curves remain unconverted. Feral Spirit's separately authenticated owner/weapon terms above exclude intrinsic base AP; no percentage or flat-damage substitute fills that gap. Archive pet_levelstats contains base attributes, not inheritance coefficients, and cannot establish level90 scaling. Xuen123999→123996 is an authentic driver, but actor AP inheritance and target policy remain unproved. See [follow-up source receipts and all40 table](../spell-guardians2-coverage.md). Movement/AI priorities, untimed-repeat cadence and talent modifiers remain uncertified. [Batch13 source recheck](../spell-dummy13-coverage.md#capabilities--limits) preserves this gap: wolf1500ms template timing does not establish its damage curve. Simultaneous split reservations with recipient absorbs and floor-limit ordering lack complete Retail proof.
- [ ] Root111859 Dummy0 cleanse bridge remains unbound; effect28 fixture is not whole-root execution proof.
- [ ] Blade Flurry secondary geometry, Atonement soft-cap curve, Touch of Karma and Spirit Link scripts remain underived.

## Out of scope
- Live world.db import, UDP5000, master merge, deployment and full rotation/client parity are excluded by this task.

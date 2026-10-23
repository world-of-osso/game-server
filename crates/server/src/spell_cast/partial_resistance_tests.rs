//! Pinned TC a352b1fa Unit.cpp1779–1888: raw resistance, not a classic level divisor.
//! Native aura application installs recipient/caster state; ordinary cast/missile and
//! periodic schedules exercise mitigation. This is not buff cast-admission or a legal build.
use super::test_harness::{Harness, harness, world_db};
use bevy::prelude::Entity;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const FIREBALL: u32 = 133;
const FLAME_SHOCK: u32 = 188389;
const MAYPOLE: u32 = 29127; // Native aura22, +10 fire resistance.
const AUTO_ADVERTISER: u32 = 1218304; // Native aura269, ignore50%, school mask127.
const SHIELD: u32 = 17;

fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[FIREBALL, FLAME_SHOCK, MAYPOLE, AUTO_ADVERTISER, SHIELD],
        8,
    )
    .expect("native Mainline source rows required")
}

fn apply_native(h: &mut Harness, target: Entity, spell_id: u32, aura_type: u32) {
    let effect = world_db()
        .unwrap()
        .store
        .get(spell_id)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == aura_type)
        .unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target,
            spell_id,
            effect_mask: 1 << effect.index,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), target, spell_id));
}

fn remove_maypole(h: &mut Harness) {
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.npc,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == MAYPOLE,
    );
}

fn damage_events(h: &Harness, spell_id: u32) -> Vec<CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(spell_id)
                && event.target == Some(h.npc.to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .cloned()
        .collect()
}

fn cast_fireball(h: &mut Harness) -> CombatLogEvent {
    let _rng = crate::random::seed_scope(1779);
    h.inbox.log.clear();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(120);
    assert!(
        h.inbox.failed.is_empty(),
        "ordinary Fireball admission: {:?}",
        h.inbox.failed
    );
    let events = damage_events(h, FIREBALL);
    assert_eq!(
        events.len(),
        1,
        "actual native Fireball impact required: {:?}",
        h.inbox.log
    );
    events[0].clone()
}

fn shield_remaining(h: &Harness) -> f32 {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .into_iter()
        .flat_map(|auras| auras.iter())
        .filter(|aura| aura.spell_id == SHIELD)
        .flat_map(|aura| &aura.effects)
        .filter(|effect| effect.aura_type == 69)
        .map(|effect| effect.amount)
        .sum()
}

#[test]
#[ignore = "RED: native magical resistance has no damage/receipt path"]
fn native_partial_resistance_fireball_uses_tc_raw_buckets_and_ignore_modifier() {
    let mut h = fixture();
    let target = h.npc;
    let caster = h.caster;
    apply_native(&mut h, target, MAYPOLE, 22);
    apply_native(&mut h, caster, AUTO_ADVERTISER, 269);
    let before = h.health(target);
    let partial = cast_fireball(&mut h);
    assert!(
        partial.resisted > 0 && partial.amount > 0,
        "native10fire and50%ignore must produce actual partial resistance: {partial:?}"
    );
    let raw = partial.amount + partial.resisted;
    // TC's raw average10 exhausts the11-entry CDF: bucket11, then native50%ignore.
    let expected = (((raw * 11) as f32 / 10.0) * 0.5) as i32;
    assert_eq!(partial.resisted, expected);
    assert!((before - h.health(target) - partial.amount as f32).abs() <= 1.0);
    remove_maypole(&mut h);
    let control = cast_fireball(&mut h);
    assert_eq!(control.resisted, 0);
    assert_eq!(
        control.amount, raw,
        "same seeded native source amount, no resistance"
    );
    assert_eq!(control.crit, partial.crit);
}

#[test]
#[ignore = "RED: magical resistance must precede native shield consumption"]
fn native_partial_resistance_full_resist_preserves_shield_until_resistance_removed() {
    let mut h = fixture();
    let target = h.npc;
    apply_native(&mut h, target, MAYPOLE, 22);
    apply_native(&mut h, target, SHIELD, 69);
    let shield = shield_remaining(&h);
    assert!(shield > 0.0, "native Power Word: Shield capacity required");
    let before = h.health(target);
    let resisted = cast_fireball(&mut h);
    assert_eq!(
        shield_remaining(&h),
        shield,
        "TC resistance runs before SCHOOL_ABSORB"
    );
    assert!(resisted.resisted > 0);
    assert_eq!((resisted.amount, resisted.absorbed), (0, 0));
    assert_eq!(h.health(target), before);
    remove_maypole(&mut h);
    let absorbed = cast_fireball(&mut h);
    assert_eq!(absorbed.resisted, 0);
    assert!(absorbed.absorbed > 0);
    assert!(shield_remaining(&h) < shield);
}

#[test]
#[ignore = "RED: native periodic damage also omits partial magical resistance"]
fn native_partial_resistance_periodic_flame_shock_uses_same_recipient_pipeline() {
    let mut h = fixture();
    let target = h.npc;
    let caster = h.caster;
    apply_native(&mut h, target, MAYPOLE, 22);
    apply_native(&mut h, caster, AUTO_ADVERTISER, 269);
    apply_native(&mut h, target, FLAME_SHOCK, 3);
    let before = h.health(target);
    h.advance(125); // Native2000ms ticks, not a fabricated periodic callback.
    let ticks = damage_events(&h, FLAME_SHOCK);
    assert!(
        !ticks.is_empty(),
        "actual native Flame Shock periodic receipts required"
    );
    for tick in &ticks {
        assert!(tick.periodic);
        assert!(
            tick.amount > 0 && tick.resisted > 0,
            "native periodic partial resistance: {tick:?}"
        );
        let raw = tick.amount + tick.resisted;
        assert_eq!(tick.resisted, (((raw * 11) as f32 / 10.0) * 0.5) as i32);
    }
    let dealt: i32 = ticks.iter().map(|tick| tick.amount).sum();
    assert!((before - h.health(target) - dealt as f32).abs() <= 1.0);
}

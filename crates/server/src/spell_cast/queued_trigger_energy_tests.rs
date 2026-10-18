//! Native Energy drain flight and queued-child NoPower; unchanged imported effects.
use bevy::prelude::*;
use shared::components::{Mana, Position, PowerType, UnitLevel};

use super::test_harness::*;

const SIPHON_ANIMA: u32 = 138644;
const GYMER_PARENT: u32 = 55427;
const GYMER_CHILD: u32 = 55426;
const SKUL: u32 = 10393;
const NATIVE_ROGUE_ENERGY: f32 = 100.0;
const NATIVE_CHILD_COST: f32 = 30.0;

fn fixture(control_class: u8, enemy_x: f32) -> (Harness, Entity, Entity) {
    let mut h = harness(Vec::new(), &[SIPHON_ANIMA, GYMER_PARENT, GYMER_CHILD], 4).unwrap();
    let db_path = &world_db().unwrap().powers_db;
    let source =
        rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app
        .insert_resource(crate::world_data::load_creature_spell_stats(&source));
    h.app
        .insert_resource(crate::world_data::load_world_data(db_path));
    let caster = h.caster;
    let drainer =
        crate::zones::summon_creature(h.app.world_mut(), caster, SKUL, [enemy_x, 0.0, 0.0, 0.0])
            .unwrap();
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = enemy_x;
    assert_eq!(h.app.world().get::<UnitLevel>(drainer).unwrap().0, 58);
    let control = spawn_player(&mut h.app, control_class, 0.0);
    h.advance(1);
    assert_eq!(h.power(PowerType::Energy), NATIVE_ROGUE_ENERGY);
    (h, control, drainer)
}

fn energy(h: &Harness, unit: Entity) -> f32 {
    super::powers::power_value(h.app.world(), unit, PowerType::Energy)
        .unwrap()
        .0
}

fn launch_siphon(h: &mut Harness, drainer: Entity) {
    // Native Level58 PlayerMana18880 * BasePoints1/100 rounds189, not raw1.
    // One unmodified cast can therefore exhaust the natural100 Energy pool.
    super::creature::creature_cast(h.app.world_mut(), drainer, SIPHON_ANIMA, None, true).unwrap();
}

fn mana(h: &Harness, unit: Entity) -> f32 {
    h.app.world().get::<Mana>(unit).unwrap().current
}

#[test]
fn native_energy_siphon_keeps_missile_flight_and_does_not_drain_mage_mana_or_health() {
    let (mut h, mage, drainer) = fixture(8, 3.0);
    let energy_before = h.power(PowerType::Energy);
    let mage_mana = mana(&h, mage);
    let rogue_mana = mana(&h, h.caster);
    let drainer_mana = mana(&h, drainer);
    let caster_health = h.health(h.caster);
    let mage_health = h.health(mage);
    let enemy_health = h.health(h.npc);
    let drainer_health = h.health(drainer);
    launch_siphon(&mut h, drainer);
    assert_eq!(h.power(PowerType::Energy), energy_before);
    // Native speed20 and minimum missile distance5yd require250ms, even at3yd.
    h.advance(3);
    assert_eq!(h.power(PowerType::Energy), energy_before);
    h.advance(4);
    let remaining = h.power(PowerType::Energy);
    assert!((0.0..5.0).contains(&remaining), "remaining={remaining}");
    assert_eq!(mana(&h, mage), mage_mana);
    assert_eq!(mana(&h, h.caster), rogue_mana);
    assert_eq!(mana(&h, drainer), drainer_mana);
    assert_eq!(h.health(h.caster), caster_health);
    assert_eq!(h.health(mage), mage_health);
    assert_eq!(h.health(h.npc), enemy_health);
    assert_eq!(h.health(drainer), drainer_health);
}

#[test]
fn native_energy_trigger64_child_refuses_after_siphon_impact_without_charging() {
    // Gymer's native area is12yd;7yd excludes unrelated5yd melee retaliation.
    let (mut h, control, drainer) = fixture(4, 7.0);
    assert_eq!(energy(&h, control), NATIVE_ROGUE_ENERGY);
    let control_health = h.health(control);
    let caster_health = h.health(h.caster);
    let target_health = h.health(h.npc);
    let drainer_health = h.health(drainer);
    launch_siphon(&mut h, drainer);
    h.advance(8); // Native7yd/20yd-per-second flight is350ms; normal regen stays enabled.
    let remaining = h.power(PowerType::Energy);
    assert!((0.0..NATIVE_CHILD_COST).contains(&remaining));
    assert_eq!(remaining, energy(&h, control));
    let child = world_db().unwrap().store.get(GYMER_CHILD).unwrap();
    assert!(matches!(
        super::spell_costs(h.app.world(), h.caster, child),
        Err(super::CastFailure::NotEnoughPower(PowerType::Energy))
    ));
    // Unlike470411's Mana fixture, this native parent has NO power cost and
    // queues AFTER depletion. This is not enqueue-before-drain proof.
    super::creature::creature_cast(
        h.app.world_mut(),
        h.caster,
        GYMER_PARENT,
        Some(h.npc),
        false,
    )
    .unwrap();
    assert_eq!(h.power(PowerType::Energy), remaining);
    assert_eq!(h.health(h.npc), target_health);
    h.advance(1);
    let after_dispatch = h.power(PowerType::Energy);
    assert_eq!(after_dispatch, energy(&h, control));
    assert!((0.0..NATIVE_CHILD_COST).contains(&after_dispatch));
    assert_eq!(
        h.health(h.caster),
        caster_health,
        "post-dispatch health attribution: roles={:?}, log={:?}",
        [h.caster, h.npc, drainer, control],
        h.inbox.log
    );
    assert_eq!(h.health(h.npc), target_health);
    assert_eq!(h.health(drainer), drainer_health);
    assert_eq!(h.health(control), control_health);
    assert!(
        h.inbox
            .failed
            .iter()
            .all(|failure| failure.spell_id != GYMER_CHILD)
    );
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        GYMER_CHILD,
        Some(h.npc),
        0,
    ));
    assert!(h.health(h.npc) > 0.0 && h.health(h.npc) < target_health);
    assert_eq!(h.power(PowerType::Energy), after_dispatch);
}

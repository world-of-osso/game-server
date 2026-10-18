//! Real level32 Crenna, sourced kit rows and native resource/cast-state boundaries.
use super::pull_forecast_tests::{add_native_healer, source_fixture};
use super::{Follower, FollowerCatalog, pull_readiness};
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{CostPool, applicable_costs, creature_cast, spell_costs};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Level, Mana, Position, PowerType, UnitLevel};

fn fixture(spells: &[u32]) -> (Harness, Entity, Entity) {
    let (mut h, tank, _) = source_fixture(46249);
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((Level(32), UnitLevel(32)));
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::catalog::load_catalog(&db).unwrap());
    let profile = h.app.world().get::<Follower>(tank).unwrap().profile.clone();
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    let healer = add_native_healer(&mut h);
    h.advance(3);
    // Bound the fixture to real kit rows, not invented spells or costs.
    h.app
        .world_mut()
        .resource_mut::<FollowerCatalog>()
        .kits
        .retain(|entry| entry.spec_id == 105 && spells.contains(&entry.spell_id));
    (h, tank, healer)
}

fn mana_cost(h: &Harness, healer: Entity, id: u32) -> f32 {
    let spell = h.app.world().resource::<SpellStore>().get(id).unwrap();
    applicable_costs(h.app.world(), healer, spell)
        .unwrap()
        .into_iter()
        .filter(|(pool, _, _)| *pool == CostPool::Power(PowerType::Mana))
        .map(|(_, amount, _)| amount)
        .sum()
}

#[test]
fn tank_pull_healer_ready_affordable_sourced_hot_is_not_default_regrowth() {
    let (mut h, tank, healer) = fixture(&[774, 8936]);
    let party = [h.caster, tank, healer];
    let hot = mana_cost(&h, healer, 774);
    let regrowth = mana_cost(&h, healer, 8936);
    assert!(
        hot > 0.0 && regrowth > hot,
        "actual source costs: Rejuvenation={hot}, Regrowth={regrowth}"
    );
    let available = (hot + regrowth) * 0.5;
    h.app.world_mut().get_mut::<Mana>(healer).unwrap().current = available;
    let default = h.app.world().resource::<SpellStore>().get(8936).unwrap();
    assert!(spell_costs(h.app.world(), healer, default).is_err());
    let before_hp = h.health(tank);
    let ready = pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
        .unwrap()
        .unwrap();
    assert_eq!(ready.healer, healer);
    assert_eq!(ready.spell_id, 774);
    let cost: f32 = ready
        .costs
        .iter()
        .filter(|(pool, _)| *pool == CostPool::Power(PowerType::Mana))
        .map(|(_, amount)| amount)
        .sum();
    assert_eq!(cost, hot);
    assert_eq!(
        h.app.world().get::<Mana>(healer).unwrap().current,
        available
    );
    assert_eq!(
        h.health(tank),
        before_hp,
        "readiness must not cast to manufacture proof"
    );
}

#[test]
fn tank_pull_healer_ready_does_not_veto_or_cancel_an_admitted_native_cast() {
    let (mut h, tank, healer) = fixture(&[774, 8936]);
    let party = [h.caster, tank, healer];
    creature_cast(h.app.world_mut(), healer, 8936, Some(tank), false).unwrap();
    let cast = h.app.world().get::<CastState>(healer).unwrap();
    assert_eq!((cast.spell_id, cast.target), (8936, tank.to_bits()));
    let before = h.app.world().get::<Mana>(healer).unwrap().current;
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_some()
    );
    let after = h.app.world().get::<CastState>(healer).unwrap();
    assert_eq!((after.spell_id, after.target), (8936, tank.to_bits()));
    assert_eq!(h.app.world().get::<Mana>(healer).unwrap().current, before);
}

#[test]
fn tank_pull_healer_ready_requires_sourced_range_and_same_copy() {
    let (mut h, tank, healer) = fixture(&[774]);
    let party = [h.caster, tank, healer];
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_some()
    );
    let from = *h.app.world().get::<Position>(healer).unwrap();
    let spell = h.app.world().resource::<SpellStore>().get(774).unwrap();
    let (_, range) = crate::spell_cast::targets::spell_range(spell, false);
    let tank_at = *h.app.world().get::<Position>(tank).unwrap();
    h.app.world_mut().entity_mut(healer).insert(Position {
        x: tank_at.x + range + 1.0,
        ..from
    });
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_none()
    );
    h.app.world_mut().entity_mut(healer).insert(from);
    let map = *h.app.world().get::<InMap>(healer).unwrap();
    h.app.world_mut().entity_mut(healer).insert(InMap {
        instance_id: map.instance_id + 1,
        ..map
    });
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_none()
    );
}

#[test]
fn tank_pull_healer_ready_rejects_incapacity_or_no_affordable_heal() {
    let (mut h, tank, healer) = fixture(&[774]);
    let party = [h.caster, tank, healer];
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_some()
    );
    let mana = *h.app.world().get::<Mana>(healer).unwrap();
    h.app.world_mut().get_mut::<Mana>(healer).unwrap().current = 0.0;
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_none()
    );
    h.app.world_mut().entity_mut(healer).insert(mana);
    let health = *h.app.world().get::<Health>(healer).unwrap();
    h.app.world_mut().get_mut::<Health>(healer).unwrap().current = 0.0;
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_none()
    );
    h.app.world_mut().entity_mut(healer).insert(health);
    let enemy = h.npc;
    let map = *h.app.world().get::<InMap>(healer).unwrap();
    let at = *h.app.world().get::<Position>(healer).unwrap();
    h.app.world_mut().entity_mut(enemy).insert((map, at));
    creature_cast(h.app.world_mut(), enemy, 118, Some(healer), true).unwrap();
    h.advance(1);
    assert!(crate::auras::has_aura(h.app.world(), healer, 118));
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_none()
    );
}

fn swiftmend_charge_count(h: &Harness, healer: Entity) -> usize {
    let spell = h.app.world().resource::<SpellStore>().get(18562).unwrap();
    let Some(category) = spell
        .categories
        .as_ref()
        .filter(|category| category.charge_category != 0 && category.max_charges != 0)
    else {
        return 1;
    };
    let mods = crate::auras::charge_mods(h.app.world(), healer, category.charge_category);
    (category.max_charges as i32 + mods.extra_charges).clamp(0, u8::MAX as i32) as usize
}

#[test]
fn tank_pull_healer_ready_honors_real_level_and_consumed_native_cooldown() {
    let (mut h, tank, healer) = fixture(&[18562]);
    let party = [h.caster, tank, healer];
    creature_cast(h.app.world_mut(), healer, 774, Some(tank), false).unwrap();
    h.advance(1);
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_some()
    );
    let minimum = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .map(|entry| entry.min_level)
        .min()
        .unwrap();
    assert!(minimum > 0);
    h.app
        .world_mut()
        .entity_mut(healer)
        .insert((Level(minimum - 1), UnitLevel(minimum - 1)));
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_none()
    );
    h.app
        .world_mut()
        .entity_mut(healer)
        .insert((Level(32), UnitLevel(32)));
    let charges = swiftmend_charge_count(&h, healer);
    assert!(charges > 0);
    for _ in 0..charges {
        creature_cast(h.app.world_mut(), healer, 774, Some(tank), false).unwrap();
        creature_cast(h.app.world_mut(), healer, 18562, Some(tank), false).unwrap();
        h.advance(1);
    }
    // Restore the real target prerequisite, so refusal must still honor recovery.
    creature_cast(h.app.world_mut(), healer, 774, Some(tank), false).unwrap();
    h.advance(1);
    let spell = h.app.world().resource::<SpellStore>().get(18562).unwrap();
    assert!(spell_costs(h.app.world(), healer, spell).is_ok());
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_none()
    );
}

#[test]
fn tank_pull_healer_area_ready_requires_tank_in_native_tranquility_area() {
    let (mut h, tank, healer) = fixture(&[740]);
    let source_level = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .find(|entry| entry.spell_id == 740)
        .unwrap()
        .min_level;
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((Level(source_level), UnitLevel(source_level)));
    for unit in [tank, healer] {
        let profile = h.app.world().get::<Follower>(unit).unwrap().profile.clone();
        super::spawning::initialize(h.app.world_mut(), owner, unit, profile).unwrap();
    }
    h.advance(3);
    let at = *h.app.world().get::<Position>(tank).unwrap();
    h.app.world_mut().entity_mut(healer).insert(at);
    let party = [owner, tank, healer];
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_some()
    );
    h.app.world_mut().entity_mut(healer).insert(Position {
        x: at.x + 1000.0,
        ..at
    });
    assert!(
        pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party)
            .unwrap()
            .is_none(),
        "caster-area healing cannot certify an out-of-area tank"
    );
}

#[test]
fn tank_pull_healer_ready_missing_kit_does_not_substitute_default_profile_heal() {
    let (mut h, tank, healer) = fixture(&[774, 8936]);
    let party = [h.caster, tank, healer];
    h.app.world_mut().remove_resource::<FollowerCatalog>();
    assert!(pull_readiness::read_ready_heal(h.app.world_mut(), tank, &party).is_err());
}

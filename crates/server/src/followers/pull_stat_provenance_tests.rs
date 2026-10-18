//! Read-only native aura/stat contribution comparison, not an admission or HP override.
use super::Follower;
use super::pull_admission_tests::add_native_dps;
use super::pull_forecast_tests::{add_native_healer, source_fixture_with_plugins};
use bevy::prelude::*;
use shared::components::{Health, Level, UnitLevel};

fn print_passive_contributions(world: &World, unit: Entity, label: &str) {
    let stats = world.get::<crate::stats::CharacterStats>(unit).unwrap();
    let mods = world.get::<crate::stats::StatModifiers>(unit).unwrap();
    let auras = world.get::<crate::auras::ActiveAuras>(unit).unwrap();
    eprintln!(
        "{label}: health={:?}, stamina={}, modifiers={mods:?}",
        world.get::<Health>(unit),
        stats.primary.stamina
    );
    let mut entries: Vec<_> = auras.iter().collect();
    entries.sort_by_key(|aura| aura.spell_id);
    for aura in entries {
        let name = &world
            .resource::<crate::spell_info::SpellStore>()
            .get(aura.spell_id)
            .unwrap()
            .name;
        let effects: Vec<_> = aura
            .effects
            .iter()
            .map(|effect| {
                (
                    effect.index,
                    effect.aura_type,
                    effect.amount,
                    effect.misc_values,
                    effect.class_mask,
                )
            })
            .collect();
        eprintln!(
            "{label}: spell{} {name}, passive={}, source={:?}, caster={:?}, stacks={}, effects={effects:?}",
            aura.spell_id, aura.passive, aura.source, aura.caster, aura.stacks
        );
    }
}

#[test]
fn native_tank_stat_provenance_partial_and_full_kit() {
    let (mut h, tank, _) = source_fixture_with_plugins(46249, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((Level(32), UnitLevel(32)));
    let profile = h.app.world().get::<Follower>(tank).unwrap().profile.clone();
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile.clone()).unwrap();
    h.advance(3);
    print_passive_contributions(h.app.world(), tank, "partial");
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::catalog::load_catalog(&db).unwrap());
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    h.advance(3);
    print_passive_contributions(h.app.world(), tank, "full-solo");
    add_native_healer(&mut h);
    let map = *h.app.world().get::<crate::maps::InMap>(tank).unwrap();
    add_native_dps(&mut h, map);
    h.advance(3);
    print_passive_contributions(h.app.world(), tank, "full-party");
}

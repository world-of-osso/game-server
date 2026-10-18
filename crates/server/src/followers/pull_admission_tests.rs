//! Native budget composition over a supplied route footprint; full route exposure has
//! separate real-map coverage. Effects, deaths, resources and timing stay native.
use super::pull_budget::{Decision, Reason};
use super::pull_forecast_tests::{add_native_healer, source_fixture, spawn_source_actor};
use super::{Follower, FollowerCatalog, pull_admission, pull_history};
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use crate::spell_info::SpellStore;
use crate::world_data::WorldData;
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Level, Mana, UnitLevel};
use std::collections::{BTreeSet, HashMap};

// Native class2/level32 stamina980, Aegis30% + Plate5%, gtHpPerSta32=8.
const NATIVE_GARRICK_MAX_HEALTH: f32 = 10584.0;

struct Fixture {
    h: Harness,
    tank: Entity,
    healer: Entity,
    members: Vec<Entity>,
    first: Entity,
    first_guid: u32,
    second: Entity,
}

fn spawn_source(h: &mut Harness, guid: u32, map: InMap) -> Entity {
    let data = crate::test_data::world_data();
    let spawn = data.spawns.iter().find(|spawn| spawn.guid == guid).unwrap();
    let template = data.template_for(spawn.template_entry, 1).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        HashMap::from([(34, 791060)]),
    );
    spawn_source_actor(h.app.world_mut(), data, template, spawn, map, &mut ground)
}

fn add_native_dps(h: &mut Harness, map: InMap) -> Vec<Entity> {
    let profiles: Vec<_> = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .profiles
        .iter()
        .filter(|profile| profile.role == shared::group::GroupRole::Dps)
        .take(2)
        .cloned()
        .collect();
    assert_eq!(profiles.len(), 2);
    profiles
        .into_iter()
        .map(|profile| {
            let native =
                crate::spell_cast::test_harness::load_native_creature_world(&[profile.entry]);
            h.app
                .world_mut()
                .resource_mut::<WorldData>()
                .templates
                .extend(native.templates);
            let unit = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
            h.app.world_mut().entity_mut(unit).insert((
                shared::components::Npc {
                    template_id: profile.entry,
                    name: profile.name.clone(),
                },
                shared::components::UnitPowers::default(),
                map,
            ));
            super::spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
            unit
        })
        .collect()
}

fn fixture() -> Fixture {
    let (mut h, tank, first) = source_fixture(46249);
    h.app.add_plugins(crate::stats::StatsPlugin);
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
    let native = crate::test_data::world_data();
    {
        let mut data = h.app.world_mut().resource_mut::<WorldData>();
        data.spawns = native
            .spawns
            .iter()
            .filter(|spawn| spawn.map == 34)
            .cloned()
            .collect();
        data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
        data.formations = native.formations.clone();
        let entries: BTreeSet<_> = data
            .spawns
            .iter()
            .map(|spawn| spawn.template_entry)
            .collect();
        for entry in entries {
            data.templates
                .insert(entry, native.template_for(entry, 1).unwrap().clone());
        }
    }
    let first_guid = h
        .app
        .world()
        .get::<crate::zones::CreatureOrigin>(first)
        .unwrap()
        .guid;
    let second_guid = native
        .spawns
        .iter()
        .find(|spawn| {
            spawn.map == 34
                && spawn.template_entry == 46249
                && spawn.guid != first_guid
                && spawn.spawns_at(1)
        })
        .unwrap()
        .guid;
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let second = spawn_source(&mut h, second_guid, map);
    let mut members = vec![tank, healer];
    members.extend(add_native_dps(&mut h, map));
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: members.clone(),
            player_role: shared::group::GroupRole::Dps,
        });
    h.advance(3);
    assert_eq!(
        h.app.world().get::<Health>(tank).unwrap().max,
        NATIVE_GARRICK_MAX_HEALTH,
        "native production stat recomputation must apply sourced passives before budget observations"
    );
    pull_history::sample_all(h.app.world_mut());
    Fixture {
        h,
        tank,
        healer,
        members,
        first,
        first_guid,
        second,
    }
}

impl Fixture {
    fn read(&mut self, units: &[Entity]) -> Result<Decision, String> {
        let projected = units
            .iter()
            .map(|unit| {
                self.h
                    .app
                    .world()
                    .get::<crate::zones::CreatureOrigin>(*unit)
                    .unwrap()
                    .guid
            })
            .collect();
        pull_admission::read_pull_budget(self.h.app.world_mut(), self.tank, &projected, 1)
    }

    fn sample(&mut self, frames: usize) {
        for _ in 0..frames {
            self.h.advance(1);
            pull_history::sample_all(self.h.app.world_mut());
        }
    }

    fn damage(&mut self, target: Entity, amount: f32) {
        let spell = self
            .h
            .app
            .world()
            .resource::<SpellStore>()
            .get(133)
            .unwrap()
            .clone();
        let context = crate::spell_cast::effects::EffectCtx {
            caster: self.h.caster,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        crate::spell_cast::effects::deal_damage(
            self.h.app.world_mut(),
            &context,
            target,
            amount,
            false,
            false,
        );
    }
}

fn measured_fixture() -> Fixture {
    let mut f = fixture();
    f.sample(100);
    let health = crate::combat::forecast::read_generic_creature(f.h.app.world(), f.first, f.tank)
        .unwrap()
        .remaining_health;
    f.damage(f.first, health as f32 * 2.0);
    assert_eq!(
        f.h.health(f.first),
        0.0,
        "actual source fight ends through ordinary native damage/death"
    );
    f.sample(120);
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    assert_eq!(history.completed_fights, 1);
    assert!(history.group_dps.is_some_and(|value| value > 0.0));
    assert_eq!(
        history.healer_hps,
        Some(0.0),
        "measured zero effective healing is not a nominal heal rate"
    );
    assert!(history.party_stable);
    f
}

#[test]
fn tank_pull_admission_native_bootstrap_is_one_source_pack_without_prior_rates() {
    let mut f = fixture();
    assert!(matches!(
        f.read(&[f.second]).unwrap(),
        Decision::Allow {
            bootstrap: true,
            forecast_health: None
        }
    ));
    assert_eq!(
        f.read(&[f.first, f.second]).unwrap(),
        Decision::Wait(Reason::FirstSinglePack)
    );
    let mana = *f.h.app.world().get::<Mana>(f.healer).unwrap();
    f.h.app
        .world_mut()
        .get_mut::<Mana>(f.healer)
        .unwrap()
        .current = 0.0;
    assert_eq!(f.read(&[f.second]).unwrap(), Decision::Wait(Reason::Healer));
    f.h.app.world_mut().entity_mut(f.healer).insert(mana);
    f.h.app.world_mut().remove_resource::<FollowerCatalog>();
    assert!(f.read(&[f.second]).is_err());
}

#[test]
fn tank_pull_admission_native_measured_capacity_and_thirty_percent_reserve() {
    let mut f = measured_fixture();
    let decision = f.read(&[f.second]).unwrap();
    let health = *f.h.app.world().get::<Health>(f.tank).unwrap();
    assert!(
        matches!(decision,Decision::Allow {bootstrap:false,forecast_health:Some(value)} if value>=f64::from(health.max)*0.30)
    );
    f.h.app
        .world_mut()
        .get_mut::<Health>(f.tank)
        .unwrap()
        .current = health.max * 0.30;
    pull_history::sample_all(f.h.app.world_mut());
    assert_eq!(
        f.read(&[f.second]).unwrap(),
        Decision::Wait(Reason::Reserve)
    );
}

#[test]
fn tank_pull_admission_native_casting_capability_is_not_replaced_by_an_in_progress_veto() {
    let mut f = measured_fixture();
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.healer, 8936, Some(f.tank), false)
        .unwrap();
    assert!(matches!(
        f.read(&[f.second]).unwrap(),
        Decision::Allow {
            bootstrap: false,
            ..
        }
    ));
    let cast = f.h.app.world().get::<CastState>(f.healer).unwrap();
    assert_eq!(
        (cast.spell_id, cast.target, cast.elapsed),
        (8936, f.tank.to_bits(), 0.0)
    );
}

#[test]
fn tank_pull_admission_native_stat_expiry_bounds_the_actual_measured_forecast() {
    let mut f = measured_fixture();
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.tank, 498, Some(f.tank), false)
        .unwrap();
    f.h.advance(3);
    pull_history::sample_all(f.h.app.world_mut());
    let estimate =
        crate::combat::forecast::read_generic_creature(f.h.app.world(), f.second, f.tank).unwrap();
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    let clear = estimate.remaining_health / history.group_dps.unwrap();
    let expiry = estimate
        .valid_until
        .expect("actual Divine Protection stat deadline");
    assert!(
        crate::auras::aura_now(f.h.app.world()) + clear > expiry,
        "native fixture really extends beyond mitigation credit"
    );
    assert!(
        f.read(&[f.second]).is_err(),
        "do not extend the current mitigation snapshot past its real expiry"
    );
}

#[test]
fn tank_pull_admission_native_pure_hot_lifetime_does_not_blanket_block() {
    let mut f = measured_fixture();
    crate::spell_cast::creature_cast(f.h.app.world_mut(), f.healer, 774, Some(f.tank), false)
        .unwrap();
    f.h.advance(3);
    pull_history::sample_all(f.h.app.world_mut());
    let estimate =
        crate::combat::forecast::read_generic_creature(f.h.app.world(), f.second, f.tank).unwrap();
    assert_eq!(
        estimate.valid_until, None,
        "actual full-kit Rejuvenation must remain a pure-HoT control"
    );
    assert!(matches!(
        f.read(&[f.second]).unwrap(),
        Decision::Allow {
            bootstrap: false,
            ..
        }
    ));
}

#[test]
fn tank_pull_admission_native_overlap_requires_complete_stable_admitted_health() {
    let mut f = measured_fixture();
    let engaged =
        crate::combat::forecast::read_generic_creature(f.h.app.world(), f.second, f.tank).unwrap();
    f.damage(f.second, engaged.remaining_health as f32 * 0.5);
    assert!(
        f.h.health(f.second) > 0.0,
        "actual progressing fight remains engaged, not a completed replacement fight"
    );
    let native = crate::test_data::world_data();
    let first_guid = f.first_guid;
    let second_guid =
        f.h.app
            .world()
            .get::<crate::zones::CreatureOrigin>(f.second)
            .unwrap()
            .guid;
    let third_guid = native
        .spawns
        .iter()
        .find(|spawn| {
            spawn.map == 34
                && spawn.template_entry == 46249
                && spawn.guid != first_guid
                && spawn.guid != second_guid
                && spawn.spawns_at(1)
        })
        .unwrap()
        .guid;
    let map = *f.h.app.world().get::<InMap>(f.tank).unwrap();
    let third = spawn_source(&mut f.h, third_guid, map);
    let decision = f.read(&[third]).unwrap();
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    let held =
        crate::combat::forecast::read_generic_creature(f.h.app.world(), f.second, f.tank).unwrap();
    let extra =
        crate::combat::forecast::read_generic_creature(f.h.app.world(), third, f.tank).unwrap();
    assert!(
        matches!(
            decision,
            Decision::Allow {
                bootstrap: false,
                ..
            }
        ),
        "native overlap decision={decision:?}, history={history:?}, tank={:?}, held={held:?}, extra={extra:?}",
        f.h.app.world().get::<Health>(f.tank)
    );
    let injured = f.members[2];
    let maximum = f.h.app.world().get::<Health>(injured).unwrap().max;
    f.h.app
        .world_mut()
        .get_mut::<Health>(injured)
        .unwrap()
        .current = maximum * 0.79;
    pull_history::sample_all(f.h.app.world_mut());
    assert_eq!(
        f.read(&[third]).unwrap(),
        Decision::Wait(Reason::HealthStability)
    );
}

#[test]
fn tank_pull_admission_native_scripted_source_has_no_automatic_numeric_budget() {
    let mut f = fixture();
    let map = *f.h.app.world().get::<InMap>(f.tank).unwrap();
    for entry in [1720, 46254] {
        let guid = crate::test_data::world_data()
            .spawns
            .iter()
            .find(|spawn| spawn.map == 34 && spawn.template_entry == entry && spawn.spawns_at(1))
            .unwrap()
            .guid;
        let actor = spawn_source(&mut f.h, guid, map);
        assert!(
            f.read(&[actor]).is_err(),
            "scripted entry{entry} requires an explicit pull decision"
        );
    }
}

#[test]
fn tank_pull_admission_native_roster_change_rejects_old_snapshot_before_next_tick() {
    let mut f = measured_fixture();
    let owner = f.h.caster;
    let profile =
        f.h.app
            .world()
            .get::<Follower>(f.members[2])
            .unwrap()
            .profile
            .clone();
    let map = *f.h.app.world().get::<InMap>(f.tank).unwrap();
    let replacement = crate::spell_cast::test_harness::spawn_npc(&mut f.h.app, 0.0);
    f.h.app.world_mut().entity_mut(replacement).insert((
        shared::components::Npc {
            template_id: profile.entry,
            name: profile.name.clone(),
        },
        shared::components::UnitPowers::default(),
        map,
    ));
    super::spawning::initialize(f.h.app.world_mut(), owner, replacement, profile).unwrap();
    f.members[2] = replacement;
    f.h.app
        .world_mut()
        .get_mut::<super::stockade_diagnostic::DiagnosticParty>(owner)
        .unwrap()
        .members = f.members.clone();
    assert!(
        pull_history::snapshot(f.h.app.world(), owner).is_none(),
        "new authoritative five-member roster must invalidate old calibration before next sampler tick"
    );
    assert!(f.read(&[f.second]).is_err());
    pull_history::sample_all(f.h.app.world_mut());
    assert!(matches!(
        f.read(&[f.second]).unwrap(),
        Decision::Allow {
            bootstrap: true,
            forecast_health: None
        }
    ));
}

#[test]
fn tank_pull_admission_native_new_copy_discards_completed_fight_and_rates() {
    let mut f = measured_fixture();
    let old = *f.h.app.world().get::<InMap>(f.tank).unwrap();
    let map = InMap {
        instance_id: old.instance_id + 1,
        ..old
    };
    let owner = f.h.caster;
    for unit in f.members.iter().copied().chain([owner]) {
        f.h.app.world_mut().entity_mut(unit).insert(map);
    }
    f.h.app
        .world_mut()
        .entity_mut(owner)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: f.members.clone(),
            player_role: shared::group::GroupRole::Dps,
        });
    let guid =
        f.h.app
            .world()
            .get::<crate::zones::CreatureOrigin>(f.second)
            .unwrap()
            .guid;
    let fresh = spawn_source(&mut f.h, guid, map);
    pull_history::sample_all(f.h.app.world_mut());
    assert!(matches!(
        f.read(&[fresh]).unwrap(),
        Decision::Allow {
            bootstrap: true,
            forecast_health: None
        }
    ));
}

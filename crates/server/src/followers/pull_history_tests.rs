//! Native follower initialization, health effects and event/update ordering.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use super::{pull_history, update_ai};
use crate::maps::InMap;
use crate::spell_cast::effects::{EffectCtx, apply_heal, deal_damage};
use crate::spell_cast::test_harness::Harness;
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::Health;

const COPY: InMap = InMap {
    map_id: 34,
    instance_id: 71,
};

fn fixture() -> (Harness, Entity) {
    let (mut h, tank, _) = spawn_profile_fixture_with_plugins(209057, |_| {});
    h.advance(3);
    for unit in [h.caster, tank, h.npc] {
        h.app.world_mut().entity_mut(unit).insert(COPY);
    }
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: COPY,
            members: vec![tank],
            player_role: shared::group::GroupRole::Dps,
        });
    let spawn = crate::test_data::world_data()
        .spawns
        .iter()
        .find(|spawn| spawn.guid == 20375682)
        .expect("authored Riverpaw spawn");
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::zones::CreatureOrigin {
            guid: spawn.guid,
            original_entry: spawn.template_entry,
        });
    update_ai(h.app.world_mut());
    (h, tank)
}

fn advance_samples(h: &mut Harness, frames: usize) {
    for _ in 0..frames {
        h.advance(1);
        update_ai(h.app.world_mut());
    }
}

fn damage(h: &mut Harness, source: Entity, target: Entity, amount: f32) -> f32 {
    let before = h.health(target);
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let context = EffectCtx {
        caster: source,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(h.app.world_mut(), &context, target, amount, false, false);
    before - h.health(target)
}

fn heal(h: &mut Harness, source: Entity, target: Entity, amount: f32) -> f32 {
    let before = h.health(target);
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(2061)
        .unwrap()
        .clone();
    let context = EffectCtx {
        caster: source,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    apply_heal(h.app.world_mut(), &context, target, amount, false, false);
    h.health(target) - before
}

#[test]
fn tank_pull_history_measures_effective_native_damage_and_heal_over_ten_seconds() {
    let (mut h, tank) = fixture();
    advance_samples(&mut h, 180);
    let owner = h.caster;
    let enemy = h.npc;
    let dealt = damage(&mut h, owner, enemy, 100.0);
    h.app.world_mut().get_mut::<Health>(tank).unwrap().current -= 50.0;
    let healed = heal(&mut h, owner, tank, 1000.0);
    assert!(dealt > 0.0);
    assert_eq!(
        healed, 50.0,
        "native overheal does not create effective capacity"
    );
    let short = pull_history::snapshot(h.app.world(), owner).expect("follower history");
    assert_eq!(
        short.group_dps, None,
        "nine seconds is not a complete window"
    );
    advance_samples(&mut h, 22);
    let measured = pull_history::snapshot(h.app.world(), owner).unwrap();
    assert_eq!(measured.group_dps, Some(f64::from(dealt) / 10.0));
    assert_eq!(measured.healer_hps, Some(f64::from(healed) / 10.0));
}

#[test]
fn tank_pull_history_requires_every_members_complete_five_second_health_history() {
    let (mut h, tank) = fixture();
    advance_samples(&mut h, 90);
    let owner = h.caster;
    assert!(
        !pull_history::snapshot(h.app.world(), owner)
            .unwrap()
            .party_stable
    );
    advance_samples(&mut h, 12);
    assert!(
        pull_history::snapshot(h.app.world(), owner)
            .unwrap()
            .party_stable
    );
    {
        let mut health = h.app.world_mut().get_mut::<Health>(tank).unwrap();
        health.current = health.max * 0.79;
    }
    advance_samples(&mut h, 1);
    {
        let mut health = h.app.world_mut().get_mut::<Health>(tank).unwrap();
        health.current = health.max;
    }
    advance_samples(&mut h, 90);
    assert!(
        !pull_history::snapshot(h.app.world(), owner)
            .unwrap()
            .party_stable
    );
    advance_samples(&mut h, 12);
    assert!(
        pull_history::snapshot(h.app.world(), owner)
            .unwrap()
            .party_stable
    );
    h.app.world_mut().entity_mut(tank).remove::<Health>();
    advance_samples(&mut h, 1);
    assert!(
        !pull_history::snapshot(h.app.world(), owner)
            .unwrap()
            .party_stable
    );
}

#[test]
fn tank_pull_history_clears_only_observed_spawn_death_not_same_template_or_disappearance() {
    let (mut h, _tank) = fixture();
    let owner = h.caster;
    let enemy = h.npc;
    let origin = *h
        .app
        .world()
        .get::<crate::zones::CreatureOrigin>(enemy)
        .unwrap();
    let other = crate::test_data::world_data()
        .spawns
        .iter()
        .find(|spawn| spawn.template_entry == origin.original_entry && spawn.guid != origin.guid)
        .expect("second real spawn of the same template");
    let unseen = h
        .app
        .world_mut()
        .spawn((
            COPY,
            crate::zones::CreatureOrigin {
                guid: other.guid,
                original_entry: origin.original_entry,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    h.app.world_mut().despawn(unseen);
    let before = pull_history::snapshot(h.app.world(), owner).unwrap();
    assert!(
        before.cleared_spawns.is_empty(),
        "disappearance is not death"
    );
    h.app.world_mut().get_mut::<Health>(enemy).unwrap().current = 1.0;
    damage(&mut h, owner, enemy, 1000.0);
    advance_samples(&mut h, 1);
    let after = pull_history::snapshot(h.app.world(), owner).unwrap();
    assert_eq!(
        after.cleared_spawns.into_iter().collect::<Vec<_>>(),
        vec![origin.guid]
    );
    assert_eq!(after.completed_fights, 1);
    advance_samples(&mut h, 2);
    assert_eq!(
        pull_history::snapshot(h.app.world(), owner)
            .unwrap()
            .completed_fights,
        1
    );
}

#[test]
fn tank_pull_history_new_copy_does_not_reuse_previous_run_measurements_or_deaths() {
    let (mut h, tank) = fixture();
    advance_samples(&mut h, 202);
    let owner = h.caster;
    let enemy = h.npc;
    damage(&mut h, owner, enemy, 100000.0);
    advance_samples(&mut h, 1);
    assert_eq!(
        pull_history::snapshot(h.app.world(), owner)
            .unwrap()
            .completed_fights,
        1
    );
    let next = InMap {
        instance_id: COPY.instance_id + 1,
        ..COPY
    };
    for unit in [owner, tank] {
        h.app.world_mut().entity_mut(unit).insert(next);
    }
    h.app
        .world_mut()
        .get_mut::<super::stockade_diagnostic::DiagnosticParty>(owner)
        .unwrap()
        .copy = next;
    advance_samples(&mut h, 1);
    let fresh = pull_history::snapshot(h.app.world(), owner).unwrap();
    assert_eq!(fresh.completed_fights, 0);
    assert_eq!(fresh.group_dps, None);
    assert_eq!(fresh.healer_hps, None);
    assert!(!fresh.party_stable);
    assert!(fresh.cleared_spawns.is_empty());
}

//! Player empower timing and real Preservation release effects.
use super::dps_parity::literal_rotation::{
    Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use bevy::prelude::*;
use shared::components::{Health, UnitFactionTemplate};
use shared::protocol::CombatLogKind;

fn release(spell_id: u32, rank: u8) -> (u32, Vec<i32>, Vec<u32>) {
    let (profile, traits) = super::dps_parity::read_reference(1468).unwrap();
    let mut run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 70 },
    )
    .unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let actor = run.actor;
    let faction = *run.app.world().get::<UnitFactionTemplate>(actor).unwrap();
    let copy = *run.app.world().get::<crate::maps::InMap>(actor).unwrap();
    let mut allies = Vec::new();
    for x in [1.0, 2.0, 3.0] {
        let ally = crate::spell_cast::test_harness::spawn_player(&mut run.app, 13, x);
        run.app.world_mut().entity_mut(ally).insert((
            faction,
            copy,
            Health {
                current: 100_000.0,
                max: 1_000_000.0,
            },
        ));
        allies.push(ally);
    }
    run.app.world_mut().entity_mut(actor).insert(Health {
        current: 100_000.0,
        max: 1_000_000.0,
    });
    if spell_id == 367226 {
        // This profile does not select Spiritbloom; install that actual spell in this fixture's book.
        run.app
            .world_mut()
            .entity_mut(actor)
            .insert(crate::class_progression::KnownSpells::fixture(vec![367226]));
    }
    let target = if spell_id == 355936 { actor } else { allies[0] };
    let targets = [target];
    let mut bindings = Bindings {
        actions: [("empower".into(), spell_id)].into(),
        global_cooldown_spell: spell_id,
        ..Default::default()
    };
    let equipment = Equipment::load(run.app.world_mut(), actor, &mut bindings).unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        actor,
        &targets,
        &equipment,
        [NativeAction {
            name: "empower",
            spell: spell_id,
            self_target: spell_id == 355936,
            talent: None,
        }],
        false,
    )
    .unwrap();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let facts = NativeFacts::new(run.app.world(), actor, &bindings, encounter, 1)
        .with_targets(Some(target), &targets);
    let row = format!("empower,empower_to={rank}");
    let mut driver =
        Driver::new(server::simc_apl::Program::parse(&[("default", row.as_str())]).unwrap());
    let (action, execution) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .unwrap();
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    driver
        .execute(run.app.world_mut(), actor, action, execution)
        .unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(actor)
            .is_some()
    );
    let mut ticks = 0;
    while run
        .app
        .world()
        .get::<shared::casting::CastState>(actor)
        .is_some()
        && ticks < 150
    {
        run.app.update();
        ticks += 1;
    }
    assert!(ticks < 150, "empower must complete");
    for _ in 0..10 {
        run.app.update();
    }
    let child = if spell_id == 355936 { 355941 } else { 367230 };
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    let amounts = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(child)
                && event.source == Some(actor.to_bits())
        })
        .map(|event| event.amount)
        .collect();
    let durations = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .into_iter()
        .flat_map(|auras| auras.iter())
        .filter(|aura| aura.spell_id == child && aura.caster == actor)
        .map(|aura| aura.duration_ms)
        .collect();
    (ticks, amounts, durations)
}

#[test]
#[ignore = "requires native Preservation MID2 inputs and canonical DB"]
fn player_empowers_release_rank_specific_healing() {
    let (dream_one_time, dream_one, one_duration) = release(355936, 1);
    let (dream_three_time, dream_three, three_duration) = release(355936, 3);
    assert!(dream_three_time > dream_one_time);
    assert!(!dream_one.is_empty() && !dream_three.is_empty());
    assert!(dream_three.iter().max().unwrap() > dream_one.iter().max().unwrap());
    assert!(one_duration[0] > three_duration[0]);
    let (one_time, one, _) = release(367226, 1);
    let (three_time, three, _) = release(367226, 3);
    assert!(three_time > one_time);
    assert_eq!(one.len(), 1);
    assert_eq!(three.len(), 3);
    assert!(three.iter().all(|amount| *amount > 0));
}

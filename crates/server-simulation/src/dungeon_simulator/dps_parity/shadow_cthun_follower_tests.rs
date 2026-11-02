use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_shadow_cthun_summons_channeling_guardians_and_earns_insanity() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    for (count, selected) in [(1, true), (3, true), (1, false)] {
        let _seed = crate::random::seed_scope(70448);
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2515).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 70);
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::followers::Follower>();
        for _ in 0..4 {
            app.update();
        }
        assert!(crate::auras::has_aura(app.world(), actor, 377349));
        if !selected {
            crate::auras::remove_auras(
                app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 377349 && aura.caster == actor,
            );
        }
        crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
        for i in 1..count {
            let other = spawn_level_dummy(&mut app, actor, 70, i).unwrap();
            crate::combat_refs::set_in_combat_with(app.world_mut(), actor, other);
        }
        app.world_mut()
            .entity_mut(actor)
            .insert(crate::networking::CombatTarget(Some(target)));
        let mut observed = Vec::new();
        for _ in 0..32 {
            crate::spell_cast::powers::change_power(
                app.world_mut(),
                actor,
                PowerType::Insanity,
                -10000.0,
            );
            crate::spell_cast::creature_cast(app.world_mut(), actor, 15407, Some(target), false)
                .unwrap();
            for _ in 0..100 {
                app.update();
            }
            let mut query = app.world_mut().query::<(
                Entity,
                &crate::spell_cast::summon::SummonedCreature,
                &shared::components::Npc,
            )>();
            let pets: Vec<_> = query
                .iter(app.world())
                .filter(|(_, summon, _)| {
                    summon.owner == actor && matches!(summon.spell_id, 377355 | 377357)
                })
                .map(|(pet, summon, npc)| {
                    (
                        pet,
                        summon.spell_id,
                        npc.template_id,
                        app.world()
                            .get::<shared::casting::CastState>(pet)
                            .map(|cast| cast.spell_id),
                    )
                })
                .collect();
            if !pets.is_empty() {
                observed.push(pets);
            }
        }
        println!("CTHUN_PETS {observed:?}");
        let timers = app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 377349)
            .map(crate::auras::procs::snapshot_proc_timers);
        println!("CTHUN_TIMERS selected={selected} targets={count} {timers:?}");
        let events = &app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let mut counts = std::collections::BTreeMap::new();
        for event in events {
            if let Some(spell) = event.spell_id {
                *counts
                    .entry((spell, format!("{:?}", event.kind)))
                    .or_insert(0usize) += 1;
            }
        }
        println!("CTHUN_EVENTS selected={selected} targets={count} {counts:?}");
        let tendril = events.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(193473)
                && event.amount > 0
        });
        let lasher = events.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(394979)
                && event.amount > 0
        });
        let gains: Vec<_> = events
            .iter()
            .filter(|event| {
                event.target == Some(actor.to_bits())
                    && event.kind == CombatLogKind::Energize
                    && event.spell_id == Some(377358)
                    && event.amount > 0
            })
            .map(|event| event.amount)
            .collect();
        assert_eq!(
            tendril,
            selected && count == 1,
            "single-target C'Thun must summon a damaging Tendril"
        );
        assert_eq!(
            lasher,
            selected && count == 3,
            "three-target C'Thun must summon a damaging Lasher"
        );
        assert_eq!(
            !gains.is_empty(),
            selected,
            "actual guardian ticks grant Insanity only with C'Thun"
        );
        assert!(
            gains.iter().all(|&amount| amount == 100),
            "each pet tick grants one Insanity: {gains:?}"
        );
    }
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_shadow_cthun_direct_summon_diagnostic() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, _, _) = prepare_class_follower(app, copy, 258, 70);
    for _ in 0..4 {
        app.update();
    }
    println!(
        "CTHUN_TEMPLATE {}",
        app.world()
            .resource::<crate::world_data::WorldData>()
            .templates
            .contains_key(&192337)
    );
    println!(
        "CTHUN_PROPS {:?}",
        app.world()
            .resource::<crate::spell_cast::summon::SummonPropertiesStore>()
            .0
            .get(&5395)
    );
    let mut spell = crate::spell_cast::load_triggered_spell(app.world(), 377355, 1).unwrap();
    spell.cast_destination = Some(
        crate::spell_cast::targets::position(app.world(), actor)
            .unwrap()
            .to_array(),
    );
    let launched =
        crate::spell_cast::launch_triggered_spell(app.world_mut(), actor, &spell, Some(actor), 1);
    for _ in 0..10 {
        app.update();
    }
    let mut query = app.world_mut().query::<(
        Entity,
        &crate::spell_cast::summon::SummonedCreature,
        &shared::components::Npc,
    )>();
    let pets: Vec<_> = query
        .iter(app.world())
        .filter(|(_, summon, _)| summon.owner == actor)
        .map(|(_, summon, npc)| (summon.spell_id, npc.template_id))
        .collect();
    println!("CTHUN_DIRECT launched={launched} pets={pets:?}");
    assert!(
        pets.iter()
            .any(|&(spell, npc)| spell == 377355 && npc == 192337)
    );
}

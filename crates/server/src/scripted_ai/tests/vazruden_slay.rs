//! Native Vazruden lethal-attribution qualification, before any slay hook.
use super::herald_call::first_west;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;

fn apply_native_low_level_stats(h: &mut Harness) {
    use bevy::ecs::system::RunSystemOnce;
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert((
        shared::components::Level(10),
        shared::components::UnitLevel(10),
    ));
    let source = crate::spell_cast::test_harness::world_db().unwrap();
    h.app
        .insert_resource(crate::spell_cast::test_harness::class_spells(source));
    h.app
        .world_mut()
        .run_system_once(crate::stats::update_character_stats)
        .unwrap();
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(player)
        .unwrap();
    assert_eq!(h.health(player).max, stats.max_health);
    assert!(h.health(player).current > 0.0);
    println!(
        "native Human warrior level10 production stat path: currentHP{} maxHP{} stamina{}",
        h.health(player).current,
        stats.max_health,
        stats.primary.stamina
    );
}

#[test]
fn encounter_coverage_vazruden_slay_ordinary_lethal_qualification() {
    let (mut h, map, _, vazruden) = first_west(2);
    h.app.init_resource::<ObservedCombatEvents>();
    super::omor_slay::install_player_death_transition(&mut h);
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&17537][&2];
    assert_eq!(texts.len(), 2);
    assert_eq!(texts.iter().map(|text| text.id).collect::<Vec<_>>(), [0, 1]);
    assert!(texts.iter().all(|text| text.chat_type == 14
        && text.text_range == 0
        && text.probability == 100.0
        && !text.male_text.is_empty()));
    println!("native17537 sourceGroup2: {texts:?}");
    let position = *h.app.world().get::<Position>(vazruden).unwrap();
    let player = h.player();
    // Level/class inputs go through the real stat producer; no Health assignment.
    apply_native_low_level_stats(&mut h);
    // Pose-only harness setup at the real grounded summon.
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    let before = h.health(player).current;
    for _ in 0..3600 {
        h.advance(0.05);
        if h.health(player).current <= 0.0 {
            break;
        }
    }
    let after = h.health(player).current;
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    let damage: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(vazruden.to_bits())
                && event.target == Some(player.to_bits())
                && event.amount > 0
        })
        .collect();
    let deaths: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Death && event.target == Some(player.to_bits())
        })
        .collect();
    println!(
        "ordinary17537 native level10 initialHP{before} remainingHP{after}, damage_count{}; actual deaths:{deaths:?}",
        damage.len()
    );
    assert!(
        !damage.is_empty(),
        "ordinary native17537 must really deal damage"
    );
    assert_eq!(
        after, 0.0,
        "VAZRUDEN_ORDINARY_LETHAL_ATTRIBUTION_NOT_QUALIFIED_WITHIN_180S: no HP/stat writes or forced casts/hits"
    );
    assert!(
        deaths
            .iter()
            .any(|event| event.source == Some(vazruden.to_bits())),
        "VAZRUDEN_ORDINARY_LETHAL_ATTRIBUTION_NOT_QUALIFIED: actual lethal source must be17537, not Nazan or a field"
    );
    let death = h
        .app
        .world()
        .get::<crate::death::PlayerDeathState>(player)
        .expect("production death transition");
    assert_eq!(death.state, shared::death::DeathState::Dead);
    assert!(death.corpse.is_some());
    assert_eq!(h.app.world().get::<InMap>(vazruden), Some(&map));
}

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

fn ordinary_lethal_fixture() -> (Harness, InMap, Entity, Entity) {
    let (mut h, map, nazan, vazruden) = first_west(2);
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
    (h, map, nazan, vazruden)
}

fn slay_count(h: &Harness, actor: Entity, listener: usize) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&17537][&2];
    h.inbox(listener)
        .chat
        .iter()
        .filter(|line| {
            line.channel == shared::protocol::ChatType::MonsterYell(actor.to_bits())
                && texts
                    .iter()
                    .any(|text| line.content == text.male_text || line.content == text.female_text)
        })
        .count()
}

#[test]
#[ignore = "unfiltered source17537 NPC victim and real corpse replay control"]
fn encounter_coverage_vazruden_slay_npc_victim_speaks_once() {
    let (mut h, _, nazan, vazruden, _, _) = super::herald_ground::reach_fall(2);
    h.advance(2.0);
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::hellfire_ramparts::HeraldUnkillable>(nazan)
            .is_none()
    );
    // HARNESS SETUP: real production Unit::Kill, not authored boss rotation,
    // a fabricated KilledUnit hook, a forced cast/hit or an HP assignment.
    crate::combat::kill_unit(h.app.world_mut(), vazruden, nazan);
    h.advance(0.05);
    assert!(h.app.world().get::<crate::combat::Corpse>(nazan).is_some());
    assert_eq!(
        slay_count(&h, vazruden, 0),
        1,
        "source KilledUnit has no player-only filter"
    );
    assert_eq!(slay_count(&h, vazruden, 1), 0);
    // HARNESS SETUP: repeat Unit::Kill on the actual corpse, never replay a hook.
    crate::combat::kill_unit(h.app.world_mut(), vazruden, nazan);
    h.advance(0.1);
    assert_eq!(slay_count(&h, vazruden, 0), 1);
}

#[test]
#[ignore = "native nonlethal damage then another real killer cannot award17537 slay"]
fn encounter_coverage_vazruden_slay_nonlethal_then_other_killer_stays_silent() {
    let (mut h, _, nazan, vazruden) = first_west(1);
    h.app.init_resource::<ObservedCombatEvents>();
    super::omor_slay::install_player_death_transition(&mut h);
    apply_native_low_level_stats(&mut h);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(vazruden).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 3.0,
        ..origin
    });
    let before = h.health(player).current;
    let mut damaged = false;
    for _ in 0..200 {
        h.advance(0.05);
        damaged = h
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(vazruden.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        if damaged {
            break;
        }
    }
    assert!(
        damaged,
        "ordinary Normal17537 actual damage must precede the attribution control"
    );
    assert!(
        h.health(player).current > 0.0 && h.health(player).current < before,
        "native nonlethal hit, without HP writes"
    );
    assert_eq!(slay_count(&h, vazruden, 0), 0);
    // HARNESS SETUP: a different actual creature kills through production Unit::Kill.
    // Prior17537 damage must not replace the explicit fatal source with last-damager inference.
    crate::combat::kill_unit(h.app.world_mut(), nazan, player);
    h.advance(0.05);
    assert_eq!(h.health(player).current, 0.0);
    assert!(
        h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(nazan.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.kind == CombatLogKind::Death
            })
    );
    assert_eq!(slay_count(&h, vazruden, 0), 0);
}

#[test]
#[ignore = "shared production Unit::Kill must notify actual owned killer, not reward owner"]
fn encounter_coverage_vazruden_slay_shared_owned_killer_does_not_notify_owner() {
    // Shared attribution control uses actual Omor30707 -> owned17540, not
    // an invented minion/owner component on Vazruden, who has no such source summon.
    let (mut h, owner, hound, _) = super::omor_slay::ordinary_hound_fixture();
    h.app.init_resource::<ObservedCombatEvents>();
    apply_native_low_level_stats(&mut h);
    let player = h.player();
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitSummonedBy>(hound)
            .unwrap()
            .0,
        owner.to_bits()
    );
    // HARNESS SETUP: real Unit::Kill from the actual owned minion; no HP write,
    // fabricated KilledUnit callback or forced spell/hit outcome.
    crate::combat::kill_unit(h.app.world_mut(), hound, player);
    h.advance(0.05);
    assert!(
        h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(hound.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.kind == CombatLogKind::Death
            })
    );
    assert_eq!(
        super::omor_slay::slay_count(&h, owner),
        0,
        "reward ownership does not redirect actual killer AI speech"
    );
}

#[test]
fn encounter_coverage_vazruden_slay_ordinary_lethal_qualification() {
    let _ = ordinary_lethal_fixture();
}

#[test]
#[ignore = "source17537 actual ordinary native player kill must emit missing Talk2"]
fn encounter_coverage_vazruden_slay_ordinary_player_death_speaks_once() {
    let (mut h, _, nazan, vazruden) = ordinary_lethal_fixture();
    assert_eq!(
        h.app
            .world()
            .get::<crate::spell_cast::summon::SummonedCreature>(vazruden)
            .unwrap()
            .owner,
        nazan,
        "actual30717 summon ownership is preserved; the killing actor, not its summoner, speaks"
    );
    assert_eq!(slay_count(&h, nazan, 0), 0);
    assert_eq!(
        slay_count(&h, vazruden, 0),
        1,
        "real native17537 killing blow emits exactly one sourceGroup2"
    );
    assert_eq!(
        slay_count(&h, vazruden, 1),
        0,
        "other copy cannot hear the source slay line"
    );
    h.advance(1.0);
    assert_eq!(
        slay_count(&h, vazruden, 0),
        1,
        "ordinary corpse/evade ticks cannot replay slay"
    );
}

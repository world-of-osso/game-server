//! Bounded local Mainline contracts; real casts/periods, fixed hit/crit fixtures.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Position, PowerType};
use shared::protocol::CombatLogKind;

fn execute(h: &mut Harness, id: u32, target: Option<Entity>) {
    let spell = world_db().unwrap().store.get(id).unwrap();
    super::execute_cast(h.app.world_mut(), h.caster, spell, target).unwrap();
}
fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
fn cast_at(h: &mut Harness, id: u32, at: [f32; 3]) {
    let mut spell = world_db().unwrap().store.get(id).unwrap().clone();
    spell.cast_destination = Some(at);
    super::execute_cast(h.app.world_mut(), h.caster, &spell, None).unwrap();
}

#[test]
fn dummy14_frozen_orb_hit_applies_owned_chilled_and_expires() {
    let mut h = harness(Vec::new(), &[84721, 205708], 8).unwrap();
    let target = h.npc;
    cast_at(&mut h, 84721, [3.0, 0.0, 0.0]);
    assert!(crate::auras::has_aura(h.app.world(), target, 205708));
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(target)
            .unwrap()
            .movement_speed,
        0.5
    );
    h.advance(170);
    assert!(!crate::auras::has_aura(h.app.world(), target, 205708));
}

#[test]
fn dummy14_blizzard_hit_uses_its_own_slow_not_orb_duration() {
    let mut h = harness(Vec::new(), &[190357, 12486], 8).unwrap();
    let target = h.npc;
    cast_at(&mut h, 190357, [3.0, 0.0, 0.0]);
    assert!(crate::auras::has_aura(h.app.world(), target, 12486));
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(target)
            .unwrap()
            .movement_speed,
        0.5
    );
    h.advance(65);
    assert!(!crate::auras::has_aura(h.app.world(), target, 12486));
}

fn pet(h: &mut Harness) -> Entity {
    let owner = h.caster;
    let pet = spawn_player(&mut h.app, 3, 2.0);
    h.app
        .world_mut()
        .entity_mut(pet)
        .remove::<shared::components::Player>()
        .insert(crate::hunter_pet::HunterPet { owner, slot: 0 });
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::hunter_pet::PetStable {
            character_id: Some(1),
            stored: default(),
            summoned: Some(pet),
        });
    pet
}
#[test]
fn dummy14_survival_kill_command_routes_named_damage_from_owned_pet() {
    // Named child damage must hit; avoidance has separate native tests.
    let mut h = harness(Vec::new(), &[259489, 259277, SUNWELL_RADIANCE], 3).unwrap();
    let pet = pet(&mut h);
    apply_native_enemy_dodge_reduction_to(&mut h, pet);
    let target = h.npc;
    execute(&mut h, 259489, Some(target));
    h.advance(1);
    assert_eq!(
        h.health(target),
        4934.0,
        "3AP from actual pet, native22AP inherited from fixture owner; outcomes {:?}",
        h.inbox.log
    );
    let hits: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(259277) && e.kind == CombatLogKind::Damage)
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].source, Some(pet.to_bits()));
}
#[test]
fn dummy14_kill_commands_never_borrow_foreign_pets() {
    for (command, damage) in [(259489, 259277), (34026, 83381)] {
        let mut h = harness(Vec::new(), &[command, damage], 3).unwrap();
        let p = pet(&mut h);
        let foreign = spawn_player(&mut h.app, 3, 0.0);
        h.app
            .world_mut()
            .get_mut::<crate::hunter_pet::HunterPet>(p)
            .unwrap()
            .owner = foreign;
        let target = h.npc;
        execute(&mut h, command, Some(target));
        assert_eq!(h.health(target), 5000.0);
    }
}

#[test]
fn dummy14_wildfire_primary_named_hit_bonus_and_owned_bleed() {
    let mut h = harness(Vec::new(), &[259495, 265163, 265157, 269747], 3).unwrap();
    let target = h.npc;
    let other = spawn_npc(&mut h.app, 4.0);
    // Unequal AP/RAP proves Hunter damage uses its separate ranged field.
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .attack_power = 37.0;
    // The master failure occurred near a variance endpoint. Exercise
    // bounded real casts, cancelling previous bleeds so primary damage is isolated.
    for attempt in 0..256 {
        crate::auras::remove_auras(
            h.app.world_mut(),
            target,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 269747,
        );
        h.app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap()
            .current = 5000.0;
        h.set_power(PowerType::Focus, 100.0);
        h.inbox = Inbox::default();
        execute(&mut h, 259495, Some(target));
        h.advance(3); // Native root35yd/sec missile must reach3yd target.
        assert!(
            // Active265157:1.9RAP;259495s3:50% primary bonus;5% variance.
            varied(1.9 * 100.0 * 1.5).contains(&(5000.0 - h.health(target))),
            "attempt{attempt} primary damage{}, caster stats{:?}, native log{:?}",
            5000.0 - h.health(target),
            h.app.world().get::<crate::stats::CharacterStats>(h.caster),
            h.inbox.log,
        );
    }
    assert!(crate::auras::has_aura(h.app.world(), target, 269747));
    assert_eq!(
        h.health(other),
        5000.0,
        "cone unimplemented, no guessed splash"
    );
    h.advance(125);
    let ticks: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(269747) && e.kind == CombatLogKind::Damage)
        .collect();
    assert_eq!(ticks.len(), 6);
    assert!(
        // Active Effect718207 snapshots fixture RAP100, not melee AP37:
        // 0.252 × 100, rounded to25 each native tick.
        ticks.iter().all(|e| e.amount == 25),
        "{ticks:?}"
    );
    assert!(!crate::auras::has_aura(h.app.world(), target, 269747));
}

#[test]
fn dummy14_deathmark_restores_native_total_energy_on_bleed_clock() {
    for active in [false, true] {
        let mut h = harness(Vec::new(), &[360194], 4).unwrap();
        let caster = h.caster;
        let target = h.npc;
        if active {
            apply(&mut h, caster, target, 360194);
        }
        h.set_power(PowerType::Energy, 0.0);
        h.advance(41);
        assert_eq!(h.power(PowerType::Energy), if active { 30.5 } else { 20.5 });
    }
}

#[test]
fn dummy14_eye_beam_native_initial_plus_ten_primary_hits_with_ranked_guaranteed_crit() {
    for rank in [false, true] {
        let mut h = harness(Vec::new(), &[198013, 198030, 320415], 12).unwrap();
        let caster = h.caster;
        h.app
            .world_mut()
            .entity_mut(caster)
            .insert(crate::class_progression::KnownSpells::fixture(vec![198013]));
        if rank {
            apply(&mut h, caster, caster, 320415);
        }
        h.set_power(PowerType::Fury, 100.0);
        let target = h.npc;
        h.cast_and_deliver(198013, Some(target));
        h.advance(45);
        let hits: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|e| {
                e.spell_id == Some(198030)
                    && e.target == Some(target.to_bits())
                    && e.kind == CombatLogKind::Damage
            })
            .collect();
        assert_eq!(
            hits.len(),
            11,
            "native EXTRA_INITIAL_PERIOD plus ten200ms ticks: {hits:?}"
        );
        assert!(hits.iter().all(|e| e.crit == rank));
        assert!(
            hits.iter()
                .all(|e| varied(40.26 * if rank { 2.0 } else { 1.0 }).contains(&(e.amount as f32)))
        );
    }
}

#[test]
fn dummy14_metamorphosis_landing_hits_at_destination_and_splits_player_cc() {
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[191427, 200166, 247121, 162264, 201453],
        12,
    )
    .unwrap();
    let target = h.npc;
    h.app.world_mut().get_mut::<Position>(target).unwrap().x = 20.0;
    let hostile = spawn_npc(&mut h.app, 27.9);
    let far = spawn_npc(&mut h.app, 28.1);
    let player = spawn_player(&mut h.app, 1, 23.0);
    // Player-v-player hostility requires an actual duel, not an NPC template.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::duel::DuelOpponent(player));
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::duel::DuelOpponent(h.caster));
    cast_at(&mut h, 191427, [20.0, 0.0, 0.0]);
    assert!(varied(44.0).contains(&(5000.0 - h.health(target))));
    assert!(varied(44.0).contains(&(5000.0 - h.health(hostile))));
    assert_eq!(h.health(far), 5000.0);
    assert!(crate::auras::has_aura(h.app.world(), target, 200166));
    assert!(crate::auras::has_aura(h.app.world(), player, 247121));
    assert!(!crate::auras::has_aura(h.app.world(), player, 200166));
}

#[test]
fn dummy14_consume_generates_named_fury_once_without_optional_fragments() {
    let mut h = harness(Vec::new(), &[473662, 1261710], 12).unwrap();
    h.set_power(PowerType::Fury, 0.0);
    let target = h.npc;
    execute(&mut h, 473662, Some(target));
    assert_eq!(h.power(PowerType::Fury), 8.0);
    // Active SpellEffect1192725: 3.67 SP × fixture SP100.
    assert!(varied(367.0).contains(&(5000.0 - h.health(target))));
}

#[test]
fn dummy14_eye_beam_cancel_stops_future_primary_hits() {
    let mut h = harness(Vec::new(), &[198013, 198030], 12).unwrap();
    let target = h.npc;
    h.set_power(PowerType::Fury, 100.0);
    h.cast_and_deliver(198013, Some(target));
    h.advance(14);
    super::cancel_cast(h.app.world_mut(), h.caster);
    h.advance(1);
    let hits = |h: &Harness| {
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(198030) && e.kind == CombatLogKind::Damage)
            .count()
    };
    let count = hits(&h);
    assert!((1..11).contains(&count));
    h.advance(60);
    assert_eq!(hits(&h), count);
}
#[test]
fn dummy14_deathmark_eight_native_gains_total_eighty_then_stop() {
    let mut h = harness(Vec::new(), &[360194], 4).unwrap();
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, caster, target, 360194);
    let mut gained = 0.0;
    for _ in 0..8 {
        h.set_power(PowerType::Energy, 0.0);
        h.advance(40);
        gained += h.power(PowerType::Energy) - 20.0; // Native10Energy/sec regeneration.
    }
    assert_eq!(gained, 80.0);
    assert!(!crate::auras::has_aura(h.app.world(), target, 360194));
    h.set_power(PowerType::Energy, 0.0);
    h.advance(40);
    assert_eq!(h.power(PowerType::Energy), 20.0);
}
#[test]
fn dummy14_consume_immune_target_does_not_generate_fury() {
    let mut h = harness(Vec::new(), &[473662, 1261710, 642], 12).unwrap();
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, caster, target, 642);
    h.set_power(PowerType::Fury, 0.0);
    execute(&mut h, 473662, Some(target));
    assert_eq!(h.power(PowerType::Fury), 0.0);
    assert_eq!(h.health(target), 5000.0);
}

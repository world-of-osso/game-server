//! TC a352b1fa Unit.cpp6163/10174, Object.cpp1725; unedited 70448 native inputs.
use super::test_harness::{Harness, harness, world_db};
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::components::PowerType;

fn fixture(class: u8, spells: &[u32]) -> Harness {
    let mut h = harness(Vec::new(), spells, class).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h
}

fn apply(h: &mut Harness, id: u32, mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

fn remove(h: &mut Harness, id: u32) {
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == id,
    );
}

fn stacks(h: &Harness, id: u32) -> u8 {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == id)
                .map(|aura| aura.stacks)
        })
        .unwrap_or(0)
}

fn hostile_duration(h: &mut Harness, id: u32) -> u32 {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.npc,
            target: h.caster,
            spell_id: id,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == id)
        .unwrap()
        .duration_ms
}

#[test]
#[ignore = "RED: generic245 dispel duration consumer"]
fn census_three_245_native_magic_reduction_and_removal() {
    let mut h = fixture(8, &[205708, 235450]);
    let baseline = hostile_duration(&mut h, 205708);
    remove(&mut h, 205708);
    apply(&mut h, 235450, 1 << 3); // Blazing Barrier's native -25% magic duration.
    assert_eq!(hostile_duration(&mut h, 205708), baseline * 75 / 100);
    remove(&mut h, 205708);
    remove(&mut h, 235450);
    assert_eq!(hostile_duration(&mut h, 205708), baseline);
}

#[test]
#[ignore = "RED: generic245 additive matching dispel modifiers"]
fn census_three_245_native_modifiers_sum_without_changing_positive_duration() {
    let mut h = fixture(8, &[205708, 235450, 454842, 774]);
    let baseline = hostile_duration(&mut h, 205708);
    remove(&mut h, 205708);
    apply(&mut h, 235450, 1 << 3);
    apply(&mut h, 454842, 1 << 1); // Native -35%, same Magic dispel selector.
    assert_eq!(hostile_duration(&mut h, 205708), baseline * 40 / 100);
    apply(&mut h, 774, 1);
    let positive = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 774)
        .unwrap();
    let duration = world_db()
        .unwrap()
        .store
        .get(774)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .unwrap()
        .duration_ms;
    assert_eq!(positive.duration_ms, duration as u32);
}

#[test]
#[ignore = "RED:262 existing masked state consumer lacks support recognition"]
fn census_three_262_native_masked_state_permission_has_no_unknown_warning() {
    let mut h = fixture(2, &[31884, 24275, 163201]);
    let hammer = world_db().unwrap().store.get(24275).unwrap();
    let execute = world_db().unwrap().store.get(163201).unwrap();
    assert!(
        super::check_target_aura_restrictions(h.app.world(), h.caster, Some(h.npc), hammer)
            .is_err()
    );
    apply(&mut h, 31884, 1 << 6);
    assert!(
        super::check_target_aura_restrictions(h.app.world(), h.caster, Some(h.npc), hammer).is_ok()
    );
    assert!(
        super::check_target_aura_restrictions(h.app.world(), h.caster, Some(h.npc), execute)
            .is_err()
    );
    assert!(!crate::auras::take_unhandled_auras(h.app.world_mut()).contains(&(31884, 262)));
    remove(&mut h, 31884);
    assert!(
        super::check_target_aura_restrictions(h.app.world(), h.caster, Some(h.npc), hammer)
            .is_err()
    );
}

#[test]
#[ignore = "RED:396 raw-power upward crossing consumer"]
fn census_three_396_native_arcane_charge_crossing_not_percent_or_repeated_gain() {
    let mut h = fixture(8, &[190427, 36032]);
    remove(&mut h, 190427);
    remove(&mut h, 36032);
    apply(&mut h, 190427, 1); // Raw Arcane Charges >=1 -> actual36032.
    assert_eq!(stacks(&h, 36032), 0);
    super::powers::change_power(h.app.world_mut(), h.caster, PowerType::ArcaneCharges, 1.0)
        .unwrap();
    assert_eq!(stacks(&h, 36032), 1);
    super::powers::change_power(h.app.world_mut(), h.caster, PowerType::ArcaneCharges, 1.0)
        .unwrap();
    assert_eq!(stacks(&h, 36032), 1);
    super::powers::change_power(h.app.world_mut(), h.caster, PowerType::ArcaneCharges, -2.0)
        .unwrap();
    super::powers::change_power(h.app.world_mut(), h.caster, PowerType::ArcaneCharges, 1.0)
        .unwrap();
    assert_eq!(stacks(&h, 36032), 2);
    remove(&mut h, 190427);
    super::powers::change_power(h.app.world_mut(), h.caster, PowerType::ArcaneCharges, -1.0)
        .unwrap();
    super::powers::change_power(h.app.world_mut(), h.caster, PowerType::ArcaneCharges, 1.0)
        .unwrap();
    assert_eq!(stacks(&h, 36032), 2);
}

#[test]
#[ignore = "RED:396 inclusive application-time check"]
fn census_three_396_native_application_at_exact_amount_triggers_once() {
    let mut h = fixture(8, &[190427, 36032]);
    remove(&mut h, 190427);
    remove(&mut h, 36032);
    super::powers::change_power(h.app.world_mut(), h.caster, PowerType::ArcaneCharges, 1.0)
        .unwrap();
    apply(&mut h, 190427, 1);
    assert_eq!(stacks(&h, 36032), 1);
    super::powers::change_power(h.app.world_mut(), h.caster, PowerType::Mana, -1.0).unwrap();
    assert_eq!(stacks(&h, 36032), 1);
    h.advance(3);
    assert_eq!(stacks(&h, 36032), 1);
}

fn engage(h: &mut Harness, attacker: Entity, victim: Entity, threat: f32) {
    h.app.world_mut().entity_mut(attacker).insert((
        shared::components::ThreatTable::default(),
        crate::creature_ai::CreatureState::Idle,
    ));
    crate::creature_aggro::add_threat(h.app.world_mut(), attacker, victim, threat);
}

#[test]
#[ignore = "RED:176 native sanctuary outside dungeons ends PvE combat"]
fn census_three_176_native_player_world_combat_stop() {
    let mut h = fixture(4, &[98877]);
    let npc = h.npc;
    let player = h.caster;
    engage(&mut h, npc, player, 100.0);
    assert!(
        h.app
            .world()
            .get::<shared::components::CombatStatus>(player)
            .unwrap()
            .0
    );
    super::creature_cast(h.app.world_mut(), player, 98877, None, false).unwrap();
    assert!(
        !h.app
            .world()
            .get::<shared::components::CombatStatus>(player)
            .unwrap()
            .0
    );
    assert!(
        !h.app
            .world()
            .get::<shared::components::CombatStatus>(npc)
            .unwrap()
            .0
    );
}

#[test]
#[ignore = "RED:176 native nonplayer sanctuary zeros only its own threat reference"]
fn census_three_176_native_nonplayer_resets_target_threat_not_entire_table() {
    let mut h = fixture(4, &[98877]);
    let attacker = super::test_harness::spawn_npc(&mut h.app, 4.0);
    let victim = h.npc;
    let other = h.caster;
    engage(&mut h, attacker, victim, 100.0);
    crate::creature_aggro::add_threat(h.app.world_mut(), attacker, other, 50.0);
    super::creature_cast(h.app.world_mut(), victim, 98877, None, false).unwrap();
    let table = h
        .app
        .world()
        .get::<shared::components::ThreatTable>(attacker)
        .unwrap();
    assert_eq!(table.entries().len(), 2);
    assert_eq!(
        table
            .entries()
            .iter()
            .find(|e| e.entity == victim.to_bits())
            .unwrap()
            .threat,
        0.0
    );
    assert_eq!(
        table
            .entries()
            .iter()
            .find(|e| e.entity == other.to_bits())
            .unwrap()
            .threat,
        50.0
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::CombatStatus>(victim)
            .unwrap()
            .0
    );
}

#[test]
#[ignore = "RED:176 sanctuary fizzles harmful missiles launched before sanctuary"]
fn census_three_176_native_sanctuary_fizzles_inflight_frostbolt_only() {
    let mut h = fixture(8, &[116, 228597, 98877]);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![116]));
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 20.0;
    let before = h.health(h.npc);
    h.cast(116, Some(h.npc));
    let mut launched = false;
    for _ in 0..60 {
        h.advance(1);
        if h.inbox.log.iter().any(|e| {
            e.spell_id == Some(116) && e.kind == shared::protocol::CombatLogKind::CastSuccess
        }) {
            launched = true;
            break;
        }
    }
    assert!(launched, "{:?}", h.inbox.failed);
    super::creature_cast(h.app.world_mut(), h.npc, 98877, None, false).unwrap();
    h.advance(20);
    assert_eq!(h.health(h.npc), before);
    h.cast(116, Some(h.npc));
    h.advance(70);
    assert!(
        h.health(h.npc) < before,
        "new casts after sanctuary still land"
    );
}

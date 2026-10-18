//! Native AP166/167 and ChrClasses ranged AP, TC a352b1fa StatSystem.cpp354–414.
use super::test_harness::*;
use crate::stats::CharacterStats;
use shared::components::Level;

const BATTLE_SHOUT: u32 = 6673;
const AIMED_SHOT: u32 = 19434;

fn actor(class: u8) -> Harness {
    let mut h = harness_with(
        Vec::new(),
        &[BATTLE_SHOUT, AIMED_SHOT, 1271909],
        class,
        true,
    )
    .unwrap();
    h.advance(5);
    h.inbox = Inbox::default();
    h
}

#[test]
fn class_completeness_native_battle_shout_increases_computed_melee_attack_power() {
    let mut h = actor(1);
    let before = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .attack_power;
    assert!(before > 0.0);
    h.cast_and_deliver(BATTLE_SHOUT, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(3);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        BATTLE_SHOUT
    ));
    let after = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .attack_power;
    assert!(
        (after - before * 1.05).abs() < 0.001,
        "native166: before={before} after={after}"
    );
}

#[test]
fn class_completeness_native_aimed_shot_uses_source_ranged_ap_not_melee_ap() {
    let mut h = actor(3);
    let conn = rusqlite::Connection::open_with_flags(
        world_db().unwrap().powers_db.clone(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let coefficient: f32 = conn
        .query_row(
            "SELECT RangedAttackPowerPerAgility FROM chr_classes WHERE ID=3",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let stats = h.app.world().get::<CharacterStats>(h.caster).unwrap();
    let level = h.app.world().get::<Level>(h.caster).unwrap().0;
    let ranged_ap = (f32::from(level) + stats.primary.agility.max(0.0)) * coefficient;
    assert!(ranged_ap > stats.attack_power);
    let ap_coefficient =
        world_db().unwrap().store.get(AIMED_SHOT).unwrap().effects[0].bonus_coefficient_from_ap;
    let expected = varied(ranged_ap * ap_coefficient);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    let before = h.health(h.npc);
    h.cast_and_deliver(AIMED_SHOT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(70);
    let damage = before - h.health(h.npc);
    assert!(
        expected.contains(&damage),
        "native ranged AP {ranged_ap}: expected {expected:?}, damage={damage}"
    );
}

#[test]
fn class_completeness_native_hunter_pet_command_inherits_owner_ranged_not_melee_ap() {
    let mut h = harness(Vec::new(), &[259489, 259277, SUNWELL_RADIANCE], 3).unwrap();
    {
        let mut owner = h
            .app
            .world_mut()
            .get_mut::<CharacterStats>(h.caster)
            .unwrap();
        owner.attack_power = 1.0;
        owner.ranged_attack_power = 1000.0;
    }
    let pet = spawn_player(&mut h.app, 3, 2.0);
    h.app
        .world_mut()
        .entity_mut(pet)
        .remove::<shared::components::Player>()
        .insert(crate::hunter_pet::HunterPet {
            owner: h.caster,
            slot: 0,
        });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::hunter_pet::PetStable {
            character_id: Some(1),
            stored: Default::default(),
            summoned: Some(pet),
        });
    // Qualify the actual pet attacker, not just its owner.
    apply_native_enemy_dodge_reduction_to(&mut h, pet);
    h.cast_and_deliver(259489, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(3);
    assert_eq!(
        5000.0 - h.health(h.npc),
        660.0,
        "native pet:3×(1000 owner RAP×0.22), not owner melee AP1"
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(259277)
                && event.source == Some(pet.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage)
    );
}

#[test]
fn class_completeness_native_ap_derived_spell_and_healing_power_stay_melee_owned() {
    // Native Retribution1258011:96% of melee AP, despite class RAP being zero.
    let mut h = harness_with(Vec::new(), &[1258011, BATTLE_SHOUT], 2, true).unwrap();
    h.advance(6);
    let before = stats(&h);
    assert_eq!(before.ranged_attack_power, 0.0);
    assert_power(
        before.spell_power,
        (before.attack_power * 0.96 + 0.5).floor(),
    );
    let caster = h.caster;
    apply_mask(&mut h, BATTLE_SHOUT, caster, 1);
    let after = stats(&h);
    assert_power(after.attack_power, before.attack_power * 1.05);
    assert_power(after.spell_power, (after.attack_power * 0.96 + 0.5).floor());
    assert_eq!(after.healing_power, after.spell_power);
    assert!(after.spell_power > before.spell_power);
}

fn first_normal_melee_hit(h: &mut Harness) -> f32 {
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    let start = h.inbox.log.len();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::combat::AutoAttack::new(h.npc));
    // Nine native attempts fit18s. P(no unblocked Normal)=0.17^9;
    // both phases together fail below2.38e-7, without extending the window.
    for _ in 0..360 {
        h.advance(1);
        let damage = h.inbox.log[start..]
            .iter()
            .find(|event| {
                event.source == Some(h.caster.to_bits())
                    && event.target == Some(h.npc.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && !event.crit
                    && !event.glancing
                    && event.blocked == 0
            })
            .map(|event| event.amount as f32);
        if let Some(damage) = damage {
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .remove::<crate::combat::AutoAttack>();
            return damage;
        }
    }
    panic!("no actual unblocked Normal melee swing in18s");
}

#[test]
fn class_completeness_native_melee_ap_percent_changes_actual_autoattack_damage() {
    let mut h = harness_with(Vec::new(), &[81420], 1, true).unwrap();
    h.advance(5);
    let before = stats(&h).attack_power;
    let baseline_damage = first_normal_melee_hit(&mut h);
    let caster = h.caster;
    apply_mask(&mut h, 81420, caster, 1);
    assert_power(stats(&h).attack_power, before * 0.70);
    let reduced_damage = first_normal_melee_hit(&mut h);
    assert!(
        reduced_damage < baseline_damage,
        "native166 -30% AP: actual swings {baseline_damage}→{reduced_damage}"
    );
}

fn stats(h: &Harness) -> CharacterStats {
    *h.app.world().get::<CharacterStats>(h.caster).unwrap()
}

fn apply_mask(h: &mut Harness, spell: u32, target: bevy::prelude::Entity, mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target,
            spell_id: spell,
            effect_mask: mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(3);
}

fn assert_power(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "AP {actual} != {expected}"
    );
}

fn assert_aimed_damage(h: &mut Harness, ranged_ap: f32) {
    let coefficient =
        world_db().unwrap().store.get(AIMED_SHOT).unwrap().effects[0].bonus_coefficient_from_ap;
    let expected = varied(ranged_ap * coefficient);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    let before = h.health(h.npc);
    h.cast_and_deliver(AIMED_SHOT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(70);
    let damage = before - h.health(h.npc);
    assert!(
        expected.contains(&damage),
        "expected {expected:?}, damage={damage}"
    );
}

#[test]
fn class_completeness_native_ap_percentage_effects_keep_melee_and_ranged_separate() {
    for (mask, melee_factor, ranged_factor) in [(1, 1.05, 1.0), (2, 1.0, 1.05)] {
        let mut h = actor(3);
        let before = stats(&h);
        let caster = h.caster;
        apply_mask(&mut h, BATTLE_SHOUT, caster, mask);
        let after = stats(&h);
        assert_power(after.attack_power, before.attack_power * melee_factor);
        assert_power(
            after.ranged_attack_power,
            before.ranged_attack_power * ranged_factor,
        );
        assert_aimed_damage(&mut h, before.ranged_attack_power * ranged_factor);
    }
}

#[test]
fn class_completeness_native_ap_buff_on_other_recipient_does_not_change_caster() {
    let mut h = actor(3);
    let before = stats(&h);
    let npc = h.npc;
    apply_mask(&mut h, BATTLE_SHOUT, npc, 3);
    assert_power(stats(&h).attack_power, before.attack_power);
    assert_power(stats(&h).ranged_attack_power, before.ranged_attack_power);
    assert_aimed_damage(&mut h, before.ranged_attack_power);
}

#[test]
fn class_completeness_native_ap_zero_ranged_sibling_is_identity() {
    let mut h = actor(3);
    let before = stats(&h);
    let caster = h.caster;
    apply_mask(&mut h, BATTLE_SHOUT, caster, 1 << 3);
    assert_power(stats(&h).attack_power, before.attack_power);
    assert_power(stats(&h).ranged_attack_power, before.ranged_attack_power);
    assert_aimed_damage(&mut h, before.ranged_attack_power);
}

#[test]
fn class_completeness_native_ap_reapplication_does_not_compound_the_same_buff() {
    let mut h = actor(3);
    let before = stats(&h);
    let caster = h.caster;
    apply_mask(&mut h, BATTLE_SHOUT, caster, 3);
    apply_mask(&mut h, BATTLE_SHOUT, caster, 3);
    assert_power(stats(&h).attack_power, before.attack_power * 1.05);
    assert_power(
        stats(&h).ranged_attack_power,
        before.ranged_attack_power * 1.05,
    );
    assert_aimed_damage(&mut h, before.ranged_attack_power * 1.05);
}

#[test]
fn class_completeness_native_ap_removal_restores_both_values_and_ranged_damage() {
    let mut h = actor(3);
    let before = stats(&h);
    let caster = h.caster;
    apply_mask(&mut h, BATTLE_SHOUT, caster, 3);
    assert_power(
        stats(&h).ranged_attack_power,
        before.ranged_attack_power * 1.05,
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == BATTLE_SHOUT,
    );
    h.advance(3);
    assert_power(stats(&h).attack_power, before.attack_power);
    assert_power(stats(&h).ranged_attack_power, before.ranged_attack_power);
    assert_aimed_damage(&mut h, before.ranged_attack_power);
}

#[test]
fn class_completeness_native_ap_buff_expires_on_its_authored_clock() {
    let mut h = actor(3);
    let before = stats(&h);
    let caster = h.caster;
    apply_mask(&mut h, BATTLE_SHOUT, caster, 3);
    let duration = world_db()
        .unwrap()
        .store
        .get(BATTLE_SHOUT)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms;
    assert!(duration > 0);
    let step = std::time::Duration::from_millis(duration as u64 + 1000);
    h.app
        .world_mut()
        .resource_mut::<bevy::prelude::Time<bevy::time::Virtual>>()
        .set_max_delta(step);
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(step));
    h.advance(2);
    assert!(!crate::auras::has_aura(h.app.world(), caster, BATTLE_SHOUT));
    assert_power(stats(&h).attack_power, before.attack_power);
    assert_power(stats(&h).ranged_attack_power, before.ranged_attack_power);
}

#[test]
fn class_completeness_native_ap_signed_independent_percentages_multiply() {
    // Native Super Shrink Ray81420:166=-30%,167=-30%; no synthetic modifier values.
    let mut h = harness_with(Vec::new(), &[BATTLE_SHOUT, AIMED_SHOT, 81420], 3, true).unwrap();
    h.advance(5);
    let before = stats(&h);
    let caster = h.caster;
    apply_mask(&mut h, BATTLE_SHOUT, caster, 3);
    apply_mask(&mut h, 81420, caster, (1 << 0) | (1 << 2));
    assert_power(stats(&h).attack_power, before.attack_power * 1.05 * 0.70);
    assert_power(
        stats(&h).ranged_attack_power,
        before.ranged_attack_power * 1.05 * 0.70,
    );
    assert_aimed_damage(&mut h, before.ranged_attack_power * 1.05 * 0.70);
}

#[test]
fn class_completeness_native_mastery_ap_percent_consumes_live_raw_mastery_points() {
    for (class, spell) in [
        (2, 76671),
        (1, 76857),
        (6, 77513),
        (10, 117906),
        (11, 159195),
        (12, 321299),
    ] {
        let mut h = harness_with(Vec::new(), &[spell], class, true).unwrap();
        if matches!(class, 11 | 12) {
            h.app
                .world_mut()
                .get_mut::<shared::components::Player>(h.caster)
                .unwrap()
                .race = 4;
        }
        h.advance(6);
        let current = stats(&h);
        let data = world_db().unwrap();
        let class_info = data
            .classes
            .classes
            .iter()
            .find(|info| info.class_id == class)
            .unwrap();
        let base = current.primary.strength * class_info.attack_power_per_strength
            + current.primary.agility * class_info.attack_power_per_agility;
        let effect = data
            .store
            .get(spell)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.aura == 166)
            .unwrap();
        let amount = effect.base_points + current.mastery_points * effect.bonus_coefficient;
        assert!(
            amount > 0.0,
            "native mastery166 must not be treated as static zero"
        );
        assert_power(current.attack_power, base * (1.0 + amount / 100.0));
        assert_eq!(current.ranged_attack_power, 0.0);
        assert!(crate::auras::has_aura(h.app.world(), h.caster, spell));
    }
}

#[test]
fn class_completeness_native_owner_wire_publishes_distinct_ranged_attack_power() {
    let h = actor(3);
    let stats = h.app.world().get::<CharacterStats>(h.caster).unwrap();
    assert!(stats.ranged_attack_power > stats.attack_power);
    let owner_wire = serde_json::to_value(stats.derived()).unwrap();
    assert_eq!(
        owner_wire["attack_power"].as_f64(),
        Some(f64::from(stats.attack_power))
    );
    assert_eq!(
        owner_wire["ranged_attack_power"].as_f64(),
        Some(f64::from(stats.ranged_attack_power)),
        "ranged owner field must not be omitted or replaced with melee AP"
    );
}

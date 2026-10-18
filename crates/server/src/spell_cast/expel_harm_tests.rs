//! Native70448 Mistweaver322101 ->115129; SimC76f8118 monk4002–4070.
//! Total heal includes overheal. No spheres, Energy, tie-selection or legal-build claim.
use super::test_harness::*;
use crate::auras::{self, AuraApplication, AuraSource};
use crate::class_progression::ActiveSpec;
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::{Level, Mana, Position};
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

const EXPEL_HARM: u32 = 322101;
const DAMAGE: u32 = 115129;
const MISTWEAVER: u32 = 137024;
const VIVIFY: u32 = 116670;
const ENVELOPING_MIST: u32 = 124682;
const FIREBALL: u32 = 133;
const MARK: u32 = 1126;
const SHIELD: u32 = 17;
const NATURE_IMMUNITY: u32 = 7941;
const INSTANT_KILL: u32 = 5;
const IDS: &[u32] = &[
    EXPEL_HARM,
    DAMAGE,
    MISTWEAVER,
    VIVIFY,
    ENVELOPING_MIST,
    FIREBALL,
    MARK,
    SHIELD,
    NATURE_IMMUNITY,
    INSTANT_KILL,
];

fn fixture(distance: f32) -> Harness {
    let mut h = harness_with(Vec::new(), IDS, 10, true).unwrap();
    let data = world_db().unwrap();
    let source = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&source).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), ActiveSpec(270)));
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: distance,
        y: 0.0,
        z: 0.0,
    });
    h.advance(4);
    assert!(auras::has_aura(h.app.world(), h.caster, MISTWEAVER));
    assert!(!auras::has_aura(h.app.world(), h.caster, 137023));
    assert!(!auras::has_aura(h.app.world(), h.caster, 137025));
    h
}

fn events<'a>(h: &'a Harness, spell: u32, kind: CombatLogKind) -> Vec<&'a CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.source == Some(h.caster.to_bits()) && e.spell_id == Some(spell) && e.kind == kind
        })
        .collect()
}

fn expel(h: &mut Harness) -> CombatLogEvent {
    let start = h.inbox.log.len();
    h.cast_and_deliver(EXPEL_HARM, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let heals: Vec<_> = h.inbox.log[start..]
        .iter()
        .filter(|e| e.spell_id == Some(EXPEL_HARM) && e.kind == CombatLogKind::Heal)
        .collect();
    assert_eq!(heals.len(), 1);
    assert_eq!(heals[0].target, Some(h.caster.to_bits()));
    assert!(heals[0].amount > 0);
    heals[0].clone()
}

fn assert_damage(h: &Harness, heal: &CombatLogEvent, target: Entity) {
    let hits = events(h, DAMAGE, CombatLogKind::Damage);
    assert_eq!(hits.len(), 1, "native Expel Harm child");
    assert_eq!(hits[0].target, Some(target.to_bits()));
    assert_eq!(hits[0].amount as f32, (heal.amount as f32 * 0.10).round());
    assert!(
        !hits[0].crit,
        "native child CANT_CRIT; heal crit is already included"
    );
    assert!(!hits[0].periodic);
}

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell: u32) {
    auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

/// Existing fixture Mage has100SP; native Fireball's unchanged coefficient,
/// variance and crit produce the injury, never a direct Health/stat write.
fn injure(h: &mut Harness, hits: usize) {
    let enemy = spawn_player(&mut h.app, 8, -3.0);
    let (caster, before) = (h.caster, h.health(h.caster));
    for _ in 0..hits {
        super::cast_triggered(h.app.world_mut(), enemy, FIREBALL, Some(caster), 1);
        h.advance(10);
    }
    assert!(h.health(caster) > 0.0);
    assert!(h.health(caster) < before);
}

#[test]
fn mistweaver_expel_native_child_selector_qualification_uses_radius_twenty_not_range_ten() {
    for distance in [15.0, 21.0] {
        let mut h = fixture(distance);
        // Independent existing-heal control remains unaffected by the future
        //322101-only callback; this is not automatic-bridge RED evidence.
        h.cast_and_deliver(VIVIFY, Some(h.caster));
        h.advance(40);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let heal = events(&h, VIVIFY, CombatLogKind::Heal)[0].clone();
        let caster = h.caster;
        super::cast_triggered_with_effect0_amount(
            h.app.world_mut(),
            caster,
            DAMAGE,
            None,
            1,
            heal.amount as f32 * 0.10,
        );
        h.advance(3);
        if distance < 20.0 {
            assert_damage(&h, &heal, h.npc);
        } else {
            assert!(events(&h, DAMAGE, CombatLogKind::Damage).is_empty());
        }
    }
}

#[test]
fn mistweaver_expel_native_full_overheal_still_damages_and_pays_only_native_mana() {
    let mut h = fixture(15.0);
    let before_health = h.health(h.caster);
    let enemy_health = h.health(h.npc);
    let before_mana = h.app.world().get::<Mana>(h.caster).unwrap().current;
    let base_mana = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .base_mana;
    let heal = expel(&mut h);
    assert_eq!(
        heal.overflow, heal.amount,
        "pinned result_total is not effective healing"
    );
    assert_eq!(h.health(h.caster), before_health);
    assert_damage(&h, &heal, h.npc);
    assert_eq!(
        enemy_health - h.health(h.npc),
        (heal.amount as f32 * 0.10).round()
    );
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        before_mana - base_mana * (1.399_999_98 / 100.0)
    );
    let root = h
        .inbox
        .log
        .iter()
        .position(|e| e.spell_id == Some(EXPEL_HARM) && e.kind == CombatLogKind::Heal)
        .unwrap();
    let child = h
        .inbox
        .log
        .iter()
        .position(|e| e.spell_id == Some(DAMAGE) && e.kind == CombatLogKind::Damage)
        .unwrap();
    assert!(
        root < child,
        "heal resolves before zero-distance caster-destination child"
    );
}

#[test]
fn mistweaver_expel_native_injury_uses_the_same_resolved_heal_not_a_new_roll() {
    let mut h = fixture(15.0);
    injure(&mut h, 4);
    let before = h.health(h.caster);
    let heal = expel(&mut h);
    assert_eq!(
        h.health(h.caster) - before,
        (heal.amount - heal.overflow) as f32
    );
    assert_damage(&h, &heal, h.npc);
}

#[test]
fn mistweaver_expel_native_partial_overheal_uses_total_not_health_gained() {
    let mut h = fixture(15.0);
    injure(&mut h, 1);
    let before = h.health(h.caster);
    let heal = expel(&mut h);
    assert!(heal.overflow > 0 && heal.overflow < heal.amount, "{heal:?}");
    assert_eq!(
        h.health(h.caster) - before,
        (heal.amount - heal.overflow) as f32
    );
    assert_damage(&h, &heal, h.npc);
}

#[test]
fn mistweaver_expel_native_versatility_in_total_is_not_applied_twice_to_child() {
    let mut h = fixture(15.0);
    let caster = h.caster;
    apply(&mut h, caster, caster, MARK);
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .versatility_pct,
        3.0
    );
    let heal = expel(&mut h);
    assert_damage(&h, &heal, h.npc);
}

#[test]
fn mistweaver_expel_native_area_cap_is_one_without_claiming_a_tie_policy() {
    let mut h = fixture(15.0);
    let other = spawn_npc(&mut h.app, 15.0);
    let before = h.health(h.npc) + h.health(other);
    let heal = expel(&mut h);
    let hits = events(&h, DAMAGE, CombatLogKind::Damage);
    assert_eq!(hits.len(), 1);
    assert!([h.npc.to_bits(), other.to_bits()].contains(&hits[0].target.unwrap()));
    assert_eq!(
        before - h.health(h.npc) - h.health(other),
        (heal.amount as f32 * 0.10).round()
    );
}

#[test]
fn mistweaver_expel_native_no_eligible_enemy_preserves_heal_and_never_hits_friends_or_other_copy() {
    let mut h = fixture(21.0);
    let friend = spawn_player(&mut h.app, 8, 3.0);
    let other_copy = spawn_npc(&mut h.app, 3.0);
    h.app
        .world_mut()
        .entity_mut(other_copy)
        .insert(crate::maps::InMap::continent(1));
    let before = (h.health(friend), h.health(other_copy), h.health(h.npc));
    let heal = expel(&mut h);
    assert_eq!(heal.overflow, heal.amount);
    assert!(events(&h, DAMAGE, CombatLogKind::Damage).is_empty());
    assert_eq!(
        (h.health(friend), h.health(other_copy), h.health(h.npc)),
        before
    );
}

#[test]
fn mistweaver_expel_native_dead_enemy_does_not_receive_child_damage() {
    let mut h = fixture(15.0);
    let (caster, target) = (h.caster, h.npc);
    super::cast_triggered(h.app.world_mut(), caster, INSTANT_KILL, Some(target), 1);
    h.advance(3);
    assert_eq!(h.health(target), 0.0);
    let heal = expel(&mut h);
    assert!(heal.amount > 0);
    assert!(events(&h, DAMAGE, CombatLogKind::Damage).is_empty());
}

#[test]
fn mistweaver_expel_native_immunity_and_absorb_apply_to_actual_child_not_the_self_heal() {
    for spell in [NATURE_IMMUNITY, SHIELD] {
        let mut h = fixture(15.0);
        let (caster, target) = (h.caster, h.npc);
        let priest = spawn_player(&mut h.app, 5, 0.0);
        apply(&mut h, priest, target, spell);
        let before = h.health(target);
        let heal = expel(&mut h);
        assert_eq!(heal.overflow, heal.amount);
        assert_eq!(h.health(target), before);
        if spell == SHIELD {
            let hits = events(&h, DAMAGE, CombatLogKind::Damage);
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].amount, 0);
            assert_eq!(hits[0].absorbed as f32, (heal.amount as f32 * 0.10).round());
        } else {
            assert_eq!(
                events(&h, DAMAGE, CombatLogKind::Miss(MissKind::Immune)).len(),
                1
            );
        }
        assert!(auras::has_aura(h.app.world(), target, spell));
        assert!(caster != priest);
    }
}

#[test]
fn mistweaver_expel_native_enveloping_and_vivify_preserve_healing_without_damage_children() {
    let mut h = fixture(15.0);
    injure(&mut h, 4);
    for spell in [ENVELOPING_MIST, VIVIFY] {
        h.cast_and_deliver(spell, Some(h.caster));
        h.advance(50);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(
            events(&h, spell, CombatLogKind::Heal)
                .iter()
                .any(|e| e.amount > 0)
        );
        assert!(events(&h, DAMAGE, CombatLogKind::Damage).is_empty());
    }
}

#[test]
fn mistweaver_expel_native_repeated_casts_have_one_child_per_own_heal_without_stale_amounts() {
    let mut h = fixture(15.0);
    let cooldown = world_db()
        .unwrap()
        .store
        .get(EXPEL_HARM)
        .unwrap()
        .cooldowns
        .as_ref()
        .unwrap();
    let wait = cooldown.recovery_ms.max(cooldown.category_recovery_ms) as usize / 50 + 4;
    let mut totals = Vec::new();
    for _ in 0..3 {
        totals.push(expel(&mut h).amount);
        h.advance(wait);
    }
    let hits = events(&h, DAMAGE, CombatLogKind::Damage);
    assert_eq!(hits.len(), 3);
    for (hit, total) in hits.iter().zip(totals) {
        assert_eq!(hit.amount as f32, (total as f32 * 0.10).round());
        assert_eq!(hit.source, Some(h.caster.to_bits()));
    }
}

//! TC a352b1fa Unit.cpp:6962/8110/9476–9500: native aura168 and unit types.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, apply_aura};
use shared::components::{Npc, Position};

const FIREBALL: u32 = 133;
const NEMESIS_UNDEAD: u32 = 208614;
const FLESH_EATER: u32 = 3;

fn native_undead() -> crate::world_data::WorldData {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|entry, _| [FLESH_EATER, 6, 113].contains(entry));
    assert_eq!(templates[&FLESH_EATER].creature_type, 6);
    crate::world_data::WorldData {
        templates,
        ..Default::default()
    }
}

fn damage_native_undead(bonus: bool) -> f32 {
    let mut h = harness_with_plugins(Vec::new(), &[FIREBALL, NEMESIS_UNDEAD], 8, false, |app| {
        app.insert_resource(native_undead());
    })
    .unwrap();
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: FLESH_EATER,
            name: "Flesh Eater".into(),
        },
        Position {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    if bonus {
        let (caster, target) = (h.caster, h.caster);
        apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster,
                target,
                spell_id: NEMESIS_UNDEAD,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
        assert!(crate::auras::has_aura(
            h.app.world(),
            caster,
            NEMESIS_UNDEAD
        ));
    }
    let before = h.health(h.npc);
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let loss = before - h.health(h.npc);
    assert!(loss > 0.0, "real Fireball must damage the native undead");
    loss
}

#[test]
fn class_completeness_native_creature_type_damage_adds_twenty_percent_against_undead() {
    let baseline = damage_native_undead(false);
    let enhanced = damage_native_undead(true);
    // Native Fireball:100 SP×3.8812499 with its authored±2.5% variance.
    // Baseline378..398 and +20%454..477 do not overlap; independent rolls need
    // not match exactly, and cannot explain away a missing damage modifier.
    assert!(
        (378.0..=398.0).contains(&baseline),
        "native baseline {baseline}"
    );
    assert!(
        (454.0..=477.0).contains(&enhanced),
        "native aura168 mask32 must affect actual Fire damage: {enhanced}"
    );
}

#[test]
fn class_completeness_native_creature_type_damage_adds_twenty_percent_to_real_autoattack() {
    let mut h = harness_with_plugins(Vec::new(), &[NEMESIS_UNDEAD], 8, false, |app| {
        app.insert_resource(native_undead());
    })
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<crate::stats::CharacterStats>();
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: FLESH_EATER,
            name: "Flesh Eater".into(),
        },
        Position {
            x: 3.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    let (caster, target) = (h.caster, h.caster);
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id: NEMESIS_UNDEAD,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::combat::AutoAttack::new(h.npc));
    let before = h.health(h.npc);
    h.advance(2);
    assert_eq!(
        before - h.health(h.npc),
        12.0,
        "native unarmed10 × aura168 +20% must hit actual health"
    );
}

fn controls() -> Harness {
    let mut h = harness_with_plugins(
        Vec::new(),
        &[
            FIREBALL,
            NEMESIS_UNDEAD,
            131283,
            217319,
            106620,
            5487,
            768,
            197625,
            10326,
            1513,
            2061,
        ],
        8,
        false,
        |app| {
            app.insert_resource(native_undead());
        },
    )
    .unwrap();
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    // Keep all native rows loaded, but apply only this case's chosen modifier.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            FIREBALL, 5487, 768, 197625, 10326, 1513, 2061,
        ]));
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| [NEMESIS_UNDEAD, 131283, 217319, 106620].contains(&aura.spell_id),
    );
    h
}

fn apply_modifier(h: &mut Harness, id: u32, target: bevy::prelude::Entity) {
    let caster = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), target, id));
}

fn cast_fire_loss(h: &mut Harness, victim: bevy::prelude::Entity) -> f32 {
    h.inbox = Inbox::default();
    let before = h.health(victim);
    h.cast_and_deliver(FIREBALL, Some(victim));
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == FIREBALL));
    before - h.health(victim)
}

fn place_native_npc(h: &mut Harness, entry: u32) {
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: entry,
            name: "native creature type fixture".into(),
        },
        Position {
            x: 3.0,
            y: 0.0,
            z: 0.0,
        },
    ));
}

#[test]
fn class_completeness_native_creature_type_damage_zero_mask_and_wrong_type_do_not_match() {
    for (bonus, entry) in [
        (106620, FLESH_EATER),
        (NEMESIS_UNDEAD, 113),
        (131283, FLESH_EATER),
    ] {
        let mut h = controls();
        place_native_npc(&mut h, entry);
        let caster = h.caster;
        apply_modifier(&mut h, bonus, caster);
        let npc = h.npc;
        let loss = cast_fire_loss(&mut h, npc);
        assert!(
            (378.0..=398.0).contains(&loss),
            "bonus{bonus} vs template{entry}: {loss}"
        );
    }
}

#[test]
fn class_completeness_native_creature_type_damage_victim_modifier_does_not_buff_attacker() {
    let mut h = controls();
    place_native_npc(&mut h, FLESH_EATER);
    let npc = h.npc;
    apply_modifier(&mut h, NEMESIS_UNDEAD, npc);
    let loss = cast_fire_loss(&mut h, npc);
    assert!(
        (378.0..=398.0).contains(&loss),
        "victim's modifier must not multiply {loss}"
    );
}

#[test]
fn class_completeness_native_creature_type_damage_combines_matching_auras_and_removes_one() {
    let mut h = controls();
    place_native_npc(&mut h, FLESH_EATER);
    let (caster, npc) = (h.caster, h.npc);
    apply_modifier(&mut h, NEMESIS_UNDEAD, caster);
    apply_modifier(&mut h, 217319, caster);
    let loss = cast_fire_loss(&mut h, npc);
    assert!(
        (545.0..=573.0).contains(&loss),
        "two native20% multipliers: {loss}"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == NEMESIS_UNDEAD,
    );
    let loss = cast_fire_loss(&mut h, npc);
    assert!(
        (454.0..=477.0).contains(&loss),
        "remaining native20%: {loss}"
    );
}

fn taurendruid(h: &mut Harness) -> bevy::prelude::Entity {
    let player = spawn_player(&mut h.app, 11, 3.0);
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(player)
        .unwrap()
        .race = 6;
    h.app.world_mut().entity_mut(player).insert((
        shared::components::UnitFactionTemplate(6),
        shared::components::Health {
            current: 10000.0,
            max: 10000.0,
        },
    ));
    h.advance(3);
    player
}

#[test]
fn class_completeness_native_creature_type_damage_tracks_actual_player_bear_and_removal() {
    let mut h = controls();
    let victim = taurendruid(&mut h);
    let caster = h.caster;
    apply_modifier(&mut h, 131283, caster);
    let loss = cast_fire_loss(&mut h, victim);
    assert!(
        (378.0..=398.0).contains(&loss),
        "native Tauren humanoid before form: {loss}"
    );
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        victim,
        5487,
        Some(victim),
        1
    ));
    let loss = cast_fire_loss(&mut h, victim);
    assert!(
        (454.0..=477.0).contains(&loss),
        "native Bear CreatureType1: {loss}"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        victim,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 5487,
    );
    let loss = cast_fire_loss(&mut h, victim);
    assert!(
        (378.0..=398.0).contains(&loss),
        "native race type restored: {loss}"
    );
}

#[test]
fn class_completeness_native_creature_type_metadata_preserves_non_humanoid_race_and_unknown_error()
{
    let mut h = controls();
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(h.caster)
        .unwrap()
        .race = 99;
    assert_eq!(
        crate::unit_creature_type::get(h.app.world(), h.caster),
        4,
        "source Stoneforged Sentinel type"
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(h.caster)
        .unwrap()
        .race = 250;
    let result = crate::unit_creature_type::CreatureTypes::from_world(h.app.world()).resolve(
        h.app.world().get::<shared::components::Player>(h.caster),
        None,
        None,
    );
    assert!(
        result.is_err(),
        "unknown race must not silently become Humanoid7"
    );
}

#[test]
fn class_completeness_native_creature_type_damage_applies_to_offhand_against_player_form() {
    let mut h = controls();
    let victim = taurendruid(&mut h);
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        victim,
        5487,
        Some(victim),
        1
    ));
    h.app
        .world_mut()
        .entity_mut(victim)
        .remove::<crate::stats::CharacterStats>();
    let caster = h.caster;
    apply_modifier(&mut h, 131283, caster);
    h.app
        .world_mut()
        .get_mut::<crate::auras::AuraStatMods>(caster)
        .unwrap()
        .melee_hit_chance_pct = 100.0;
    let weapon = crate::item_stats::WeaponDamage {
        min: 60.0,
        max: 60.0,
        speed: 2.0,
        kind: shared::components::WeaponType::OneHand,
    };
    {
        let mut stats = h
            .app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(caster)
            .unwrap();
        stats.attack_power = 0.0;
        stats.off_hand = Some(weapon);
    }
    let mut attack = crate::combat::AutoAttack::new(victim);
    attack.timer = 100.0;
    h.app.world_mut().entity_mut(caster).insert(attack);
    let before = h.health(victim);
    h.advance(2);
    assert_eq!(before - h.health(victim), 36.0, "offhand60×0.5×native20%");
}

#[test]
fn class_completeness_native_creature_type_damage_does_not_double_modify_resolved_copy() {
    let mut h = controls();
    place_native_npc(&mut h, FLESH_EATER);
    let (caster, victim) = (h.caster, h.npc);
    apply_modifier(&mut h, NEMESIS_UNDEAD, caster);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(FIREBALL)
        .unwrap()
        .clone();
    let ctx = super::effects::EffectCtx {
        caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let before = h.health(victim);
    super::effects::deal_copied_damage(h.app.world_mut(), &ctx, victim, 40.0);
    assert_eq!(
        before - h.health(victim),
        40.0,
        "already-resolved copy is not multiplied again"
    );
}

#[test]
fn class_completeness_native_creature_type_damage_expiry_restores_baseline() {
    let mut h = controls();
    place_native_npc(&mut h, FLESH_EATER);
    let (caster, victim) = (h.caster, h.npc);
    apply_modifier(&mut h, NEMESIS_UNDEAD, caster);
    let loss = cast_fire_loss(&mut h, victim);
    assert!((454.0..=477.0).contains(&loss));
    h.advance(1260); // Native Nemesis duration60000ms, no synthetic timer edit.
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        NEMESIS_UNDEAD
    ));
    let loss = cast_fire_loss(&mut h, victim);
    assert!((378.0..=398.0).contains(&loss), "expiry: {loss}");
}

#[test]
fn class_completeness_native_creature_type_damage_does_not_modify_healing_to_matching_form() {
    let mut h = controls();
    let victim = taurendruid(&mut h);
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(victim)
        .unwrap()
        .race = 4;
    h.app
        .world_mut()
        .entity_mut(victim)
        .insert(shared::components::UnitFactionTemplate(4));
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        victim,
        5487,
        Some(victim),
        1
    ));
    let caster = h.caster;
    apply_modifier(&mut h, 131283, caster);
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(victim)
        .unwrap()
        .current = 400.0;
    h.inbox = Inbox::default();
    h.cast_and_deliver(2061, Some(victim));
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let healing = h.health(victim) - 400.0;
    assert!(
        (665.0..=699.0).contains(&healing),
        "native FlashHeal100×6.82±2.5%, not20% bonus: {healing}"
    );
}

#[test]
fn class_completeness_native_creature_type_target_admission_uses_actual_player_form() {
    let mut h = harness_with_plugins(Vec::new(), &[1513, 5487], 3, false, |app| {
        app.insert_resource(native_undead());
    })
    .unwrap();
    let victim = taurendruid(&mut h);
    h.cast_and_deliver(1513, Some(victim));
    h.advance(45);
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::InvalidTarget
    );
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        victim,
        5487,
        Some(victim),
        1
    ));
    h.inbox = Inbox::default();
    h.cast_and_deliver(1513, Some(victim));
    h.advance(45);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 1513));
    assert!(h.app.world().get::<crate::auras::Feared>(victim).is_some());
}

#[test]
fn class_completeness_native_creature_type_damage_zero_form_type_uses_authored_race() {
    let mut h = controls();
    let victim = taurendruid(&mut h);
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        victim,
        197625,
        Some(victim),
        1
    ));
    assert_eq!(
        crate::unit_creature_type::get(h.app.world(), victim),
        7,
        "native Moonkin31 CreatureType0 uses native race"
    );
}

#[test]
fn class_completeness_native_creature_type_damage_native_ignore_caster_flags_are_preserved() {
    for id in [65280, 65723] {
        let mut h = harness_with_plugins(Vec::new(), &[id, NEMESIS_UNDEAD], 8, false, |app| {
            app.insert_resource(native_undead());
        })
        .unwrap();
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .crit_pct = 0.0;
        place_native_npc(&mut h, FLESH_EATER);
        let (caster, victim) = (h.caster, h.npc);
        h.app
            .world_mut()
            .entity_mut(victim)
            .insert(shared::components::Health {
                current: 1_000_000.0,
                max: 1_000_000.0,
            });
        h.cast_and_deliver(id, Some(victim));
        h.advance(40);
        assert!(
            h.inbox.failed.is_empty(),
            "native{id}: {:?}",
            h.inbox.failed
        );
        let baseline = 1_000_000.0 - h.health(victim);
        assert!(baseline > 0.0);
        apply_modifier(&mut h, NEMESIS_UNDEAD, caster);
        h.app
            .world_mut()
            .get_mut::<shared::components::Health>(victim)
            .unwrap()
            .current = 1_000_000.0;
        h.inbox = Inbox::default();
        h.cast_and_deliver(id, Some(victim));
        h.advance(40);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            1_000_000.0 - h.health(victim),
            baseline,
            "native ignore-caster flag{id} excludes168"
        );
    }
}

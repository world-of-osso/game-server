//! TC EffectThreat ignores threat modifiers;70448 Kvaldir has a literal level49 cap.
use super::*;
use crate::creature_ai::CreatureState;
use crate::spell_info::SpellStore;
use shared::components::{Health, ThreatTable, UnitLevel};

const HORN: u32 = 129111;
const SPELL: u32 = 197828;

fn install_source(h: &mut Harness) {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
}

fn threat_creature(h: &mut Harness, x: f32, level: u8) -> Entity {
    let unit = spawn_npc(&mut h.app, x);
    h.app.world_mut().entity_mut(unit).insert((
        UnitLevel(level),
        ThreatTable::default(),
        CreatureState::Idle,
    ));
    unit
}

fn threat(h: &Harness, creature: Entity, attacker: Entity) -> f32 {
    h.app
        .world()
        .get::<ThreatTable>(creature)
        .unwrap()
        .threat_for(attacker.to_bits())
}

#[test]
fn toy_kvaldir_real_use_casts_then_pulls_only_source_range_level_and_copy_audience() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("kvaldir.redb"));
    install_source(&mut h);
    let owner = h.caster;
    let eligible = threat_creature(&mut h, 20.0, 49);
    let too_high = threat_creature(&mut h, 20.0, 50);
    let far = threat_creature(&mut h, 50.0, 49);
    let other_copy = threat_creature(&mut h, 20.0, 49);
    h.app
        .world_mut()
        .entity_mut(other_copy)
        .insert(crate::maps::InMap {
            map_id: 0,
            instance_id: 42,
        });
    let dead = threat_creature(&mut h, 20.0, 49);
    h.app.world_mut().get_mut::<Health>(dead).unwrap().current = 0.0;
    let passive = spawn_npc(&mut h.app, 20.0);
    let friendly = threat_creature(&mut h, 20.0, 49);
    h.app
        .world_mut()
        .entity_mut(friendly)
        .insert(shared::components::UnitFactionTemplate(1));
    let player = spawn_player(&mut h.app, 8, 20.0);
    learn_effect_toy(&mut h, character, HORN);
    let health = h.health(eligible);
    assert_eq!(activate(&mut h, HORN)[0].error, None);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(owner)
            .is_some()
    );
    h.advance(12); // Pump350ms+600ms remains below source1500ms.
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    assert_eq!(threat(&h, eligible, owner), 0.0);
    h.advance(25);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    assert_eq!(threat(&h, eligible, owner), 1.0);
    assert_eq!(h.health(eligible), health);
    assert!(h.app.world().get::<CombatStatus>(eligible).unwrap().0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(eligible)
            .unwrap()
            .0,
        Some(owner)
    );
    for untouched in [too_high, far, other_copy, dead, friendly] {
        let table = h.app.world().get::<ThreatTable>(untouched).unwrap();
        assert!(
            table.entries().is_empty(),
            "{untouched:?} must not receive zero-threat hit bookkeeping"
        );
        assert!(
            !h.app
                .world()
                .get::<CombatStatus>(untouched)
                .is_some_and(|status| status.0)
        );
    }
    for untouched in [passive, player] {
        assert!(h.app.world().get::<ThreatTable>(untouched).is_none());
    }
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == SPELL && cd.duration_ms == 120_000 && !cd.is_gcd)
    );
    assert_eq!(
        activate(&mut h, HORN)[0].error.as_deref(),
        Some("OnCooldown")
    );
}

#[test]
fn toy_kvaldir_effect63_bypasses_source_school_and_taunt_modifiers_without_taunting() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("threat-modifiers.redb"));
    install_source(&mut h);
    let owner = h.caster;
    let npc = threat_creature(&mut h, 20.0, 49);
    let rival = spawn_player(&mut h.app, 8, 22.0);
    crate::creature_aggro::add_threat(h.app.world_mut(), npc, rival, 100.0);
    // Actual Righteous Fury950% physical-school modifier and Provoke800% target modifier.
    for (spell_id, target) in [(25780, owner), (116189, npc)] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: owner,
                target,
                spell_id,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(crate::auras::has_aura(h.app.world(), target, spell_id));
    }
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(SPELL)
        .unwrap()
        .clone();
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: owner,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let handler = crate::spell_cast::effects::handler(63).expect("generic source EffectThreat");
    handler(h.app.world_mut(), &ctx, &spell.effects[0], npc);
    assert_eq!(threat(&h, npc, owner), 1.0);
    assert_eq!(threat(&h, npc, rival), 100.0);
    assert_eq!(
        crate::creature_aggro::current_victim(h.app.world_mut(), npc),
        Some(rival)
    );
    crate::creature_aggro::add_threat(h.app.world_mut(), npc, owner, 1.0);
    assert_eq!(
        threat(&h, npc, owner),
        95.5,
        "ordinary threat retains10.5x9 modifiers"
    );
    h.app.world_mut().get_mut::<Health>(owner).unwrap().current = 0.0;
    handler(h.app.world_mut(), &ctx, &spell.effects[0], npc);
    assert_eq!(
        threat(&h, npc, owner),
        95.5,
        "dead caster generates no EffectThreat"
    );
}

#[test]
fn toy_kvaldir_zero_max_target_level_keeps_uncapped_generic_effect63() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("uncapped-threat.redb"));
    install_source(&mut h);
    let owner = h.caster;
    let npc = threat_creature(&mut h, 20.0, 80);
    let mut spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(SPELL)
        .unwrap()
        .clone();
    spell.target_restrictions.as_mut().unwrap().max_target_level = 0;
    h.app.insert_resource(SpellStore::fixture(vec![spell]));
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        owner,
        SPELL,
        None,
        0
    ));
    assert_eq!(threat(&h, npc, owner), 1.0);
}

#[test]
fn toy_kvaldir_census_admits_horn_but_retains_battle_horn_native_audience_gap() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_KVALDIR_REPORT");
    let rows = report["toys"].as_array().unwrap();
    let horn = rows.iter().find(|row| row["item_id"] == HORN).unwrap();
    assert!(horn["unavailable"].is_null(), "{horn}");
    let battle = rows.iter().find(|row| row["item_id"] == 86565).unwrap();
    assert!(
        !battle["unavailable"].is_null(),
        "Pandaria-native creature binding is not sourced"
    );
}

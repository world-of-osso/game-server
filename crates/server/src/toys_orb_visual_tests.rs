//! Orb color conditions read existing priest spec passives, not toy-created helper auras.
use super::*;
use crate::class_progression::{ActiveSpec, KnownSpells};
use crate::spell_info::ClassSpells;
use shared::components::UnitLevel;

const ORB: u32 = 136928;
const ORB_SPELL: u32 = 127850;
const PRIEST: u8 = 5;
const SOURCE_LEVEL: u8 = 80;
const ORB_DURATION_MS: u32 = 300_000;
const ORB_RECOVERY_MS: u32 = 600_000;
const AFFINITIES: &[(u32, u32)] = &[(257, 137031), (256, 137032), (258, 137033)];

fn install_orb_sources(h: &mut Harness) {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    // The lightweight cast harness intentionally strips specs from its ClassSpells.
    let (store, classes) = crate::spell_info::load_spell_data(&conn).unwrap();
    h.app.insert_resource(store).insert_resource(classes);
}

fn find_source_affinity_passive(classes: &ClassSpells, spec_id: u32, passive_id: u32) -> u32 {
    let priest = classes
        .classes
        .iter()
        .find(|class| class.class_id == PRIEST)
        .unwrap();
    let spec = priest
        .specs
        .iter()
        .find(|spec| spec.spec_id == spec_id)
        .unwrap();
    let passive = spec
        .spells
        .iter()
        .find(|spell| spell.spell_id == passive_id)
        .unwrap();
    assert!(passive.passive && passive.required_level <= u32::from(SOURCE_LEVEL));
    passive.spell_id
}

fn prepare_priest_affinity(h: &mut Harness, spec_id: u32, passive_id: u32) {
    let source =
        find_source_affinity_passive(h.app.world().resource::<ClassSpells>(), spec_id, passive_id);
    let mut known: Vec<_> = h
        .app
        .world()
        .get::<KnownSpells>(h.caster)
        .unwrap()
        .iter()
        .collect();
    known.push(source);
    h.app.world_mut().entity_mut(h.caster).insert((
        ActiveSpec(spec_id),
        KnownSpells::from_baseline(known),
        Level(SOURCE_LEVEL),
        UnitLevel(SOURCE_LEVEL),
    ));
    // Native AuraPlugin consumes Changed<KnownSpells>; no direct apply_aura or fake marker.
    h.advance(5);
    let passive = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == source)
        .expect("native source spec passive applies");
    assert_eq!(passive.source, crate::auras::AuraSource::KnownSpell);
}

#[test]
fn toy_coverage_orb_real_priest_affinities_preserve_native_passive_and_owner_contract() {
    for &(spec_id, passive_id) in AFFINITIES {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) =
            setup_for_race_class(&dir.path().join("priest-orb.redb"), 1, PRIEST);
        install_orb_sources(&mut h);
        prepare_priest_affinity(&mut h, spec_id, passive_id);
        let owner = h.caster;
        for &(_, other_passive) in AFFINITIES {
            assert_eq!(
                crate::auras::has_aura(h.app.world(), owner, other_passive),
                other_passive == passive_id
            );
        }
        learn_effect_toy(&mut h, character, ORB);
        h.select(Some(h.npc));
        h.advance(2);
        let health = *h.app.world().get::<Health>(owner).unwrap();
        let model = h.app.world().get::<ModelDisplay>(owner).copied();
        let position = *h.app.world().get::<Position>(owner).unwrap();
        assert_eq!(
            activate(&mut h, ORB)[0].error,
            None,
            "source priest spec {spec_id}"
        );
        let aura = h
            .app
            .world()
            .get::<ActiveAuras>(owner)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == ORB_SPELL)
            .unwrap();
        assert_eq!(aura.caster, owner);
        assert_eq!(aura.effects[0].amount, 0.0);
        assert_eq!(aura.duration_ms, ORB_DURATION_MS);
        assert!(aura.expires_at.is_some());
        assert!(
            h.app
                .world()
                .get::<UnitAuras>(owner)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == ORB_SPELL && aura.caster == Some(owner.to_bits()))
        );
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(h.npc)
                .is_some_and(|auras| auras.auras.iter().any(|aura| aura.spell_id == ORB_SPELL))
        );
        assert!(h.inbox.go.iter().any(|go| go.spell_id == ORB_SPELL));
        assert!(
            h.inbox.cooldowns.iter().any(|cd| cd.spell_id == ORB_SPELL
                && cd.duration_ms == ORB_RECOVERY_MS
                && !cd.is_gcd)
        );
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
            .unwrap()
            .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
                spell_id: ORB_SPELL,
            });
        h.advance(5);
        assert!(!crate::auras::has_aura(h.app.world(), owner, ORB_SPELL));
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(owner)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == ORB_SPELL)
        );
        assert!(crate::auras::has_aura(h.app.world(), owner, passive_id));
        assert_eq!(h.app.world().get::<Health>(owner), Some(&health));
        assert_eq!(h.app.world().get::<ModelDisplay>(owner).copied(), model);
        assert_eq!(h.app.world().get::<Position>(owner), Some(&position));
        assert_eq!(
            activate(&mut h, ORB)[0].error.as_deref(),
            Some("OnCooldown")
        );
    }
}

#[test]
fn toy_coverage_orb_account_ownership_does_not_bypass_priest_class_restriction() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup_for_race_class(&dir.path().join("orb-class.redb"), 1, PRIEST);
    install_orb_sources(&mut h);
    prepare_priest_affinity(&mut h, 257, 137031);
    learn_effect_toy(&mut h, character, ORB);
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(h.caster)
        .unwrap()
        .class = 8;
    assert_eq!(
        activate(&mut h, ORB)[0].error.as_deref(),
        Some("toy race/class restriction")
    );
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, ORB_SPELL));
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == ORB_SPELL));
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == ORB_SPELL));
}

#[test]
fn toy_coverage_orb_source_eligible_visual_or_amount_drift_refuses_without_effects() {
    for absent_visual in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) =
            setup_for_race_class(&dir.path().join("orb-drift.redb"), 1, PRIEST);
        install_orb_sources(&mut h);
        prepare_priest_affinity(&mut h, 257, 137031);
        learn_effect_toy(&mut h, character, ORB);
        let mut changed = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(ORB_SPELL)
            .unwrap()
            .clone();
        if absent_visual {
            changed.has_spell_visual = false;
        } else {
            changed.effects[0].base_points = 1.0;
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![changed]));
        assert_eq!(
            activate(&mut h, ORB)[0].error.as_deref(),
            Some("unsupported toy spell: aura 4")
        );
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, ORB_SPELL));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == ORB_SPELL));
        assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == ORB_SPELL));
    }
}

#[test]
fn toy_coverage_orb_census_keeps_heating_sound_filter_audience_and_dummy_gaps() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_ORB_VISUAL_REPORT");
    let rows = report["toys"].as_array().unwrap();
    for item in [ORB, 175140, 228789, 235041, 129111] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [180873, 205936, 86565, 18986] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}

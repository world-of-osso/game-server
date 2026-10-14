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
    templates.retain(|entry, _| *entry == FLESH_EATER);
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
    assert_eq!(
        enhanced,
        (baseline * 1.2).round(),
        "native aura168 mask32 must affect actual Fire damage"
    );
}

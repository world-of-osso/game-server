use super::test_harness::{Harness, harness, spawn_player};
use crate::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use crate::simc_apl::{Action, ActionFacts};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

fn fact(h: &Harness, target: Entity, spell: u32) -> f64 {
    let bindings = Bindings {
        actions: [("dispel".into(), spell)].into(),
        ..Default::default()
    };
    let targets = [h.caster, target];
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &bindings,
        Encounter::dungeon(0.0),
        0,
    )
    .with_targets(Some(h.caster), &targets);
    let action = Action {
        source_row: None,
        name: "dispel".into(),
        target: Some(target.to_bits()),
        options: Default::default(),
    };
    facts
        .value_for(&action, "target.dispellable_debuffs")
        .unwrap()
}

#[test]
fn native_healer_dispel_fact_matches_spell_masks_and_actual_purify_removal() {
    let mut h = harness(
        Vec::new(),
        &[
            527, 2782, 4987, 77130, 88423, 115450, 360823, 1294569, 2818, 191587, 980, 17, 11984,
        ],
        5,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::UnitFactionTemplate(14));
    let ally = spawn_player(&mut h.app, 5, 3.0);
    for spell_id in [1294569, 2818, 191587, 980, 11984] {
        auras::apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: h.npc,
                target: ally,
                spell_id,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: ally,
            spell_id: 17,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    // Same four concrete debuffs; capabilities are read from the dispel's DB2 effects.
    for (spell, expected) in [
        (527, 2.0),
        (2782, 2.0),
        (4987, 3.0),
        (77130, 2.0),
        (88423, 3.0),
        (115450, 3.0),
        (360823, 2.0),
    ] {
        assert_eq!(
            fact(&h, ally, spell),
            expected,
            "native dispel spell{spell}"
        );
        assert_eq!(fact(&h, h.caster, spell), 0.0, "unafflicted current target");
    }
    h.cast_and_deliver(527, Some(ally));
    assert_eq!(
        fact(&h, ally, 527),
        0.0,
        "Magic and Disease were actually removed"
    );
    assert_eq!(
        fact(&h, ally, 360823),
        1.0,
        "Poison remains eligible to Naturalize"
    );
    let remaining = h.app.world().get::<ActiveAuras>(ally).unwrap();
    assert!(
        remaining
            .iter()
            .any(|aura| aura.spell_id == 11984 && aura.passive)
    );
    for spell in [2818, 980, 17, 11984] {
        assert!(
            remaining.iter().any(|aura| aura.spell_id == spell),
            "unmatched or helpful aura{spell}"
        );
    }
    let removed: std::collections::BTreeSet<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Dispel)
        .filter_map(|event| event.extra_spell_id)
        .collect();
    assert_eq!(removed, [1294569, 191587].into());
    let auras = h.app.world().get::<ActiveAuras>(ally).unwrap();
    let after_expiry = auras
        .iter()
        .filter(|aura| aura.harmful && !aura.passive)
        .map(|aura| aura.expires_at.expect("finite fixture debuff"))
        .fold(0.0_f64, f64::max)
        + 0.01;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(360823)
        .unwrap();
    assert_eq!(
        auras::dispellable::debuff_count(auras, spell, after_expiry),
        0,
        "expired aura state cannot keep a dispel candidate eligible"
    );
}

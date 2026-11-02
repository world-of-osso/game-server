use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::{self, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn blast_damage(spell: u32, selected: bool) -> (i32, f32) {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 90);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    assert!(auras::has_aura(app.world(), actor, 373212));
    let bonus = crate::spell_cast::scripts::script_effect_value(app.world(), actor, 373212, 0);
    if !selected {
        auras::remove_auras(app.world_mut(), actor, RemoveReason::Cancelled, |aura| {
            aura.spell_id == 373212
        });
    }
    for spell_id in [589, 34914, 335467] {
        auras::apply_aura(
            app.world_mut(),
            &AuraApplication {
                caster: actor,
                target,
                spell_id,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    app.world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let _cast_seed = crate::random::seed_scope(1234);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, spell, Some(target), 0);
    let damage = app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
                && event.source == Some(actor.to_bits())
        })
        .expect("real native Blast damage")
        .amount;
    (damage, bonus)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_insidious_ire_boosts_mind_blast_and_void_blast_with_three_dots() {
    for spell in [8092, 450983] {
        let (baseline, _) = blast_damage(spell, false);
        let (buffed, percent) = blast_damage(spell, true);
        assert!(
            (buffed as f32 / baseline as f32 - (1.0 + percent / 100.0)).abs() < 0.02,
            "spell={spell} baseline={baseline} buffed={buffed} percent={percent}"
        );
    }
}

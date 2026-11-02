use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::{self, AuraApplication, AuraSource, RemoveReason, SpellModOp, spell_mods};
use crate::spell_info::SpellStore;
use shared::components::ActiveSpec;
use shared::protocol::CombatLogKind;

fn cast_damage(
    app: &mut bevy::prelude::App,
    actor: bevy::prelude::Entity,
    target: bevy::prelude::Entity,
    spell: u32,
) -> i32 {
    let _seed = crate::random::seed_scope(70448);
    let events = &mut app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    events.clear();
    crate::spell_cast::cast_triggered(app.world_mut(), actor, spell, Some(target), 0);
    app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
                && event.source == Some(actor.to_bits())
        })
        .expect("actual spell damage")
        .amount
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn overwhelming_shadows_modifies_only_the_selected_specialization_spells() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    let blast = app
        .world()
        .resource::<SpellStore>()
        .get(8092)
        .unwrap()
        .clone();
    let void_blast = app
        .world()
        .resource::<SpellStore>()
        .get(450983)
        .unwrap()
        .clone();
    let torrent = app
        .world()
        .resource::<SpellStore>()
        .get(263165)
        .unwrap()
        .clone();

    for (spec, expected_blast, expected_torrent) in [(258, 1.0, 1.3), (256, 2.0, 1.0)] {
        app.world_mut().entity_mut(actor).insert(ActiveSpec(spec));
        auras::remove_auras(app.world_mut(), actor, RemoveReason::Cancelled, |aura| {
            aura.spell_id == 1266883
        });
        let amount = |world: &bevy::prelude::World, spell, op| {
            spell_mods::modify(world, actor, spell, op, 100.0)
        };
        let before = [
            amount(app.world(), &blast, SpellModOp::HealingAndDamage),
            amount(app.world(), &void_blast, SpellModOp::HealingAndDamage),
            amount(app.world(), &torrent, SpellModOp::PeriodicHealingAndDamage),
        ];
        let damage_before = cast_damage(&mut app, actor, target, 450983);
        auras::apply_aura(
            app.world_mut(),
            &AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 1266883,
                effect_mask: u32::MAX,
                source: AuraSource::KnownSpell,
            },
        );
        let after = [
            amount(app.world(), &blast, SpellModOp::HealingAndDamage),
            amount(app.world(), &void_blast, SpellModOp::HealingAndDamage),
            amount(app.world(), &torrent, SpellModOp::PeriodicHealingAndDamage),
        ];
        let damage_after = cast_damage(&mut app, actor, target, 450983);
        assert!(
            (damage_after as f32 / damage_before as f32 - expected_blast).abs() < 0.02,
            "spec={spec} damage_before={damage_before} damage_after={damage_after}"
        );
        for (spell, (base, modified), expected) in [
            (8092, (before[0], after[0]), expected_blast),
            (450983, (before[1], after[1]), expected_blast),
            (263165, (before[2], after[2]), expected_torrent),
        ] {
            assert!(
                (modified / base - expected).abs() < 0.001,
                "spec={spec} spell={spell} base={base} modified={modified}"
            );
        }
    }
}

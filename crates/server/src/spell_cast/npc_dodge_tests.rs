//! TC a352b1fa Unit.cpp2647–2717/2750–2785: NPC dodge includes rear attacks.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use bevy::prelude::*;
use shared::components::{Npc, Rotation, UnitFactionTemplate, UnitLevel};
use shared::protocol::{CombatLogKind, MissKind};

const STRIKE: u32 = 38598;
const CLOAK_OF_DARKNESS: u32 = 70188;

fn spawn_native_dodge_target(facing: f32) -> Harness {
    let mut h = harness_with(Vec::new(), &[STRIKE, CLOAK_OF_DARKNESS], 4, true).unwrap();
    let native = load_native_creature_world(&[40]);
    let template = &native.templates[&40];
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: 40,
            name: template.name.clone(),
        },
        UnitFactionTemplate(u32::from(template.faction)),
        UnitLevel(10),
        Rotation {
            x: 0.0,
            y: facing + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    h.app.insert_resource(native);
    h.advance(2);
    assert!(super::targets::is_hostile(h.app.world(), h.caster, h.npc));
    let target = h.npc;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: target,
            target,
            spell_id: CLOAK_OF_DARKNESS,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(2);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(target)
            .unwrap()
            .modifier(49),
        100.0
    );
    h
}

#[test]
fn class_completeness_native_npc_dodge_defends_front_and_rear_weapon_spells() {
    // Target at +X faces the caster with pi, or away with zero.
    for facing in [std::f32::consts::PI, 0.0] {
        let mut h = spawn_native_dodge_target(facing);
        let before = h.health(h.npc);
        h.inbox.log.clear();
        let (caster, target) = (h.caster, h.npc);
        super::creature_cast(h.app.world_mut(), caster, STRIKE, Some(target), true).unwrap();
        h.advance(2);
        assert_eq!(
            h.health(target),
            before,
            "native NPC dodge admitted damage: {:?}",
            h.inbox.log
        );
        assert!(
            h.inbox.log.iter().any(|event| {
                event.spell_id == Some(STRIKE)
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Miss(MissKind::Dodge)
            }),
            "native owner received no dodge: {:?}",
            h.inbox.log
        );
    }
}

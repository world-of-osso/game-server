use super::dps_parity::literal_rotation::execution::resolve_action;
use crate::auras::{AuraApplication, AuraSource};

#[test]
#[ignore = "requires native Preservation profile and canonical DB"]
fn merithra_override_is_consumed_by_the_actual_cast() {
    let (profile, traits) = super::dps_parity::read_reference(1468).unwrap();
    let mut run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 70 },
    )
    .unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let actor = run.actor;
    let apply = |world: &mut bevy::prelude::World| {
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 1256579,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        )
    };
    apply(run.app.world_mut());
    assert_eq!(
        resolve_action(run.app.world(), actor, 366155).unwrap(),
        Some(1256581)
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1256581, Some(actor), false)
        .unwrap();
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1256579));
    assert_eq!(
        resolve_action(run.app.world(), actor, 366155).unwrap(),
        Some(366155)
    );
    apply(run.app.world_mut());
    assert!(
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1256581, Some(actor), false)
            .is_err()
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1256579),
        "a rejected cast must not consume the next-use aura"
    );
}

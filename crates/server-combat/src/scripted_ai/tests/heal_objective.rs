use super::*;

#[test]
fn temple_avatar_healing_to_full_completes_bound_encounter() {
    let (mut h, avatar, map) = bound_boss_at_difficulty(133392, 1877, 3, 8, None);
    h.app
        .world_mut()
        .resource_mut::<MapStore>()
        .encounter_heal_objectives
        .insert((1877, 133392), 25.0);
    crate::scripted_ai::heal_objective::initialize(h.app.world_mut(), avatar);
    let max = h.health(avatar).max;
    assert!((h.health(avatar).current - max * 0.25).abs() < 1.0);
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let ctx = EffectCtx {
        caster: h.player(),
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, avatar, max * 0.5);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & (1 << 3),
        0
    );
    crate::spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, avatar, max);
    assert_eq!(h.health(avatar).current, max);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & (1 << 3),
        1 << 3
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(avatar)
            .is_none()
    );
}

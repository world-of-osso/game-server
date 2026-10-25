//! Native Effect188: summon a stored hunter pet as a temporary guardian.
use super::*;

pub(crate) const EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN: u32 = 188;

#[derive(Component)]
pub(super) struct StabledPetGuardian {
    expires_at: Option<f64>,
}

pub(crate) fn effect_summon_stabled_pet(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    // SimC76f8118 sc_hunter.cpp6702–6703: Nature's Ally uses the stable-pet
    // body, with its native summon duration. No alternate creature identity.
    let Some(main) = summoned_pet(world, ctx.caster) else {
        return;
    };
    let Some(slot) = world.get::<HunterPet>(main).map(|pet| pet.slot) else {
        return;
    };
    let Some((at, facing)) = close_point(world, ctx.caster) else {
        return;
    };
    let count =
        crate::spell_cast::effects::modded_base_points(world, ctx.caster, ctx.spell, effect) as u32;
    let duration = crate::spell_cast::duration_ms(world, ctx.caster, ctx.spell);
    let expires_at =
        (duration > 0).then(|| crate::auras::aura_now(world) + f64::from(duration) / 1000.0);
    for _ in 0..count {
        spawn_stabled_guardian(world, ctx, slot, at, facing, expires_at);
    }
}

fn spawn_stabled_guardian(
    world: &mut World,
    ctx: &EffectCtx,
    slot: u8,
    at: Position,
    facing: f32,
    expires_at: Option<f64>,
) {
    let Some((pet, stored)) = spawn_pet(world, ctx.caster, slot, at, facing) else {
        error!(owner=?ctx.caster, spell=ctx.spell.id, "Native stabled-pet guardian spawn failed");
        return;
    };
    // Stable guardians have melee and commanded actions, not the main pet's
    // autonomous Focus-spending family ability (sc_hunter.cpp2150–2155/2230–2234).
    world.entity_mut(pet).insert((
        StabledPetGuardian { expires_at },
        crate::pet_control::PetCharm::new(Default::default(), stored.react_state),
    ));
    if let Some(mut health) = world.get_mut::<Health>(pet) {
        health.current = health.max;
    }
    if let Some(target) = ctx.spell.cast_primary_target {
        crate::pet_control::attack_start(world, pet, target);
    }
    crate::spell_cast::log_summon(world, ctx.caster, pet, ctx.spell);
}

pub(super) fn remove_expired_guardians(world: &mut World) {
    let now = crate::auras::aura_now(world);
    let expired: Vec<_> = world
        .query::<(Entity, &StabledPetGuardian, &Health)>()
        .iter(world)
        .filter(|(_, guardian, health)| {
            health.current <= 0.0 || guardian.expires_at.is_some_and(|expires| expires <= now)
        })
        .map(|(pet, _, _)| pet)
        .collect();
    for pet in expired {
        // A guardian is not PetStable.summoned: do not use remove_pet, which
        // persists/dismisses the owner's actual current stable pet.
        crate::combat::combat_stop(world, pet);
        world.entity_mut(pet).despawn();
    }
}

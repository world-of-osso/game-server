//! Native Aura493 and Effect260: a stable companion bound to the current pet.
use super::*;

pub(crate) const EFFECT_SUMMON_STABLED_PET: u32 = 260;
const AURA_MOD_SUMMON_PET: u32 = 493;

#[derive(Component)]
pub(super) struct StableCompanion {
    pub main: Entity,
}

pub(super) fn trigger_summon_auras(world: &mut World, owner: Entity) {
    // SimC76f8118 sc_hunter.cpp2373–2377: Call Pet also summons the
    // selected Animal Companion. Native Aura493 supplies its trigger spell.
    let triggers: Vec<_> = world
        .get::<crate::auras::ActiveAuras>(owner)
        .into_iter()
        .flat_map(|auras| auras.iter())
        .flat_map(|aura| &aura.effects)
        .filter(|effect| effect.aura_type == AURA_MOD_SUMMON_PET && effect.trigger_spell != 0)
        .map(|effect| effect.trigger_spell)
        .collect();
    for trigger in triggers {
        crate::spell_cast::cast_triggered(world, owner, trigger, Some(owner), 1);
    }
}

pub(crate) fn effect_summon_companion(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    let Some(main) = summoned_pet(world, ctx.caster) else {
        return;
    };
    if companion_of(world, main).is_some() {
        return;
    }
    let Some(slot) = world.get::<HunterPet>(main).map(|pet| pet.slot) else {
        return;
    };
    let Some((at, facing)) = close_point(world, ctx.caster) else {
        return;
    };
    spawn_companion(world, ctx, main, slot, at, facing);
}

fn spawn_companion(
    world: &mut World,
    ctx: &EffectCtx,
    main: Entity,
    slot: u8,
    at: Position,
    facing: f32,
) {
    let Some((pet, stored)) = spawn_pet(world, ctx.caster, slot, at, facing) else {
        error!(owner=?ctx.caster, spell=ctx.spell.id, "Native stable companion spawn failed");
        return;
    };
    // sc_hunter.cpp2150–2155/2230–2234: only the main pet has a
    // Focus-spending basic attack; the companion has melee and owner commands.
    world.entity_mut(pet).insert((
        StableCompanion { main },
        PetFocusRegen::Disabled,
        crate::pet_control::PetCharm::new(Default::default(), stored.react_state),
    ));
    if let Some(mut health) = world.get_mut::<Health>(pet) {
        health.current = health.max;
    }
    crate::spell_cast::log_summon(world, ctx.caster, pet, ctx.spell);
}

fn companion_of(world: &mut World, main: Entity) -> Option<Entity> {
    world
        .query::<(Entity, &StableCompanion)>()
        .iter(world)
        .find(|(_, companion)| companion.main == main)
        .map(|(pet, _)| pet)
}

pub(super) fn remove_companion(world: &mut World, main: Entity) {
    // sc_hunter.cpp2383–2394: the companion's lifetime ends with its main pet.
    if let Some(pet) = companion_of(world, main) {
        crate::combat::combat_stop(world, pet);
        world.entity_mut(pet).despawn();
    }
}

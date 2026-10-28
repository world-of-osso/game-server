//! Avenger's Shield grants Strength in Adversity for its actual targets.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

const TALENT: u32 = 393071;
const BUFF: u32 = 393038;

#[derive(Component)]
struct LatestShield(Option<Entity>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeNativeOnCast(reset_strength),
    SpellScriptHook::AfterResolvedHits(after_shield),
];

fn reset_strength(
    world: &mut World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
    _depth: u8,
) {
    if spell.id == 31935 {
        world
            .entity_mut(caster)
            .insert(LatestShield(spell.cast_primary_target));
        auras::remove_auras(world, caster, RemoveReason::Replaced, |aura| {
            aura.spell_id == BUFF && aura.caster == caster
        });
    }
}

fn after_shield(world: &mut World, ctx: &EffectCtx<'_>) {
    if ctx.spell.id != 31935
        || !world
            .get::<LatestShield>(ctx.caster)
            .is_some_and(|latest| latest.0 == ctx.primary_target())
        || !world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 66)
        || !world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
    {
        return;
    }
    // SimC76f8118 sc_paladin_protection.cpp283–291: replace the previous
    // value with the number of Shield targets. DB2 393038 supplies the real
    // 15-second, maximum-five-stack parry aura instead of SimC's dummy buff.
    let hits = spell_cast::resolving_damage(world, ctx)
        .expect("Avenger's Shield resolved hits")
        .len();
    for _ in 0..hits {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster: ctx.caster,
                target: ctx.caster,
                spell_id: BUFF,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}

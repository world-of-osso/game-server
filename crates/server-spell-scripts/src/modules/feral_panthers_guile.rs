//! SimC76f8118 sc_druid.cpp3862–3910: Panther's Guile fills combo points on eligible gains.
use bevy::prelude::*;
use shared::components::PowerType;

use super::combat;
use combat::auras;
use combat::random::Rng;
use combat::spell_cast::{effects::EffectCtx, powers, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const TALENT: u32 = 1280316;

#[derive(Component)]
struct PanthersGuileReady(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterComboGain(on_combo_gain)];

fn on_combo_gain(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, gained: f32) {
    if target != ctx.caster
        || gained <= 0.0
        || !matches!(ctx.spell.id, 5221 | 155625)
        || !auras::has_aura(world, ctx.caster, TALENT)
    {
        return;
    }
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world
        .get::<PanthersGuileReady>(ctx.caster)
        .is_some_and(|ready| now < ready.0)
    {
        return;
    }
    let (chance, cooldown) = {
        let store = world.resource::<SpellStore>();
        let options = store
            .get(TALENT)
            .expect("selected Panther's Guile must exist")
            .aura_options
            .as_ref()
            .expect("Panther's Guile proc data");
        (
            options.proc_chance as f32 / 100.0,
            options.proc_category_recovery_ms as f64 / 1000.0,
        )
    };
    if combat::random::thread_rng().gen_range(0.0..1.0) >= chance {
        return;
    }
    let (current, max) =
        powers::power_value(world, ctx.caster, PowerType::ComboPoints).expect("Feral combo points");
    let fill = max - current + super::feral_overflow::extra_max(world, ctx.caster);
    if fill <= 0.0 {
        return;
    }
    world
        .entity_mut(ctx.caster)
        .insert(PanthersGuileReady(now + cooldown));
    powers::change_power(world, ctx.caster, PowerType::ComboPoints, fill);
}

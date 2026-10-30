//! Mograine's real Death and Decay and the owner's Might window.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, summon::SummonedCreature,
};
use combat::spell_info::SpellStore;

#[derive(Component)]
struct Decay {
    next: f64,
    ends: f64,
    period: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AdditionalReachActions(autonomous_actions),
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::SummonInitialized(initialize),
    SpellScriptHook::AfterCastUpdate(update),
    SpellScriptHook::AfterPolicyDirectDamage(extend_diseases),
];

fn autonomous_actions(world: &World, actor: Entity) -> Vec<u32> {
    if world.get::<Decay>(actor).is_some() {
        vec![1251951]
    } else {
        Vec::new()
    }
}

fn declared_reach(
    _world: &World,
    _caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if spell.id != 1251951 {
        return Default::default();
    }
    combat::spell_cast::script_registry::ScriptReach::caster(combat::spell_cast::targets::radius(
        &spell.effects[0],
        1,
    ))
}

fn initialize(world: &mut World, actor: Entity) {
    let Some(owner) = world
        .get::<SummonedCreature>(actor)
        .filter(|summon| summon.spell_id == 444248)
        .map(|summon| summon.owner)
    else {
        return;
    };
    let spell = world
        .resource::<SpellStore>()
        .get(444474)
        .expect("native Mograine Death and Decay");
    let duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("native decay duration")
        .duration_ms;
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .expect("native decay tick")
        .aura_period_ms;
    assert!(
        duration > 0 && period > 0,
        "native decay timing must be positive"
    );
    let now = combat::auras::aura_now(world);
    let duration = f64::from(duration) / 1000.0;
    world.entity_mut(actor).insert(Decay {
        next: now,
        ends: now + duration,
        period: f64::from(period) / 1000.0,
    });
    if !combat::auras::has_aura(world, owner, 444047) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp4977–4989/5038–5043: Might lasts a
    // normal sample (mean 65% of native decay, sigma 2s), bounded above by decay.
    let sample = loop {
        let u1 = combat::random::random::<f64>().max(f64::MIN_POSITIVE);
        let u2 = combat::random::random::<f64>();
        let value =
            duration * 0.65 + 2.0 * (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        if value <= duration {
            break value;
        }
    };
    if sample <= 0.0 {
        return;
    }
    for (spell, seconds) in [(444505, sample), (188290, sample + 4.0)] {
        if !spell_cast::try_cast_triggered(world, owner, spell, Some(owner), 1) {
            warn!(?owner, spell, "Mograine's Might owner aura refused");
            continue;
        }
        assert!(
            combat::auras::set_owned_duration(
                world,
                owner,
                owner,
                spell,
                (seconds * 1000.0).round() as u32
            ),
            "Mograine owner aura must exist"
        );
    }
}

fn update(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let due: Vec<_> = world
        .query::<(Entity, &mut Decay)>()
        .iter_mut(world)
        .filter_map(|(actor, mut decay)| {
            if now < decay.next || decay.next >= decay.ends {
                return None;
            }
            decay.next += decay.period;
            Some(actor)
        })
        .collect();
    for actor in due {
        // SimC76f8118 sc_death_knight.cpp4927–4970: tick immediately and
        // at the native interval, from the actual Mograine actor.
        if !spell_cast::try_cast_triggered(world, actor, 1251951, Some(actor), 1) {
            warn!(?actor, "Mograine Death and Decay damage refused");
        }
    }
}

fn extend_diseases(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.spell.id != 1251951 {
        return;
    }
    let Some(owner) = world
        .get::<SummonedCreature>(ctx.caster)
        .filter(|summon| summon.spell_id == 444248)
        .map(|summon| summon.owner)
    else {
        return;
    };
    let milliseconds = spell_cast::scripts::script_effect_value(world, owner, 444005, 1) as u32;
    // SimC76f8118 sc_death_knight.cpp4938–4953 extends each existing owned disease.
    combat::auras::extend_owned_duration(world, owner, target, &[191587, 1240996], milliseconds);
}

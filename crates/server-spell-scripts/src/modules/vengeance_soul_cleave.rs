//! Soul Cleave consumes actual fragments and earns Untethered Rage.
use super::combat;
use bevy::prelude::*;
use combat::random::Rng;
use combat::soul_fragments::{self, FragmentFilter};
use combat::spell_cast::{
    self,
    script_registry::{HitHook, SpellScriptHook},
};
use combat::spell_info::SpellInfo;

struct PendingCleave {
    at: f64,
    target: Entity,
    multiplier: f32,
}
#[derive(Component, Default)]
struct PendingCleaves(Vec<PendingCleave>);

#[derive(Component)]
struct UntetheredMetaDuration(u32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectEnabled(disable_native_child),
    SpellScriptHook::AfterNativeLaunch(launch),
    SpellScriptHook::AfterNativeCastUpdate(update),
    SpellScriptHook::BeforeNativeOnCast(snapshot_meta),
    SpellScriptHook::AfterResolvedHits(finish_meta),
    SpellScriptHook::EffectHit(sigil_extension_hook),
    SpellScriptHook::ActualDamageMultiplier(actual_focused_cleave),
    SpellScriptHook::ExpectedDamageMultiplier(focused_cleave),
];

fn actual_focused_cleave(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx<'_>,
    target: Entity,
) -> f32 {
    focused_cleave(world, ctx.caster, target, ctx.spell)
}
fn focused_cleave(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != 228478
        || spell.cast_primary_target != Some(target)
        || !selected(world, caster, 343207)
    {
        return 1.0;
    }
    // SimC76f8118 sc_demon_hunter.cpp8179–8185 applies Focused Cleave only to the primary target.
    1.0 + spell_cast::scripts::script_effect_value(world, caster, 343207, 0) / 100.0
}

fn sigil_extension_hook(spell: u32, index: u8) -> Option<HitHook> {
    (spell == 228478 && index == 0).then_some(extend_sigil)
}

fn extend_sigil(world: &mut World, caster: Entity, target: Entity, _: u8) {
    if !combat::auras::has_aura(world, caster, 1296613) {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp8145–8175: the equipped MID2 2pc
    // extends the target's active Sigil of Flame on each Soul Cleave hit.
    let milliseconds = spell_cast::scripts::script_effect_value(world, caster, 1296613, 1);
    assert!(
        milliseconds >= 0.0 && milliseconds.fract() == 0.0,
        "invalid MID2 Sigil extension"
    );
    combat::auras::extend_owned_duration(world, caster, target, &[204598], milliseconds as u32);
}

fn selected(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == spell)
        })
}

fn disable_native_child(_: &World, _: Entity, spell: u32, index: u8) -> bool {
    !(spell == 228477 && index == 1)
}

fn launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 228477 {
        return;
    }
    let target = spell
        .cast_primary_target
        .expect("native Soul Cleave target");
    let effect = |index| {
        spell
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native Soul Cleave effect")
    };
    let maximum = spell_cast::scripts::script_effect_value(world, caster, 228477, 1)
        + spell_cast::scripts::script_effect_value(world, caster, 228477, 2);
    assert!(
        maximum >= 0.0 && maximum.fract() == 0.0,
        "invalid native Soul Cleave fragment cap"
    );
    let delay = effect(1).misc_values[0];
    assert!(delay >= 0, "invalid native Soul Cleave delay");
    let percent = spell_cast::scripts::script_effect_value(world, caster, 228477, 0);
    let consumed =
        soul_fragments::consume_active(world, caster, FragmentFilter::Any, maximum as usize, true)
            .expect("native Soul Cleave fragment consumption");
    trigger_untethered_rage(world, caster, consumed);
    // SimC76f8118 sc_demon_hunter.cpp8190–8236: native heal remains on
    // the parent; the separately scaled damage child lands at effect2's delay.
    let pending = PendingCleave {
        at: spell_cast::now(world) + f64::from(delay) / 1000.0,
        target,
        multiplier: 1.0 + percent * consumed as f32 / 100.0,
    };
    world
        .entity_mut(caster)
        .entry::<PendingCleaves>()
        .or_default()
        .into_mut()
        .0
        .push(pending);
}

pub(super) fn trigger_untethered_rage(world: &mut World, caster: Entity, consumed: usize) {
    if consumed == 0 || !selected(world, caster, 1270444) {
        return;
    }
    let stacks = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 1270547 && aura.caster == caster)
                .map(|aura| aura.stacks)
        })
        .unwrap_or(0);
    // SimC76f8118 sc_demon_hunter.cpp2623–2640 supplies this proc curve;
    // the simulator notes that the game has no authored chance data for it.
    let chance = consumed as f64 * 0.0075 * 1.35_f64.powi(i32::from(stacks));
    if combat::random::thread_rng().r#gen::<f64>() < chance {
        combat::auras::remove_auras(
            world,
            caster,
            combat::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 1270547 && aura.caster == caster,
        );
        if !spell_cast::try_cast_triggered(world, caster, 1270476, Some(caster), 1) {
            error!(?caster, "Untethered Rage proc refused");
        }
    } else if !combat::auras::has_aura(world, caster, 1270476) && selected(world, caster, 1270449) {
        if !spell_cast::try_cast_triggered(world, caster, 1270547, Some(caster), 1) {
            error!(?caster, "Seething Anger stack refused");
        }
    }
}

fn snapshot_meta(world: &mut World, caster: Entity, spell: &SpellInfo, _: u8) {
    if spell.id != 187827 || !combat::auras::has_aura(world, caster, 1270476) {
        return;
    }
    let now = spell_cast::now(world);
    let remaining = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 187827 && aura.caster == caster)
        })
        .and_then(|aura| aura.expires_at)
        .map_or(0.0, |expiry| (expiry - now).max(0.0));
    let seconds = world
        .resource::<combat::spell_info::SpellStore>()
        .get(1270444)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
        .expect("native Untethered Rage duration")
        .base_points;
    assert!(seconds > 0.0, "invalid native Untethered Rage duration");
    world.entity_mut(caster).insert(UntetheredMetaDuration(
        ((remaining + f64::from(seconds)) * 1000.0).round() as u32,
    ));
}

fn finish_meta(world: &mut World, ctx: &spell_cast::effects::EffectCtx) {
    if ctx.spell.id != 187827 {
        return;
    }
    let Some(duration) = world
        .entity_mut(ctx.caster)
        .take::<UntetheredMetaDuration>()
    else {
        return;
    };
    assert!(
        combat::auras::set_owned_duration(world, ctx.caster, ctx.caster, 187827, duration.0),
        "native Metamorphosis aura absent"
    );
    combat::auras::remove_auras(
        world,
        ctx.caster,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 1270476 && aura.caster == ctx.caster,
    );
}

fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let casters: Vec<_> = world
        .query_filtered::<Entity, With<PendingCleaves>>()
        .iter(world)
        .collect();
    for caster in casters {
        let due = {
            let mut pending = world
                .get_mut::<PendingCleaves>(caster)
                .expect("Soul Cleave queue");
            let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut pending.0)
                .into_iter()
                .partition(|cleave| cleave.at <= now);
            pending.0 = later;
            due
        };
        for cleave in due {
            if !spell_cast::cast_triggered_with_overrides(
                world,
                caster,
                228478,
                Some(cleave.target),
                1,
                spell_cast::TriggeredSpellOverrides {
                    damage_multiplier: Some(cleave.multiplier),
                    ..Default::default()
                },
            ) {
                error!(?caster, "native Soul Cleave child refused");
            }
        }
    }
}

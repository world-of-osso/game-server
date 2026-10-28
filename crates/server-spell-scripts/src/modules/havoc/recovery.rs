//! SimC76f8118 sc_demon_hunter.cpp4002–4005,4033–4047,4987–4998,7814–7833.
use super::{combat, havoc::has_talent};
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance, scripts::AuraScripts};
use combat::spell_cast::{
    SpellHistory,
    script_registry::{Policy, Relationship, SpellScriptHook},
    scripts::AfterCastHook,
};
use combat::spell_info::{SpellInfo, SpellStore};

const BEAMS: &[u32] = &[198013, 452497];
const CYCLE: u32 = 1214887;
const CYCLE_TALENT: u32 = 258887;
const FURIOUS_GAZE: u32 = 343312;
const GAZE_TALENT: u32 = 343311;
// SimC76f8118 sc_demon_hunter.cpp4984–4985: hard Meta adds its
// native duration to an already-live transformation instead of replacing it.
const POLICIES: &[Policy] = &[Policy {
    name: "SIMC_HAVOC_META_REAPPLICATION_EXTENDS",
    spec: 577,
    talent: 191427,
    relationship: Relationship::ReapplicationExtend { payload: 162264 },
}];
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::Policies(POLICIES),
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::LateAuraRegistration(register_channels),
    SpellScriptHook::AfterAuraApplication(apply_serrated_glaive),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        191427 => Some(reset_meta_actions),
        198013 | 452497 | 258860 => Some(advance_cycle),
        _ => None,
    }
}

fn reset_meta_actions(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if has_talent(world, caster, 388112) {
        for action in [198013, 452497, 188499, 210152] {
            reset_action(world, caster, action, false);
        }
    }
    if has_talent(world, caster, 452409) {
        reset_action(world, caster, 258920, true);
        reset_action(world, caster, 204596, false);
    }
}

pub(super) fn reset_action(world: &mut World, caster: Entity, id: u32, all_charges: bool) {
    let spell = world
        .resource::<SpellStore>()
        .get(id)
        .expect("native Havoc reset action");
    let category = spell
        .categories
        .as_ref()
        .map(|category| (category.charge_category, category.max_charges));
    let remaining = world.get::<SpellHistory>(caster).map_or(0, |history| {
        history.remaining_cooldown_ms(spell, combat::spell_cast::now(world))
    });
    combat::spell_cast::scripts::reduce_cooldown(world, caster, id, remaining as f64 / 1000.0);
    if let Some((category, maximum)) = category.filter(|(category, _)| *category != 0) {
        let count = if all_charges { maximum } else { 1 };
        combat::spell_cast::restore_charges(
            world,
            caster,
            category,
            u8::try_from(count).expect("native Havoc charge count"),
        );
    }
}

fn advance_cycle(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let four_piece = super::havoc::owned_passive_instance(world, caster, 1296612).is_some();
    if !has_talent(world, caster, CYCLE_TALENT) || (spell.id == 258860 && !four_piece) {
        return;
    }
    let stacks = read_cycle_stacks(world, caster);
    let seconds = f64::from(combat::spell_cast::scripts::script_effect_value(
        world,
        caster,
        CYCLE_TALENT,
        0,
    )) / 1000.0;
    // SimC4033–4047/7814–7833 reads existing stacks before earning the next.
    combat::spell_cast::scripts::reduce_cooldown(
        world,
        caster,
        spell.id,
        seconds * f64::from(stacks),
    );
    if four_piece {
        reduce_paired_recovery(world, caster, spell.id, stacks, seconds);
    }
    combat::spell_cast::cast_triggered(world, caster, CYCLE, Some(caster), 1);
}

fn read_cycle_stacks(world: &World, caster: Entity) -> u8 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == CYCLE && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks)
}

fn reduce_paired_recovery(world: &mut World, caster: Entity, spell: u32, stacks: u8, seconds: f64) {
    let maximum = world
        .resource::<SpellStore>()
        .get(CYCLE)
        .expect("native Cycle buff")
        .aura_options
        .as_ref()
        .expect("native Cycle stack cap")
        .cumulative_aura;
    if stacks == 0 || u32::from(stacks) >= maximum {
        return;
    }
    // Eye/Gaze share native category1582; reduce that one cooldown only once.
    let paired = if spell == 258860 { 198013 } else { 258860 };
    combat::spell_cast::scripts::reduce_cooldown(world, caster, paired, seconds);
}

fn register_channels(scripts: &mut AuraScripts) {
    scripts.0.entry(427794).or_default().on_periodic = Some(update_never_say_die);
    for beam in BEAMS {
        let script = scripts.0.entry(*beam).or_default();
        script.on_apply = Some(super::havoc_fury::apply_blind_fury);
        script.on_natural_end = Some(finish_beam);
        script.on_remove = Some(remove_beam);
    }
}

// SimC76f8118 sc_demon_hunter.cpp2197–2199,10044–10066: the native
// periodic driver enables the damage modifier at the talent's health threshold.
// The source does not expire an already-active damage buff below that threshold.
fn update_never_say_die(world: &mut World, caster: Entity, aura: &AuraInstance, index: u8) {
    const TALENT: u32 = 427794;
    if index != 0 || aura.caster != caster || !has_talent(world, caster, TALENT) {
        return;
    }
    let threshold = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 2);
    let health = world
        .get::<shared::components::Health>(caster)
        .expect("Never Say Die caster health");
    if health.current * 100.0 < health.max * threshold {
        return;
    }
    let amount = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    for effect in [3, 4] {
        assert!(
            combat::auras::set_effect_amount(world, caster, aura.instance_id, effect, amount),
            "native Never Say Die damage modifier must exist"
        );
    }
}

// SimC76f8118 sc_demon_hunter.cpp2195–2196,7534–7539: the earned native
// buff's zero-valued modifier takes its amount from the selected talent.
fn apply_serrated_glaive(world: &mut World, caster: Entity, aura: &AuraInstance) {
    const TALENT: u32 = 390154;
    if aura.spell_id != 390155 || aura.caster != caster || !has_talent(world, caster, TALENT) {
        return;
    }
    let amount = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    assert!(
        combat::auras::set_effect_amount(world, caster, aura.instance_id, 0, amount),
        "native Serrated Glaive modifier must exist"
    );
}

// SimC76f8118 sc_demon_hunter.cpp3305,4002–4011 permits the final two
// scheduled ticks to be clipped without losing the channel's earned effects.
fn remove_beam(
    world: &mut World,
    caster: Entity,
    aura: &AuraInstance,
    reason: combat::auras::RemoveReason,
) {
    let interrupted = reason == combat::auras::RemoveReason::ChannelEnded
        && aura.expires_at.is_some_and(|expiry| combat::auras::aura_now(world) < expiry);
    if aura.caster == caster && interrupted {
        let remaining = combat::auras::count_remaining_ticks(aura)
            .expect("native Beam channel has a bounded tick schedule");
        if remaining <= 2 {
            finish_beam(world, caster, aura);
        }
    }
    super::havoc_eternal::remove_beam(world, caster, aura, reason);
}

fn finish_beam(world: &mut World, caster: Entity, aura: &AuraInstance) {
    if aura.caster == caster {
        super::havoc::beam::finish_final_breath(world, caster);
        super::havoc_eternal::finish_beam(world, caster);
    }
    if aura.caster == caster && has_talent(world, caster, GAZE_TALENT) {
        combat::spell_cast::cast_triggered(world, caster, FURIOUS_GAZE, Some(caster), 1);
    }
}

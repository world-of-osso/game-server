//! SimC76f8118 sc_druid.cpp3351–3420: MID2 Clarity follows Eclipse progress.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};

const FOUR_PIECE: u32 = 1296604;
const CLARITY: u32 = 1301768;
const ECLIPSES: [u32; 2] = [48517, 48518];

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(CLARITY).or_default().on_periodic = Some(update_clarity);
    for eclipse in ECLIPSES {
        scripts.0.entry(eclipse).or_default().on_remove = Some(end_eclipse);
    }
}

pub(super) fn begin_eclipse(world: &mut World, caster: Entity) {
    if !auras::has_aura(world, caster, FOUR_PIECE) {
        return;
    }
    let maximum = script_effect_value(world, caster, CLARITY, 2);
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        CLARITY,
        Some(caster),
        1,
        spell_cast::TriggeredSpellOverrides {
            // VERIFIED_WCL fgXxRTNdyPqvcLab fight13 actor5: Clarity survives
            // its 10s DB2 duration and ends with Eclipse at8784232/8843108ms.
            // SimC sc_druid.cpp3365–3368 likewise ties lifetime to Eclipse.
            fixed_aura_duration_ms: Some(0),
            aura_stacks: Some(maximum as u8),
            ..Default::default()
        },
    );
    if let Some(instance) = clarity_instance(world, caster) {
        set_bonus(world, caster, instance, maximum);
    }
}

fn clarity_instance(world: &World, caster: Entity) -> Option<u32> {
    world.get::<ActiveAuras>(caster)?.iter().find_map(|aura| {
        (aura.spell_id == CLARITY && aura.caster == caster).then_some(aura.instance_id)
    })
}

fn eclipse_bonus(aura: &AuraInstance, now: f64, minimum: f64, maximum: f64) -> f64 {
    let Some(end) = aura.expires_at.filter(|end| *end > now) else {
        return 0.0;
    };
    let duration = f64::from(aura.duration_ms) / 1000.0;
    let elapsed = (now - (end - duration)).round();
    let total = duration.round();
    let center = ((total - 1.0) * 0.5).max(1.0);
    let distance = elapsed.min(total - 1.0 - elapsed);
    (maximum - (maximum - minimum) * distance / center)
        .ceil()
        .clamp(minimum, maximum)
}

fn current_bonus(world: &World, caster: Entity) -> f32 {
    let minimum = f64::from(script_effect_value(world, caster, CLARITY, 3));
    let maximum = f64::from(script_effect_value(world, caster, CLARITY, 2));
    let now = spell_cast::now(world);
    world.get::<ActiveAuras>(caster).map_or(0.0, |auras| {
        auras
            .iter()
            .filter(|aura| aura.caster == caster && ECLIPSES.contains(&aura.spell_id))
            .map(|aura| eclipse_bonus(aura, now, minimum, maximum))
            .fold(0.0_f64, f64::max) as f32
    })
}

fn set_bonus(world: &mut World, caster: Entity, instance: u32, bonus: f32) {
    auras::set_self_cast_stacks(world, caster, CLARITY, bonus as u8);
    for effect in [0, 1] {
        auras::set_effect_amount(world, caster, instance, effect, bonus);
    }
}

fn update_clarity(world: &mut World, caster: Entity, aura: &AuraInstance, index: u8) {
    if index == 4 {
        let bonus = current_bonus(world, caster);
        // SimC sc_druid.cpp3397–3400 leaves expiry to the Eclipse removal.
        if bonus > 0.0 {
            set_bonus(world, caster, aura.instance_id, bonus);
        }
    }
}

fn end_eclipse(world: &mut World, caster: Entity, _: &AuraInstance, _: RemoveReason) {
    if current_bonus(world, caster) == 0.0 {
        auras::remove_auras(world, caster, RemoveReason::Expired, |aura| {
            aura.spell_id == CLARITY && aura.caster == caster
        });
    }
}

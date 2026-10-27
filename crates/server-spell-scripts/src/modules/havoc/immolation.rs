//! SimC76f8118 sc_demon_hunter.cpp8980–9040: Undying Embers aura renewal.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraInstance, AuraSource, scripts::AuraScripts};
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, queued_triggers};
use combat::spell_info::SpellStore;

const IMMOLATION: u32 = 258920;
const UNDYING: u32 = 1272405;

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(IMMOLATION).or_default().on_natural_end = Some(renew_immolation);
}

fn renew_immolation(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit || !super::has_talent(world, unit, UNDYING) {
        return;
    }
    let chance = spell_cast::scripts::script_effect_value(world, unit, UNDYING, 0) / 100.0;
    let roll = combat::random::thread_rng().r#gen::<f32>();
    if std::env::var_os("IMMOLATION_TRACE").is_some() {
        eprintln!("EMBER roll={roll} chance={chance}");
    }
    if roll >= chance {
        return;
    }
    let spell = world
        .resource::<SpellStore>()
        .get(IMMOLATION)
        .expect("native Immolation Aura")
        .clone();
    let ctx = EffectCtx {
        caster: unit,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    queued_triggers::schedule_callback(world, &ctx, unit, 300, apply_renewal);
}

fn apply_renewal(world: &mut World, caster: Entity, target: Entity, _: u8) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: IMMOLATION,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

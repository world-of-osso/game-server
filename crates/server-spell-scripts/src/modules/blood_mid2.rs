//! Blood Midnight season 2 set: Death Strike builds Blood Debt for Marrowrend.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterForegroundSpecial(on_cast)];

const TWO: u32 = 1296650;
const FOUR: u32 = 1296651;
const DEBT: u32 = 1310372;
const STRENGTH: u32 = 1300369;
const DEMISE: u32 = 1310571;

#[derive(Component, Default)]
struct DebtCooldown(f64);

fn equipped(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == spell
                && aura.caster == caster
                && matches!(aura.source, AuraSource::ItemSet(_))
        })
    })
}

fn on_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !equipped(world, caster, TWO) {
        return;
    }
    match spell.id {
        49998 => add_debt(world, caster),
        195182 => spend_debt(world, caster, spell.cast_primary_target),
        _ => {}
    }
}

fn add_debt(world: &mut World, caster: Entity) {
    let now = spell_cast::now(world);
    if world
        .get::<DebtCooldown>(caster)
        .is_some_and(|cooldown| cooldown.0 > now)
    {
        return;
    }
    let store = world.resource::<SpellStore>();
    let set = store.get(TWO).expect("Blood 2pc spell");
    let cooldown = set
        .aura_options
        .as_ref()
        .expect("Blood 2pc proc options")
        .proc_category_recovery_ms;
    let debt = store.get(DEBT).expect("Blood Debt spell");
    // SimC76f8118 sc_death_knight.cpp17115–17117: in-game strength
    // is one tenth of the native Blood Debt effect amount.
    let amount = debt
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Blood Debt strength")
        .base_points
        / 10.0;
    world
        .entity_mut(caster)
        .insert(DebtCooldown(now + f64::from(cooldown) / 1000.0));
    spell_cast::cast_triggered_with_effect0_amount(world, caster, DEBT, Some(caster), 1, amount);
}

fn spend_debt(world: &mut World, caster: Entity, target: Option<Entity>) {
    let max = world
        .resource::<SpellStore>()
        .get(DEBT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Blood Debt stacks")
        .cumulative_aura;
    let ready = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == DEBT && u32::from(aura.stacks) >= max)
    });
    if !ready {
        return;
    }
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == DEBT && aura.caster == caster
    });
    spell_cast::cast_triggered(world, caster, STRENGTH, Some(caster), 1);
    if equipped(world, caster, FOUR) {
        let count = world
            .resource::<SpellStore>()
            .get(FOUR)
            .expect("Blood 4pc spell")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("Blood 4pc bones")
            .base_points as u32;
        for _ in 0..count {
            auras::apply_aura(
                world,
                &AuraApplication {
                    caster,
                    target: caster,
                    spell_id: 195181,
                    effect_mask: u32::MAX,
                    source: AuraSource::Cast,
                },
            );
        }
        if let Some(target) = target {
            spell_cast::cast_triggered(world, caster, DEMISE, Some(target), 1);
        }
    }
}

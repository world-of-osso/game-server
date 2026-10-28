//! Blood's Crimson Scourge and standing in its real Death and Decay field.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Position};

use super::combat;
use combat::auras::{
    self, ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason, procs::AuraEvent,
    scripts::AuraScripts,
};
use combat::spell_cast::{
    self, area_trigger::SpellAreaTrigger, script_registry::SpellScriptHook, targets,
};
use combat::spell_info::SpellStore;

const SCOURGE: u32 = 81136;
const SCOURGE_BUFF: u32 = 81141;
const PLAGUE: u32 = 55078;
const DECAY: u32 = 43265;
const IN_DECAY: u32 = 188290;
const SANGUINE: u32 = 391458;
const SANGUINE_BUFF: u32 = 391459;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterCastUpdate(sync_fields),
    SpellScriptHook::AfterForegroundSpecial(consume_scourge),
];

fn register(scripts: &mut AuraScripts) {
    let scourge = scripts.0.entry(SCOURGE).or_default();
    scourge.check_proc = Some(qualifies);
    scourge.on_proc = Some(proc_scourge);
}

// SimC76f8118 sc_death_knight.cpp7724–7736: landed auto-attacks roll
// the native proc only against owned Blood Plague, outside active DnD.
fn qualifies(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    world
        .get::<ActiveSpec>(aura.caster)
        .is_some_and(|spec| spec.0 == 250)
        && world.get::<HasBloodDecay>(aura.caster).is_none()
        && event
            .target
            .and_then(|target| world.get::<ActiveAuras>(target))
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|dot| dot.spell_id == PLAGUE && dot.caster == aura.caster)
            })
}

fn proc_scourge(world: &mut World, caster: Entity, _: &AuraInstance, _: Option<Entity>) {
    spell_cast::cast_triggered(world, caster, SCOURGE_BUFF, Some(caster), 1);
    let remaining = world
        .get::<spell_cast::SpellHistory>(caster)
        .map_or(0.0, |history| {
            let spell = world
                .resource::<SpellStore>()
                .get(DECAY)
                .expect("native Death and Decay");
            history.remaining_cooldown_ms(spell, spell_cast::history_now(world)) as f64 / 1000.0
        });
    spell_cast::scripts::reduce_cooldown(world, caster, DECAY, remaining);
}

// SimC76f8118 sc_death_knight.cpp9702–9717 consumes the free cast's proc.
fn consume_scourge(world: &mut World, caster: Entity, spell: &combat::spell_info::SpellInfo) {
    if spell.id == DECAY && world.get::<ActiveSpec>(caster).is_some_and(|spec| spec.0 == 250) {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == SCOURGE_BUFF && aura.caster == caster
        });
    }
}

#[derive(Component)]
struct InBloodDecay;
#[derive(Component)]
struct HasBloodDecay;

// SimC76f8118 sc_death_knight.cpp9680–9686: the selected ground buff is
// present only while the caster is in their actual active DnD field.
fn sync_fields(world: &mut World) {
    let now = spell_cast::now(world);
    let fields: Vec<_> = world
        .query::<(&SpellAreaTrigger, &Position)>()
        .iter(world)
        .filter(|(field, _)| {
            field.spell_id == DECAY && field.expires_at.is_none_or(|end| end > now)
        })
        .map(|(field, position)| (field.caster, *position))
        .collect();
    let actors: Vec<_> = world
        .query::<(Entity, &ActiveSpec, &Position)>()
        .iter(world)
        .filter(|(_, spec, _)| spec.0 == 250)
        .map(|(actor, _, position)| (actor, *position))
        .collect();
    if actors.is_empty() {
        return;
    }
    let radius = if fields.is_empty() {
        0.0
    } else {
        let spell = world
            .resource::<SpellStore>()
            .get(DECAY)
            .expect("active native Death and Decay");
        targets::effect_radius(
            spell
                .effects
                .iter()
                .find(|effect| effect.index == 1)
                .expect("native DnD radius"),
        )
    };
    for (actor, position) in actors {
        if fields.iter().any(|(caster, _)| *caster == actor) {
            world.entity_mut(actor).insert(HasBloodDecay);
        } else {
            world.entity_mut(actor).remove::<HasBloodDecay>();
        }
        let inside = fields.iter().any(|(caster, center)| {
            *caster == actor
                && Vec3::new(position.x, position.y, position.z)
                    .distance(Vec3::new(center.x, center.y, center.z))
                    <= radius
        });
        let was_inside = world.get::<InBloodDecay>(actor).is_some();
        if inside {
            if !was_inside {
                world.entity_mut(actor).insert(InBloodDecay);
            }
            if !auras::has_aura(world, actor, IN_DECAY) {
                apply(world, actor, IN_DECAY);
            }
            if auras::has_aura(world, actor, SANGUINE)
                && !auras::has_aura(world, actor, SANGUINE_BUFF)
            {
                apply(world, actor, SANGUINE_BUFF);
            }
        } else if was_inside {
            world.entity_mut(actor).remove::<InBloodDecay>();
            auras::remove_auras(world, actor, RemoveReason::Cancelled, |aura| {
                aura.caster == actor && matches!(aura.spell_id, IN_DECAY | SANGUINE_BUFF)
            });
        }
    }
}

fn apply(world: &mut World, actor: Entity, spell: u32) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

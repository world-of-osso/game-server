//! Native empower progress, separate from ordinary channel completion.
//! TC a352b1fa Spell.cpp4019–4038/4303–4344; Spell.h88–89.
use bevy::prelude::*;
use shared::casting::CastState;

use super::{CastFailure, PendingCastSpell, scripts};
use crate::spell_info::{SpellInfo, SpellStore};

const HOLD_AT_MAX_MS: u32 = 1_000;
const FINISH_GCD_SPELL: u32 = 359115;

#[derive(Component)]
pub(super) struct EmpowerProgress {
    stage_ends_ms: Vec<u32>,
    release_requested: bool,
    paid_costs: Vec<(super::CostPool, f32)>,
}

impl EmpowerProgress {
    fn completed_stages(&self, elapsed_ms: u32) -> u8 {
        self.stage_ends_ms
            .iter()
            .take_while(|end| **end <= elapsed_ms)
            .count() as u8
    }
}

fn compute_stage_ends(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<Vec<u32>, CastFailure> {
    let original = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .map_or(0, |duration| duration.duration_ms);
    if original <= 0 || spell.empower_stages.is_empty() {
        return Err(CastFailure::Unsupported(format!(
            "empower{} has no positive native stage duration",
            spell.id
        )));
    }
    let duration = super::duration_ms(world, caster, spell).max(0) as u32;
    let ratio = duration as f64 / f64::from(original);
    let mut elapsed = 0;
    let mut ends = Vec::with_capacity(spell.empower_stages.len());
    for (index, stage) in spell.empower_stages.iter().enumerate() {
        // TC assigns the remaining modified duration to the last stage.
        elapsed = if index + 1 == spell.empower_stages.len() {
            duration
        } else {
            elapsed + (f64::from(stage.duration_ms) * ratio) as u32
        };
        ends.push(elapsed);
    }
    Ok(ends)
}

pub(super) fn begin(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    if scripts::empower_completion(spell.id).is_none() {
        return Err(CastFailure::Unsupported(format!(
            "empower{} release script is not implemented",
            spell.id
        )));
    }
    let stage_ends_ms = compute_stage_ends(world, caster, spell)?;
    let total = stage_ends_ms
        .last()
        .copied()
        .expect("native stages checked")
        + HOLD_AT_MAX_MS;
    let paid_costs = super::spell_costs(world, caster, spell)?
        .into_iter()
        .map(|(pool, amount, _)| (pool, amount))
        .collect();
    super::execute_cast(world, caster, spell, target)?;
    let mut cast = CastState::channel(
        spell.id,
        target.map_or(0, Entity::to_bits),
        total as f32 / 1000.0,
        0.0,
        true,
    );
    super::insert_cast_state(world, caster, spell, &mut cast);
    world.entity_mut(caster).insert(EmpowerProgress {
        stage_ends_ms,
        release_requested: false,
        paid_costs,
    });
    Ok(())
}

fn should_release(
    progress: &EmpowerProgress,
    spell: &SpellInfo,
    elapsed_ms: u32,
    total_ms: u32,
) -> bool {
    let completed = progress.completed_stages(elapsed_ms);
    if completed == 0 {
        return false;
    }
    let selected = spell
        .cast_empower_to
        .is_some_and(|rank| completed >= rank.min(progress.stage_ends_ms.len() as u8).max(1));
    progress.release_requested || selected || elapsed_ms >= total_ms
}

/// Returns false only for an ordinary cast/channel.
pub(super) fn advance(world: &mut World, caster: Entity, dt: f32) -> bool {
    if world.get::<EmpowerProgress>(caster).is_none() {
        return false;
    }
    let (elapsed_ms, total_ms, target) = advance_channel_clock(world, caster, dt);
    let progress = world.get::<EmpowerProgress>(caster).unwrap();
    let pending = world
        .get::<PendingCastSpell>(caster)
        .expect("empower retains its admitted spell");
    if !should_release(progress, &pending.spell, elapsed_ms, total_ms) {
        return true;
    }
    let mut spell = pending.spell.clone();
    spell.cast_empower_level = Some(progress.completed_stages(elapsed_ms));
    finish(world, caster, &spell, target);
    true
}

fn advance_channel_clock(world: &mut World, caster: Entity, dt: f32) -> (u32, u32, Option<Entity>) {
    let mut cast = world
        .get_mut::<CastState>(caster)
        .expect("empower owns a channel");
    cast.tick(dt);
    let clock = (
        (cast.elapsed * 1000.0).round() as u32,
        (cast.duration * 1000.0).round() as u32,
        (cast.target != 0)
            .then_some(cast.target)
            .and_then(Entity::try_from_bits),
    );
    super::mark_dirty(world, caster);
    clock
}

fn finish(world: &mut World, caster: Entity, spell: &SpellInfo, target: Option<Entity>) {
    let handler =
        scripts::empower_completion(spell.id).expect("release handler validated at admission");
    let result = handler(world, caster, spell, target);
    world.entity_mut(caster).remove::<EmpowerProgress>();
    super::finish_channel(world, caster, spell.id);
    apply_finish_gcd(world, caster);
    if let Err(failure) = result {
        if let Ok((reason, _)) = failure.to_contract() {
            super::send_spell_failure(world, caster, spell.id, reason, None);
        }
        super::send_cast_rejection(world, caster, spell.id, &failure);
    }
}

fn apply_finish_gcd(world: &mut World, caster: Entity) {
    let gcd = world
        .resource::<SpellStore>()
        .get(FINISH_GCD_SPELL)
        .expect("TC empower finish GCD359115 requires native metadata")
        .clone();
    super::start_gcd(world, caster, &gcd);
}

/// Spell::finish failure: normal cooldown reset, actual paid power refund, finish GCD.
/// Charge recovery is separate in TC SpellHistory and is not restored here.
pub(super) fn cancel(world: &mut World, caster: Entity) {
    let Some(progress) = world.entity_mut(caster).take::<EmpowerProgress>() else {
        return;
    };
    let spell = world
        .get::<PendingCastSpell>(caster)
        .expect("empower retains its spell")
        .spell
        .clone();
    reset_spell_cooldown(world, caster, &spell);
    refund_costs(world, caster, progress.paid_costs);
    apply_finish_gcd(world, caster);
}

fn reset_spell_cooldown(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some(mut history) = world.get_mut::<super::SpellHistory>(caster) else {
        return;
    };
    if history.spell_ready_at.remove(&spell.id).is_none() {
        return;
    }
    let category = spell
        .categories
        .as_ref()
        .map_or(0, |category| category.category);
    if history.category_spell_ids.get(&category) == Some(&spell.id) {
        history.category_ready_at.remove(&category);
        history.category_spell_ids.remove(&category);
    }
    super::send_to_owner(
        world,
        caster,
        shared::protocol::SpellCooldownUpdate {
            spell_id: spell.id,
            category,
            duration_ms: 0,
            remaining_ms: 0,
            is_gcd: false,
        },
    );
}

fn refund_costs(world: &mut World, caster: Entity, costs: Vec<(super::CostPool, f32)>) {
    for (pool, amount) in costs {
        match pool {
            super::CostPool::Power(power) => {
                super::powers::change_power(world, caster, power, amount);
            }
            super::CostPool::Health => {
                let mut health = world
                    .get_mut::<shared::components::Health>(caster)
                    .expect("power caster health");
                health.current = (health.current + amount).clamp(0.0, health.max);
            }
        }
    }
}

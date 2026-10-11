//! TrinityCore a352b1fa SUMMON_CREATURE and TempSummon::Update.
use super::*;
use shared::components::{CombatStatus, Position, Rotation};

const SUMMON_PERSONAL: u32 = 1;
const SUMMON_PREFER_INVOKER: u32 = 2;
const SUMMON_ATTACK_TARGET: u32 = 4;

#[derive(Component)]
struct SummonLifetime {
    kind: u32,
    remaining_ms: u32,
    duration_ms: u32,
}

impl SmartAi {
    pub(super) fn summon_action(
        &mut self,
        ctx: &mut Ctx,
        event: &SmartEvent,
        targets: &[Entity],
        invoker: Option<Entity>,
    ) {
        let [entry, _, _, flags, _, stored_id, created_by_spell] = event.action_params;
        if flags & SUMMON_PERSONAL != 0 || created_by_spell != 0 {
            warn!(
                entry,
                flags,
                created_by_spell,
                "SmartAI personal/spell-property summons are not implemented"
            );
            return;
        }
        let summoner = if flags & SUMMON_PREFER_INVOKER != 0 {
            let Some(invoker) = invoker else { return };
            invoker
        } else {
            ctx.me
        };
        if stored_id != 0 {
            self.stored_targets.remove(&stored_id);
        }
        let positions = summon_positions(ctx, event, targets, invoker);
        let mut summoner_ctx = Ctx {
            world: ctx.world,
            me: summoner,
        };
        for (position, attack_target) in positions {
            for _ in 0..event.action_params[4].max(1) {
                if let Some(summon) = summoner_ctx.summon_creature(entry, position) {
                    self.record_summon(&mut summoner_ctx, event, summon, attack_target);
                }
            }
        }
    }

    fn record_summon(
        &mut self,
        ctx: &mut Ctx,
        event: &SmartEvent,
        summon: Entity,
        attack_target: Option<Entity>,
    ) {
        let [_, kind, duration_ms, flags, _, stored_id, _] = event.action_params;
        ctx.world.entity_mut(summon).insert(SummonLifetime {
            kind,
            remaining_ms: duration_ms,
            duration_ms,
        });
        if stored_id != 0 {
            self.stored_targets
                .entry(stored_id)
                .or_default()
                .push(summon);
        }
        if flags & SUMMON_ATTACK_TARGET != 0
            && let Some(target) = attack_target
        {
            crate::creature_aggro::add_threat(ctx.world, summon, target, 0.0);
        }
    }
}

fn summon_positions(
    ctx: &Ctx,
    event: &SmartEvent,
    targets: &[Entity],
    invoker: Option<Entity>,
) -> Vec<([f32; 4], Option<Entity>)> {
    if event.target_type == TARGET_POSITION {
        return vec![(event.target_position, invoker)];
    }
    targets
        .iter()
        .filter_map(|&target| {
            let position = ctx.world.get::<Position>(target)?;
            let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
            let orientation = ctx
                .world
                .get::<Rotation>(target)
                .map_or(0.0, |r| r.y - std::f32::consts::FRAC_PI_2);
            let offset = event.target_position;
            Some((
                [
                    x + offset[0],
                    y + offset[1],
                    z + offset[2],
                    orientation + offset[3],
                ],
                Some(target),
            ))
        })
        .collect()
}

fn expire_summon(
    timer: &mut SummonLifetime,
    diff_ms: u32,
    dead: bool,
    corpse: bool,
    combat: bool,
) -> bool {
    if dead {
        return true;
    }
    let tick = match timer.kind {
        1 => !combat && !corpse,
        2 => {
            if corpse {
                return true;
            }
            !combat
        }
        3 => true,
        4 => !combat,
        5 => return corpse,
        6 => corpse,
        7 | 8 => return false,
        _ => {
            warn!(kind = timer.kind, "SmartAI unknown temporary summon type");
            return true;
        }
    };
    if tick {
        if timer.remaining_ms <= diff_ms {
            return true;
        }
        timer.remaining_ms -= diff_ms;
    } else if matches!(timer.kind, 1 | 2 | 4) {
        timer.remaining_ms = timer.duration_ms;
    }
    false
}

pub(super) fn tick_summons(world: &mut World, diff_ms: u32) {
    let mut expired = Vec::new();
    for (entity, mut timer, dying, corpse, combat) in world
        .query::<(
            Entity,
            &mut SummonLifetime,
            Has<crate::combat::DeathState>,
            Has<crate::combat::Corpse>,
            Option<&CombatStatus>,
        )>()
        .iter_mut(world)
    {
        if expire_summon(
            &mut timer,
            diff_ms,
            dying && !corpse,
            corpse,
            combat.is_some_and(|status| status.0),
        ) {
            expired.push(entity);
        }
    }
    for entity in expired {
        super::lifetimes::despawn_creature(world, entity, 0);
    }
}

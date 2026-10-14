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

impl SummonLifetime {
    fn new(kind: u32, duration_ms: u32) -> Self {
        Self {
            kind,
            remaining_ms: duration_ms,
            duration_ms,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct GroupMember {
    entry: u32,
    position: [f32; 4],
    kind: u32,
    duration_ms: u32,
}

pub(super) fn load_summon_groups(
    conn: &Connection,
) -> rusqlite::Result<HashMap<(u32, u32), Vec<GroupMember>>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='tdb_creature_summon_groups')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        warn!(
            "SmartAI SUMMON_CREATURE_GROUP disabled: tdb_creature_summon_groups absent; apply overlay --tdb-world-sql"
        );
        return Ok(HashMap::new());
    }
    let mut query = conn.prepare("SELECT summonerId,groupId,entry,position_x,position_y,position_z,orientation,summonType,summonTime FROM tdb_creature_summon_groups WHERE summonerType=0")?;
    let rows = query.query_map([], |row| {
        Ok((
            (row.get(0)?, row.get(1)?),
            GroupMember {
                entry: row.get(2)?,
                position: [row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?],
                kind: row.get(7)?,
                duration_ms: row.get(8)?,
            },
        ))
    })?;
    let mut groups: HashMap<(u32, u32), Vec<GroupMember>> = HashMap::new();
    for row in rows {
        let (key, member) = row?;
        groups.entry(key).or_default().push(member);
    }
    Ok(groups)
}

/// WorldObject::SummonCreatureGroup, shared by native C++ ports and SmartAI.
pub(crate) fn summon_creature_group(ctx: &mut Ctx, group_id: u32) -> Option<Vec<Entity>> {
    let Some(entry) = ctx
        .world
        .get::<shared::components::Npc>(ctx.me)
        .map(|npc| npc.template_id)
    else {
        warn!(actor = ?ctx.me, group_id, "SummonCreatureGroup has no creature owner");
        return None;
    };
    let members = ctx
        .world
        .resource::<SmartScripts>()
        .3
        .get(&(entry, group_id))
        .cloned();
    let Some(members) = members else {
        warn!(
            entry,
            group_id, "SummonCreatureGroup: authored group is absent"
        );
        return None;
    };
    let mut summons = Vec::with_capacity(members.len());
    for member in members {
        if let Some(summon) = summon_group_member(ctx, member) {
            summons.push(summon);
        }
    }
    Some(summons)
}

fn summon_group_member(ctx: &mut Ctx, member: GroupMember) -> Option<Entity> {
    let summon = ctx.summon_creature(member.entry, member.position)?;
    ctx.world
        .entity_mut(summon)
        .insert(SummonLifetime::new(member.kind, member.duration_ms));
    Some(summon)
}

impl SmartAi {
    pub(super) fn summon_group(
        &mut self,
        ctx: &mut Ctx,
        event: &SmartEvent,
        invoker: Option<Entity>,
    ) {
        let [group_id, attack_invoker, stored_id, ..] = event.action_params;
        let Some(summons) = ctx.summon_creature_group(group_id) else {
            return;
        };
        if attack_invoker != 0
            && let Some(invoker) = invoker
        {
            for &summon in &summons {
                crate::creature_aggro::add_threat(ctx.world, summon, invoker, 0.0);
            }
        }
        if stored_id != 0 {
            self.stored_targets.insert(stored_id, summons);
        }
    }

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
        ctx.world
            .entity_mut(summon)
            .insert(SummonLifetime::new(kind, duration_ms));
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

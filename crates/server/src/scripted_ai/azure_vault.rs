//! TC a352b1fa Leymor intro/Sprouts/Infused Strike, Brand and Telash Frost Bomb hooks.
//! Telash power/first-loop Frost and Icy clocks are ported; full phases/jumps/runes are unported.
use bevy::prelude::*;
use shared::components::{PowerType, UnitFlags};

use crate::spell_cast::powers::{PowerPools, PowerTypeTable};

use crate::auras::{
    AuraInstance, RemoveReason, handlers,
    scripts::{AuraScript, AuraScripts},
};

pub(crate) const FROST_BOMB_CAST: u32 = 386781;
const FROST_BOMB_AURA: u32 = 386881;
const FROZEN_GROUND: u32 = 387149;
const FROST_BOMB_DAMAGE: u32 = 386910;
pub(crate) const EXPLOSIVE_BRAND: u32 = 374567;
const EXPLOSIVE_BRAND_CHILD: u32 = 374582;
const EXPLOSIVE_BRAND_DAMAGE: u32 = 374570;
const LEYMOR_SCRIPT: &str = "boss_leymor";
const TELASH_SCRIPT: &str = "boss_telash_greywing";
const SPROUT_SCRIPT: &str = "npc_ley_line_sprouts";
const LEY_LINE_SPROUT_AURA: u32 = 374161;
const TELASH_INITIAL_POWER: f32 = 60.0;
const ICE_POWER_PERIODIC: u32 = 389453;
const ICE_POWER_PER_TICK: f32 = 2.0;
const EVENT_FROST_BOMB: u32 = 1;
const ICY_DEVASTATOR: u32 = 387151;
const EVENT_ICY_DEVASTATOR: u32 = 2;
const AZURE_INSTANCE: &str = "instance_azure_vault";
const STASIS: u32 = 375729;
const LEYMOR: u32 = 186644;
const DATA_LEYMOR_INTRO_DONE: u32 = 4;
const ACTION_FINISH_LEYMOR_INTRO: i32 = 251501;
const EVENT_AWAKEN: u32 = 1;
const ARCANE_ERUPTION: u32 = 375749;
const INFUSED_STRIKE: u32 = 374789;
const EVENT_INFUSED_STRIKE: u32 = 5;
const ERUPTING_FISSURE: u32 = 386660;
const EVENT_ERUPTING_FISSURE: u32 = 4;
const EVENT_EXPLOSIVE_BRAND: u32 = 6;
const CONSUMING_STOMP: u32 = 374720;
const CONSUMING_STOMP_DAMAGE: u32 = 374731;
const EVENT_CONSUMING_STOMP: u32 = 3;
pub(crate) const LEY_LINE_SPROUTS: u32 = 374364;
const LEY_LINE_SPROUTS_MISSILE: u32 = 374362;
const SPROUT_GROUP_ORIGINS: [(f32, f32, f32); 7] = [
    (-5129.39, 1253.30, 555.58),
    (-5101.68, 1253.71, 555.90),
    (-5114.70, 1230.28, 555.89),
    (-5141.62, 1230.33, 555.83),
    (-5155.62, 1253.60, 555.87),
    (-5141.42, 1276.70, 555.89),
    (-5114.78, 1277.42, 555.87),
];
const EVENT_LEY_LINE_SPROUTS: u32 = 2;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(name, LEYMOR_SCRIPT | TELASH_SCRIPT | SPROUT_SCRIPT)
}

pub(super) fn creature_ai(
    world: &World,
    actor: Entity,
    script_name: &str,
) -> Option<Box<dyn super::CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let maps = world.get_resource::<crate::maps::MapStore>()?;
    if maps.instance_scripts.get(&map.map_id).map(String::as_str) != Some(AZURE_INSTANCE) {
        return None;
    }
    if script_name == SPROUT_SCRIPT {
        return Some(Box::new(LeyLineSprout));
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    match script_name {
        LEYMOR_SCRIPT => Some(Box::new(Leymor {
            boss,
            tasks: super::EventMap::default(),
        })),
        TELASH_SCRIPT => Some(Box::new(Telash { boss })),
        _ => None,
    }
}

/// TC instance_azure_vault::OnCreatureGroupDepleted: match this group's leader only.
pub(super) fn finish_leymor_intro(
    world: &mut World,
    map: crate::maps::InMap,
    leader_ids: &crate::zones::CreatureStringIds,
) {
    let is_azure = world
        .get_resource::<crate::maps::MapStore>()
        .and_then(|maps| maps.instance_scripts.get(&map.map_id))
        .is_some_and(|name| name == AZURE_INSTANCE);
    if !is_azure || !leader_ids.has("leymor_arcane_tender") {
        return;
    }
    if super::instance_script::get_data(world, map, DATA_LEYMOR_INTRO_DONE) != 0 {
        return;
    }
    super::instance_script::set_data(world, map, DATA_LEYMOR_INTRO_DONE, 1);
    let leymor = world
        .query::<(Entity, &shared::components::Npc, &crate::maps::InMap)>()
        .iter(world)
        .find_map(|(entity, npc, copy)| {
            (npc.template_id == LEYMOR && *copy == map).then_some(entity)
        });
    if let Some(leymor) = leymor {
        super::dispatch(
            world,
            leymor,
            super::Hook::DoAction(ACTION_FINISH_LEYMOR_INTRO),
        );
    }
}

struct LeyLineSprout;

impl super::CreatureAi for LeyLineSprout {
    fn just_appeared(&mut self, ctx: &mut super::Ctx) {
        ctx.do_cast_self(LEY_LINE_SPROUT_AURA);
    }
}

struct Leymor {
    boss: super::boss_ai::BossAi,
    tasks: super::EventMap,
}

impl super::boss_ai::BossScript for Leymor {
    fn boss_ai(&mut self) -> &mut super::boss_ai::BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut super::Ctx, event_id: u32) {
        match event_id {
            EVENT_INFUSED_STRIKE => ctx.do_cast_victim(INFUSED_STRIKE),
            EVENT_ERUPTING_FISSURE => ctx.do_cast_victim(ERUPTING_FISSURE),
            EVENT_EXPLOSIVE_BRAND => ctx.do_cast_self(EXPLOSIVE_BRAND),
            EVENT_CONSUMING_STOMP => ctx.do_cast_self(CONSUMING_STOMP),
            EVENT_LEY_LINE_SPROUTS => ctx.do_cast_self(LEY_LINE_SPROUTS),
            _ => return,
        }
        self.boss.events.repeat(48_000);
    }
}

impl super::CreatureAi for Leymor {
    fn reset(&mut self, ctx: &mut super::Ctx) {
        self.tasks.reset();
        self.boss.reset(ctx);
    }

    fn just_appeared(&mut self, ctx: &mut super::Ctx) {
        let Some(map) = ctx.map() else {
            return;
        };
        if super::instance_script::get_data(ctx.world, map, DATA_LEYMOR_INTRO_DONE) != 0 {
            return;
        }
        if let Some(mut flags) = ctx.world.get_mut::<UnitFlags>(ctx.me) {
            flags.0 |= UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC;
        }
        ctx.do_cast_self(STASIS);
    }

    fn channel_finished(&mut self, ctx: &mut super::Ctx, spell_id: u32) {
        if spell_id == CONSUMING_STOMP {
            ctx.do_cast_at(None, CONSUMING_STOMP_DAMAGE, true);
        }
    }

    fn just_engaged_with(&mut self, ctx: &mut super::Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(EVENT_LEY_LINE_SPROUTS, 3_000);
        self.boss.events.schedule(EVENT_INFUSED_STRIKE, 10_000);
        self.boss.events.schedule(EVENT_ERUPTING_FISSURE, 20_000);
        self.boss.events.schedule(EVENT_EXPLOSIVE_BRAND, 31_000);
        self.boss.events.schedule(EVENT_CONSUMING_STOMP, 45_000);
    }

    fn just_died(&mut self, ctx: &mut super::Ctx, _killer: Option<Entity>) {
        self.tasks.reset();
        self.boss.just_died(ctx);
    }

    fn do_action(&mut self, _ctx: &mut super::Ctx, action: i32) {
        if action == ACTION_FINISH_LEYMOR_INTRO {
            self.tasks.schedule(EVENT_AWAKEN, 1_000);
        }
    }

    fn update_ai(&mut self, ctx: &mut super::Ctx, diff_ms: u32) {
        self.tasks.update(u64::from(diff_ms));
        if self.tasks.execute_event() == Some(EVENT_AWAKEN) {
            crate::auras::remove_auras(ctx.world, ctx.me, RemoveReason::Cancelled, |aura| {
                aura.spell_id == STASIS
            });
            if let Some(mut flags) = ctx.world.get_mut::<UnitFlags>(ctx.me) {
                flags.0 &= !(UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC);
            }
            ctx.do_cast_self(ARCANE_ERUPTION);
            ctx.talk(0);
        }
        super::boss_ai::boss_update_ai(self, ctx, diff_ms);
    }
}

struct Telash {
    boss: super::boss_ai::BossAi,
}

impl super::boss_ai::BossScript for Telash {
    fn boss_ai(&mut self) -> &mut super::boss_ai::BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut super::Ctx, event_id: u32) {
        match event_id {
            EVENT_FROST_BOMB => {
                ctx.do_cast_self(FROST_BOMB_CAST);
                self.boss.events.reschedule(EVENT_ICY_DEVASTATOR, 10_800);
            }
            EVENT_ICY_DEVASTATOR => {
                if let Some(target) = ctx.select_target_random(75.0, true) {
                    ctx.do_cast_at(Some(target), ICY_DEVASTATOR, false);
                }
            }
            _ => {}
        }
        // TC starts with one Frost/Icy each; post-phase action/counters remain unported.
    }
}

impl super::CreatureAi for Telash {
    fn reset(&mut self, ctx: &mut super::Ctx) {
        self.boss.reset(ctx);
    }

    fn just_appeared(&mut self, ctx: &mut super::Ctx) {
        let Some((primary, mut powers)) = read_telash_primary_powers(ctx.world, ctx.me) else {
            warn!(actor = ?ctx.me, "Telash initial power requires native template/class/power data");
            return;
        };
        let Some(pool) = powers.get_mut(primary) else {
            warn!(actor = ?ctx.me, ?primary, "Telash native primary power has no non-mana pool");
            return;
        };
        pool.current = TELASH_INITIAL_POWER.min(pool.max);
        ctx.world.entity_mut(ctx.me).insert(powers);
    }

    fn just_engaged_with(&mut self, ctx: &mut super::Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.do_cast(ICE_POWER_PERIODIC);
        self.boss.events.schedule(EVENT_FROST_BOMB, 3_500);
        self.boss.events.schedule(EVENT_ICY_DEVASTATOR, 14_500);
    }

    fn just_died(&mut self, ctx: &mut super::Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut super::Ctx, diff_ms: u32) {
        super::boss_ai::boss_update_ai(self, ctx, diff_ms);
    }
}

fn read_telash_primary_powers(world: &World, actor: Entity) -> Option<(PowerType, PowerPools)> {
    let npc = world.get::<shared::components::Npc>(actor)?;
    let data = world.get_resource::<crate::world_data::WorldData>()?;
    let template = data.templates.get(&npc.template_id)?;
    let table = world.get_resource::<PowerTypeTable>()?;
    let primary = table.display_power(template.unit_class)?;
    table.get(primary)?;
    let powers = PowerPools::from_table(table, vec![primary]);
    Some((primary, powers))
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        ICE_POWER_PERIODIC,
        AuraScript {
            on_periodic: Some(tick_ice_power),
            ..Default::default()
        },
    );
    scripts.0.insert(
        EXPLOSIVE_BRAND,
        AuraScript {
            on_remove: Some(explode_expired_brand),
            ..Default::default()
        },
    );
    scripts.0.insert(
        FROST_BOMB_AURA,
        AuraScript {
            on_remove: Some(explode_expired_frost_bomb),
            ..Default::default()
        },
    );
}

/// TC389453 effect0/PERIODIC_DUMMY: SetPower(primary, GetPower(ENERGY) +2).
fn tick_ice_power(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    let periodic_dummy = index == 0
        && aura
            .effects
            .iter()
            .any(|effect| effect.index == 0 && effect.aura_type == handlers::PERIODIC_DUMMY);
    if !periodic_dummy {
        return;
    }
    let Some((primary, delta)) = read_ice_power_delta(world, target) else {
        warn!(
            ?target,
            "Ice Power389453 requires native Energy and primary power pools"
        );
        return;
    };
    crate::spell_cast::powers::change_power(world, target, primary, delta);
}

fn read_ice_power_delta(world: &World, target: Entity) -> Option<(PowerType, f32)> {
    let energy = crate::spell_cast::powers::power_value(world, target, PowerType::Energy)?.0;
    let primary = world.get::<PowerPools>(target)?.order.first().copied()?;
    let current = crate::spell_cast::powers::power_value(world, target, primary)?.0;
    Some((primary, energy + ICE_POWER_PER_TICK - current))
}

/// TC374567 effect0/APPLY_AURA OnEffectHit: one child, retaining native parent auras.
pub(crate) fn cast_brand_child(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
) {
    crate::spell_cast::cast_triggered(
        world,
        ctx.caster,
        EXPLOSIVE_BRAND_CHILD,
        None,
        ctx.depth + 1,
    );
}

/// TC a352b1fa374364 effect0 OnEffectHit: two native missiles per authored origin.
pub(crate) fn cast_sprout_missiles(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
) {
    for (x, y, z) in SPROUT_GROUP_ORIGINS {
        let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
        let position = Vec3::new(x, y, z);
        for _ in 0..2 {
            crate::spell_cast::cast_triggered_at_destination(
                world,
                ctx.caster,
                LEY_LINE_SPROUTS_MISSILE,
                position,
                ctx.depth + 1,
            );
        }
    }
}

/// OnEffectHitTarget, effect0/SPELL_EFFECT_DUMMY.
pub(crate) fn apply_frost_bomb(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    cast_azure_child(world, caster, target, FROST_BOMB_AURA);
}

/// OnEffectRemove, effect0/SPELL_AURA_DUMMY, AURA_REMOVE_BY_EXPIRE only.
fn explode_expired_frost_bomb(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    let removed_dummy = aura
        .effects
        .iter()
        .any(|effect| effect.index == 0 && effect.aura_type == handlers::DUMMY);
    if reason != RemoveReason::Expired || !removed_dummy || world.get_entity(aura.caster).is_err() {
        return;
    }
    cast_azure_child(world, aura.caster, target, FROZEN_GROUND);
    cast_azure_child(world, aura.caster, target, FROST_BOMB_DAMAGE);
}

/// AfterEffectRemove, effect0/SPELL_AURA_DUMMY, AURA_REMOVE_BY_EXPIRE only.
fn explode_expired_brand(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    let removed_dummy = aura
        .effects
        .iter()
        .any(|effect| effect.index == 0 && effect.aura_type == handlers::DUMMY);
    if reason != RemoveReason::Expired || !removed_dummy || world.get_entity(aura.caster).is_err() {
        return;
    }
    cast_azure_child(world, aura.caster, target, EXPLOSIVE_BRAND_DAMAGE);
}

fn cast_azure_child(world: &mut World, caster: Entity, target: Entity, spell: u32) {
    if let Err(error) = crate::spell_cast::creature_cast(world, caster, spell, Some(target), true) {
        warn!("Azure child {spell} from {caster} to {target} failed: {error:?}");
    }
}

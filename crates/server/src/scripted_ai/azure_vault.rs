//! TC a352b1fa Leymor intro/Sprouts/Infused Strike, Brand and Telash Frost Bomb hooks.
//! Telash power/rune startup/first-loop clocks, first100Energy poll and outbound
//! jump hit/first center landing, Rune activation and effect1 strip are ported;
//! native AfterCast return/landing and source post-phase counters are ported.
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
const ARCANE_POWER: u32 = 374736;
const TELASH_INITIAL_POWER: f32 = 60.0;
const TELASH_VAULT_RUNES_GROUP: u32 = 0;
const ICE_POWER_PERIODIC: u32 = 389453;
const ICE_POWER_PER_TICK: f32 = 2.0;
const EVENT_FROST_BOMB: u32 = 1;
const ICY_DEVASTATOR: u32 = 387151;
const EVENT_ICY_DEVASTATOR: u32 = 2;
const EVENT_ABSOLUTE_ZERO: u32 = 3;
const ABSOLUTE_ZERO_POLL_MS: u64 = 500;
const ABSOLUTE_ZERO_POWER: f32 = 100.0;
pub(crate) const ABSOLUTE_ZERO_CAST: u32 = 387928;
const ABSOLUTE_ZERO_JUMP: u32 = 387981;
pub(crate) const ABSOLUTE_ZERO_DAMAGE: u32 = 388008;
const TELASH_JUMP_POSITION_WOW: Vec3 = Vec3::new(-5336.8003, 1066.6493, 344.32678);
const ABSOLUTE_ZERO_JUMP_BACK: u32 = 388082;
const ACTION_RESCHEDULE_SPELLS: i32 = 1;
const TELASH_JUMP_BACK_POSITIONS_WOW: [Vec3; 3] = [
    Vec3::new(-5350.0835, 1089.8976, 340.5709),
    Vec3::new(-5310.4464, 1066.8134, 340.5967),
    Vec3::new(-5350.3934, 1043.9323, 340.5894),
];
const ABSOLUTE_ZERO_SHIELD: u32 = 388084;
pub(crate) const ACTIVATE_VAULT_RUNE: u32 = 390209;
const VAULT_RUNE_AT_AURA: u32 = 388065;
const VAULT_RUNE_SHIELD: u32 = 390201;
const INACTIVE_VAULT_RUNE: u32 = 390238;
const AZURE_INSTANCE: &str = "instance_azure_vault";
const STASIS: u32 = 375729;
const LEYMOR: u32 = 186644;
const DATA_LEYMOR_INTRO_DONE: u32 = 4;
const ACTION_FINISH_LEYMOR_INTRO: i32 = 251501;
const EVENT_AWAKEN: u32 = 1;
const ARCANE_ERUPTION: u32 = 375749;
const INFUSED_STRIKE: u32 = 374789;
const EVENT_INFUSED_STRIKE: u32 = 5;
pub(crate) const ERUPTING_FISSURE: u32 = 386660;
const FISSURE_SPROUT_SELECTOR: u32 = 394154;
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
    matches!(
        name,
        LEYMOR_SCRIPT
            | TELASH_SCRIPT
            | SPROUT_SCRIPT
            | super::azureblade::SCRIPT
            | super::umbrelskul::SCRIPT
    )
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
        super::azureblade::SCRIPT => Some(super::azureblade::creature_ai(boss)),
        super::umbrelskul::SCRIPT => Some(super::umbrelskul::creature_ai(boss)),
        LEYMOR_SCRIPT => Some(Box::new(Leymor {
            boss,
            tasks: super::EventMap::default(),
        })),
        TELASH_SCRIPT => Some(Box::new(Telash {
            boss,
            is_in_center: false,
            frost_bombs_remaining: 0,
            icy_devastators_remaining: 0,
        })),
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
        ctx.do_cast_at(None, ARCANE_POWER, true);
    }

    fn spell_hit(&mut self, ctx: &mut super::Ctx, _caster: Entity, spell_id: u32) {
        // TC boss_leymor.cpp:303–310: death owns stack consumption, not this hit.
        if matches!(spell_id, FISSURE_SPROUT_SELECTOR | EXPLOSIVE_BRAND_DAMAGE) {
            crate::combat::kill_unit(ctx.world, ctx.me, ctx.me);
        }
    }

    fn just_died(&mut self, ctx: &mut super::Ctx, _killer: Option<Entity>) {
        // TC boss_leymor.cpp:286–300: the actual TempSummon owner loses one stack.
        let Some(owner) = ctx.world.get::<crate::zones::TempSummon>(ctx.me) else {
            return;
        };
        let owner = owner.summoner;
        let instance = ctx
            .world
            .get::<crate::auras::ActiveAuras>(owner)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == ARCANE_POWER)
                    .map(|aura| aura.instance_id)
            });
        if let Some(instance) = instance {
            crate::auras::spend_stack(ctx.world, owner, instance);
        }
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
    is_in_center: bool,
    frost_bombs_remaining: u8,
    icy_devastators_remaining: u8,
}

impl Telash {
    fn cast_frost_bomb(&mut self, ctx: &mut super::Ctx) {
        ctx.do_cast_self(FROST_BOMB_CAST);
        self.frost_bombs_remaining -= 1;
        if self.frost_bombs_remaining == 0 {
            if self.icy_devastators_remaining == 1 {
                self.boss.events.reschedule(EVENT_ICY_DEVASTATOR, 10_800);
            }
            return;
        }
        let delay = if self.frost_bombs_remaining == 2 {
            18_200
        } else {
            15_800
        };
        self.boss.events.schedule(EVENT_FROST_BOMB, delay);
    }

    fn cast_icy_devastator(&mut self, ctx: &mut super::Ctx) {
        let Some(target) = ctx.select_target_random(75.0, true) else {
            return;
        };
        ctx.do_cast_at(Some(target), ICY_DEVASTATOR, false);
        self.icy_devastators_remaining -= 1;
        if self.icy_devastators_remaining == 0 {
            if self.frost_bombs_remaining == 1 {
                self.boss.events.reschedule(EVENT_FROST_BOMB, 7_300);
            }
            return;
        }
        self.boss.events.schedule(EVENT_ICY_DEVASTATOR, 23_100);
    }

    fn start_absolute_zero_when_ready(&mut self, ctx: &mut super::Ctx) {
        let Some(power) = read_current_primary_power(ctx.world, ctx.me) else {
            warn!(actor = ?ctx.me, "Telash phase polling requires its actual primary power");
            return;
        };
        if power < ABSOLUTE_ZERO_POWER {
            return;
        }
        ctx.talk(3);
        ctx.talk(4);
        ctx.set_react_state(super::ReactState::Passive);
        ctx.world
            .entity_mut(ctx.me)
            .insert(super::waypoint_path::GravityDisabled);
        crate::auras::remove_auras(ctx.world, ctx.me, RemoveReason::Cancelled, |aura| {
            aura.spell_id == ICE_POWER_PERIODIC
        });
        ctx.do_cast_self(ABSOLUTE_ZERO_CAST);
        ctx.do_cast_self(ABSOLUTE_ZERO_SHIELD);
        ctx.do_cast_self(ACTIVATE_VAULT_RUNE);
        self.boss.events.cancel(EVENT_FROST_BOMB);
        self.boss.events.cancel(EVENT_ICY_DEVASTATOR);
    }
}

impl super::boss_ai::BossScript for Telash {
    fn boss_ai(&mut self) -> &mut super::boss_ai::BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut super::Ctx, event_id: u32) {
        match event_id {
            EVENT_FROST_BOMB => self.cast_frost_bomb(ctx),
            EVENT_ICY_DEVASTATOR => self.cast_icy_devastator(ctx),
            EVENT_ABSOLUTE_ZERO => {
                self.start_absolute_zero_when_ready(ctx);
                self.boss.events.repeat(ABSOLUTE_ZERO_POLL_MS);
            }
            _ => {}
        }
    }
}

impl super::CreatureAi for Telash {
    fn reset(&mut self, ctx: &mut super::Ctx) {
        self.boss.reset(ctx);
    }

    fn just_appeared(&mut self, ctx: &mut super::Ctx) {
        ctx.summon_creature_group(TELASH_VAULT_RUNES_GROUP);
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
        self.frost_bombs_remaining = 1;
        self.icy_devastators_remaining = 1;
        ctx.do_cast(ICE_POWER_PERIODIC);
        self.boss.events.schedule(EVENT_FROST_BOMB, 3_500);
        self.boss.events.schedule(EVENT_ICY_DEVASTATOR, 14_500);
        self.boss
            .events
            .schedule(EVENT_ABSOLUTE_ZERO, ABSOLUTE_ZERO_POLL_MS);
    }

    fn movement_inform(&mut self, ctx: &mut super::Ctx, _kind: super::MotionType, id: u32) {
        // TC boss_telash_greywing.cpp:133–149; actual jump arrival, type ignored.
        if id != super::jump_movement::EVENT_JUMP {
            return;
        }
        if self.is_in_center {
            ctx.do_cast_self(ICE_POWER_PERIODIC);
            crate::auras::remove_auras(ctx.world, ctx.me, RemoveReason::Cancelled, |aura| {
                aura.spell_id == ABSOLUTE_ZERO_SHIELD
            });
            ctx.set_react_state(super::ReactState::Aggressive);
            self.is_in_center = false;
        } else {
            ctx.do_cast_self(ABSOLUTE_ZERO_DAMAGE);
            self.is_in_center = true;
        }
    }

    fn do_action(&mut self, _ctx: &mut super::Ctx, action: i32) {
        if action == ACTION_RESCHEDULE_SPELLS {
            self.frost_bombs_remaining = 3;
            self.icy_devastators_remaining = 2;
            self.boss.events.schedule(EVENT_FROST_BOMB, 4_300);
            self.boss.events.schedule(EVENT_ICY_DEVASTATOR, 15_200);
        }
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

fn read_current_primary_power(world: &World, actor: Entity) -> Option<f32> {
    let primary = *world.get::<PowerPools>(actor)?.order.first()?;
    crate::spell_cast::powers::power_value(world, actor, primary).map(|(current, _)| current)
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

/// TC boss_leymor.cpp:495–513: once-effect HIT preserves the actual destination.
pub(crate) fn cast_fissure_selector(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
) {
    let Some(destination) = ctx.dest else {
        warn!("Erupting Fissure386660 cannot cast394154 without its hit destination");
        return;
    };
    crate::spell_cast::cast_triggered_at_destination(
        world,
        ctx.caster,
        FISSURE_SPROUT_SELECTOR,
        destination.position,
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

/// TC a352b1fa boss_telash_greywing.cpp:85,390–397: effect0 HIT casts the
/// full-mask jump from the original caster at the authored elevated center.
pub(crate) fn cast_absolute_zero_outbound_jump(
    world: &mut World,
    caster: Entity,
    _target: Entity,
    depth: u8,
) {
    let source = TELASH_JUMP_POSITION_WOW;
    let (x, y, z) = crate::terrain_height::wow_to_bevy(source.x, source.y, source.z);
    crate::spell_cast::cast_triggered_at_destination(
        world,
        caster,
        ABSOLUTE_ZERO_JUMP,
        Vec3::new(x, y, z),
        depth + 1,
    );
}

/// TC a352b1fa388008 AfterCast: AI/current MaxThreat guards precede both actions.
pub(crate) fn return_after_absolute_zero(
    world: &mut World,
    caster: Entity,
    _spell: &crate::spell_info::SpellInfo,
) {
    if world.get::<super::CreatureAiState>(caster).is_none() {
        return;
    }
    let Some(victim) = (super::Ctx { world, me: caster }).select_target_max_threat() else {
        return;
    };
    super::dispatch(
        world,
        caster,
        super::Hook::DoAction(ACTION_RESCHEDULE_SPELLS),
    );
    let Some(destination) = find_nearest_telash_return_position(world, victim) else {
        warn!(
            ?caster,
            ?victim,
            "Absolute Zero return requires source victim position and combat reach"
        );
        return;
    };
    crate::spell_cast::cast_triggered_at_destination(
        world,
        caster,
        ABSOLUTE_ZERO_JUMP_BACK,
        destination,
        1,
    );
}

fn find_nearest_telash_return_position(world: &World, victim: Entity) -> Option<Vec3> {
    let victim_position = crate::spell_cast::targets::position(world, victim)?;
    let reach = super::model_reach::combat_reach(world, victim)?;
    let points = TELASH_JUMP_BACK_POSITIONS_WOW.map(|point| {
        let (x, y, z) = crate::terrain_height::wow_to_bevy(point.x, point.y, point.z);
        Vec3::new(x, y, z)
    });
    let mut nearest = points[0];
    let mut nearest_distance = f32::MAX;
    for point in points {
        // Object.cpp436–439 GetDistance(Position): reach subtraction and zero floor.
        let distance = (victim_position.distance(point) - reach).max(0.0);
        if distance < nearest_distance {
            nearest_distance = distance;
            nearest = point;
        }
    }
    Some(nearest)
}

/// TC a352b1fa390209 effect0/DUMMY: source entry conditions select the Rune;
/// inactive Runes are skipped, otherwise the hit Rune casts with implicit self.
pub(crate) fn activate_vault_rune(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    if crate::auras::has_aura(world, target, INACTIVE_VAULT_RUNE) {
        return;
    }
    if let Err(error) =
        crate::spell_cast::creature_cast(world, target, VAULT_RUNE_AT_AURA, None, true)
    {
        warn!(
            ?target,
            ?error,
            "Vault Rune activation could not cast native388065"
        );
    }
}

/// TC a352b1fa388008 effect1/DUMMY: source entry conditions select197204.
/// Remove the field aura before the shield so native OnRemove sees live membership.
pub(crate) fn strip_vault_rune(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    crate::auras::remove_auras(world, target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == VAULT_RUNE_AT_AURA
    });
    crate::auras::remove_auras(world, target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == VAULT_RUNE_SHIELD
    });
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

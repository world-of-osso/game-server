//! TC a352b1fa Leymor intro/Sprouts/Infused Strike, Brand and Telash Frost Bomb hooks.
//! Other boss schedules, energy/jump phases and rune/ground actions remain unported.
use bevy::prelude::*;
use shared::components::UnitFlags;

use crate::auras::{
    AuraInstance, RemoveReason, handlers,
    scripts::{AuraScript, AuraScripts},
};

pub(crate) const FROST_BOMB_CAST: u32 = 386781;
const FROST_BOMB_AURA: u32 = 386881;
const FROZEN_GROUND: u32 = 387149;
const FROST_BOMB_DAMAGE: u32 = 386910;
const EXPLOSIVE_BRAND: u32 = 374567;
const EXPLOSIVE_BRAND_DAMAGE: u32 = 374570;
const LEYMOR_SCRIPT: &str = "boss_leymor";
const AZURE_INSTANCE: &str = "instance_azure_vault";
const STASIS: u32 = 375729;
const LEYMOR: u32 = 186644;
const DATA_LEYMOR_INTRO_DONE: u32 = 4;
const ACTION_FINISH_LEYMOR_INTRO: i32 = 251501;
const EVENT_AWAKEN: u32 = 1;
const ARCANE_ERUPTION: u32 = 375749;
const INFUSED_STRIKE: u32 = 374789;
const EVENT_INFUSED_STRIKE: u32 = 5;
const LEY_LINE_SPROUTS: u32 = 374364;
const EVENT_LEY_LINE_SPROUTS: u32 = 2;

pub(super) fn is_creature_script(name: &str) -> bool {
    name == LEYMOR_SCRIPT
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn super::CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let maps = world.get_resource::<crate::maps::MapStore>()?;
    if maps.instance_scripts.get(&map.map_id).map(String::as_str) != Some(AZURE_INSTANCE) {
        return None;
    }
    Some(Box::new(Leymor {
        boss: super::instance_script::bound_boss(world, actor)?,
        tasks: super::EventMap::default(),
    }))
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

    fn just_engaged_with(&mut self, ctx: &mut super::Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(EVENT_LEY_LINE_SPROUTS, 3_000);
        self.boss.events.schedule(EVENT_INFUSED_STRIKE, 10_000);
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

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
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

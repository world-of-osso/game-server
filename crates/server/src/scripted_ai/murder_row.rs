//! Murder Row Kystia combat events from TrinityCore a352b1fa
//! src/server/scripts/QuelThalas/MurderRow/boss_kystia_manaheart.cpp.
//! Partial encounter port: primary/companion events, shields and mirror callbacks.
//! Infusion completion, Fel Spray damage and door/boundary behavior remain gaps.

use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, EventMap, instance_script};
use crate::maps::{InMap, InstanceManager, MapStore};
use shared::components::{Health, ModelDisplay, Npc, UnitFactionTemplate};

const SCRIPT: &str = "boss_kystia_manaheart";
const NIBBLES_SCRIPT: &str = "boss_kystia_manaheart_nibbles";
const INSTANCE: &str = "instance_murder_row";
const EVENT_CHAOS_BARRAGE: u32 = 1;
const EVENT_MIRROR_IMAGES: u32 = 2;
const EVENT_FEL_NOVA: u32 = 3;
const SPELL_CHAOS_BARRAGE: u32 = 1230298;
const SPELL_MIRROR_IMAGES: u32 = 1264095;
pub(crate) const SPELL_MIRROR_IMAGE: u32 = 1264098;
const SPELL_FELSTORM: u32 = 1264106;
pub(crate) const SPELL_FEL_NOVA_SELECTOR: u32 = 474240;
pub(crate) const SPELL_BLINK: u32 = 474183;
const SPELL_FEL_NOVA: u32 = 1223906;
pub(crate) const SPELL_FEL_CRAZED: u32 = 474365;
const SPELL_FELSHIELD: u32 = 1217989;
const SPELL_ILLICIT_INFUSION_VISUAL: u32 = 1217464;
const NPC_KYSTIA: u32 = 234648;
const NPC_NIBBLES: u32 = 234660;
const EVENT_FEL_SPRAY: u32 = 4;
const EVENT_CORRODING_SPITTLE: u32 = 5;
const EVENT_CHECK_HEALTH: u32 = 6;
const SPELL_FEL_SPRAY: u32 = 1253811;
const SPELL_CORRODING_SPITTLE: u32 = 1228198;
const SPELL_ILLICIT_INFUSION: u32 = 1230289;
const SPELL_LIGHT_INFUSION: u32 = 1230304;
const FACTION_FRIENDLY: u32 = 35;
const DISPLAY_FRIENDLY: u32 = 128332;
const DISPLAY_UNFRIENDLY: u32 = 126199;
const SPELL_ILLICIT_INFUSION_CAST: u32 = 474238;
const SPELL_DESTABILIZED: u32 = 1265412;

pub fn is_creature_script(script_name: &str) -> bool {
    matches!(script_name, SCRIPT | NIBBLES_SCRIPT)
}

pub fn creature_ai(
    world: &World,
    creature: Entity,
    script_name: &str,
) -> Option<Box<dyn CreatureAi>> {
    if !is_creature_script(script_name) {
        return None;
    }
    let map = world.get::<InMap>(creature)?;
    let store = world.get_resource::<MapStore>()?;
    if store.instance_scripts.get(&map.map_id).map(String::as_str) != Some(INSTANCE) {
        return None;
    }
    if script_name == NIBBLES_SCRIPT {
        return Some(Box::new(Nibbles::default()));
    }
    // Only an authenticated primary-actor binding grants encounter completion.
    let boss = instance_script::bound_boss(world, creature)?;
    let heroic = world
        .get_resource::<InstanceManager>()
        .and_then(|manager| manager.copy_difficulty(*map))
        .is_some_and(|difficulty| matches!(difficulty.difficulty_id, 2 | 8 | 23));
    Some(Box::new(KystiaManaheart {
        boss,
        heroic,
        felshield_count: 0,
    }))
}

/// TC spell_kystia_manaheart_fel_nova_selector::HandleHitTarget.
pub(crate) fn cast_blink_at_selected_target(
    world: &mut World,
    caster: Entity,
    target: Entity,
    _depth: u8,
) {
    Ctx { world, me: caster }.do_cast_at(Some(target), SPELL_BLINK, false);
}

/// TC spell_kystia_manaheart_blink::HandleCast (AfterCast).
pub(crate) fn cast_fel_nova_after_blink(
    world: &mut World,
    caster: Entity,
    _spell: &crate::spell_info::SpellInfo,
) {
    Ctx { world, me: caster }.do_cast_self(SPELL_FEL_NOVA);
}

/// TC FelCrazed effect1 prevents the default trigger and sets aura stacks to4.
pub(crate) fn cast_initial_felshield(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
) {
    cast_felshield_stacks(world, ctx.caster, 4);
}

fn cast_felshield_stacks(world: &mut World, caster: Entity, stacks: u8) {
    // The source explicitly requests TRIGGERED_FULL_MASK for this shield.
    let mut ctx = Ctx { world, me: caster };
    if !ctx.do_cast_at(Some(caster), SPELL_FELSHIELD, true) {
        warn!("Kystia Felshield {SPELL_FELSHIELD} cast failed for {caster:?}");
        return;
    }
    if !crate::auras::set_self_cast_stacks(ctx.world, caster, SPELL_FELSHIELD, stacks) {
        warn!("Kystia Felshield {SPELL_FELSHIELD} aura missing after cast for {caster:?}");
    }
}

struct KystiaManaheart {
    boss: BossAi,
    heroic: bool,
    felshield_count: u8,
}

pub(crate) fn register_aura_scripts(mut scripts: ResMut<crate::auras::scripts::AuraScripts>) {
    use crate::auras::scripts::AuraScript;
    scripts.0.insert(
        SPELL_MIRROR_IMAGES,
        AuraScript {
            on_periodic: Some(cast_mirror_image),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SPELL_FELSTORM,
        AuraScript {
            on_remove: Some(retire_felstorm_actor),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SPELL_LIGHT_INFUSION,
        AuraScript {
            on_apply: Some(apply_destabilized),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SPELL_ILLICIT_INFUSION,
        AuraScript {
            on_apply: Some(apply_corrupted_faction),
            ..Default::default()
        },
    );
}

/// TC spell_kystia_manaheart_mirror_images::HandlePeriodicEffect, effect0/aura23.
fn cast_mirror_image(
    world: &mut World,
    target: Entity,
    aura: &crate::auras::AuraInstance,
    index: u8,
) {
    if index == 0
        && aura.effects.iter().any(|effect| {
            effect.index == index
                && effect.aura_type == crate::auras::handlers::PERIODIC_TRIGGER_SPELL
        })
    {
        cast_ignoring_in_progress(world, target, target, SPELL_MIRROR_IMAGE);
    }
}

/// TC Felstorm effect0/aura23 AfterEffectRemove calls DespawnOrUnsummon(1ms).
fn retire_felstorm_actor(
    world: &mut World,
    target: Entity,
    aura: &crate::auras::AuraInstance,
    _reason: crate::auras::RemoveReason,
) {
    let has_periodic_effect = aura.effects.iter().any(|effect| {
        effect.index == 0 && effect.aura_type == crate::auras::handlers::PERIODIC_TRIGGER_SPELL
    });
    if has_periodic_effect && world.get::<Npc>(target).is_some() {
        super::smart_ai::lifetimes::schedule_despawn(world, target, 1, 0, false);
    }
}

/// TC spell_kystia_manaheart_mirror_image::SetDest. WoW y is negative engine z.
pub(crate) fn relocate_mirror_destination(dest: &mut crate::spell_cast::targets::Dest) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    dest.position.x += rng.gen_range(-30.0..=25.0);
    dest.position.z -= rng.gen_range(-15.0..=26.0);
}

fn cast_ignoring_in_progress(world: &mut World, caster: Entity, target: Entity, spell: u32) {
    if let Err(error) =
        crate::spell_cast::creature_cast_ignoring_in_progress(world, caster, spell, Some(target))
    {
        warn!("Murder Row scripted cast{spell} from{caster:?} to{target:?} failed: {error:?}");
    }
}

fn apply_destabilized(world: &mut World, target: Entity, aura: &crate::auras::AuraInstance) {
    if aura
        .effects
        .iter()
        .any(|effect| effect.index == 0 && effect.aura_type == crate::auras::handlers::DUMMY)
    {
        cast_ignoring_in_progress(world, target, target, SPELL_DESTABILIZED);
    }
}

fn apply_corrupted_faction(world: &mut World, target: Entity, aura: &crate::auras::AuraInstance) {
    // Local1230289 effect1 is MOD_FACTION243/misc16; consume this authored field.
    if let Some(effect) = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1 && effect.aura_type == 243)
    {
        world
            .entity_mut(target)
            .insert(UnitFactionTemplate(effect.misc_values[0] as u32))
            .remove::<super::ScriptedVictimUpdatePaused>();
    }
}

fn find_instance_actor(ctx: &mut Ctx, entry: u32) -> Option<Entity> {
    let map = ctx.map()?;
    ctx.world
        .query::<(Entity, &Npc, &InMap)>()
        .iter(ctx.world)
        .find_map(|(unit, npc, unit_map)| {
            (npc.template_id == entry && *unit_map == map).then_some(unit)
        })
}

/// TC's companion constructor sets unkillable; DATA_NIBBLES is not a completion boss.
#[derive(Default)]
struct Nibbles {
    events: EventMap,
}

impl Nibbles {
    fn check_health(&mut self, ctx: &mut Ctx) {
        let threshold = crate::spell_cast::scripts::script_effect_value(
            ctx.world,
            ctx.me,
            SPELL_ILLICIT_INFUSION,
            0,
        );
        let health = ctx.world.get::<Health>(ctx.me).expect("Nibbles health");
        if health.current * 100.0 / health.max < threshold {
            self.start_light_infusion(ctx);
            self.events.reschedule(EVENT_CHECK_HEALTH, 15_000);
        }
        // Pinned source schedules15s above and then repeats1s; both are retained.
        self.events.repeat(1_000);
    }

    fn start_light_infusion(&mut self, ctx: &mut Ctx) {
        let me = ctx.me;
        crate::auras::remove_auras(
            ctx.world,
            me,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == SPELL_ILLICIT_INFUSION,
        );
        crate::spell_cast::cancel_cast(ctx.world, me);
        super::attack_stop(ctx.world, me);
        ctx.world.entity_mut(me).insert((
            super::ScriptedVictimUpdatePaused,
            UnitFactionTemplate(FACTION_FRIENDLY),
            ModelDisplay {
                display_id: DISPLAY_FRIENDLY,
            },
        ));
        if let Some(kystia) = find_instance_actor(ctx, NPC_KYSTIA) {
            ctx.do_cast_at(Some(kystia), SPELL_LIGHT_INFUSION, false);
            ctx.talk(0);
        }
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event_id: u32) {
        match event_id {
            EVENT_FEL_SPRAY => {
                let target = ctx.select_target_random(0.0, false);
                if target.is_some() {
                    ctx.do_cast_at(target, SPELL_FEL_SPRAY, false);
                }
                ctx.talk(1);
                self.events.repeat(51_000);
            }
            EVENT_CORRODING_SPITTLE => {
                let target = ctx.select_target_random(0.0, false);
                if target.is_some() {
                    ctx.do_cast_at(target, SPELL_CORRODING_SPITTLE, false);
                }
                self.events.repeat(14_500);
            }
            EVENT_CHECK_HEALTH => self.check_health(ctx),
            _ => {}
        }
    }
}

impl CreatureAi for Nibbles {
    fn channel_finished(&mut self, ctx: &mut Ctx, spell_id: u32) {
        if spell_id == SPELL_LIGHT_INFUSION {
            if let Some(kystia) = find_instance_actor(ctx, NPC_KYSTIA) {
                cast_ignoring_in_progress(ctx.world, kystia, ctx.me, SPELL_ILLICIT_INFUSION_CAST);
            }
        }
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.events.reset();
        ctx.world
            .entity_mut(ctx.me)
            .remove::<super::ScriptedVictimUpdatePaused>()
            .insert(crate::combat::Unkillable);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        if let Some(map) = ctx.map() {
            instance_script::send_engage_unit_with_priority(ctx.world, map, ctx.me, 2);
        }
        if let Some(kystia) = find_instance_actor(ctx, NPC_KYSTIA) {
            ctx.do_zone_in_combat(kystia);
        }
        self.events.schedule(EVENT_CHECK_HEALTH, 1_000);
        self.events.schedule(EVENT_FEL_SPRAY, 8_000);
        self.events.schedule(EVENT_CORRODING_SPITTLE, 4_500);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        if let Some(map) = ctx.map() {
            instance_script::send_disengage_unit(ctx.world, map, ctx.me);
        }
        self.reset(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        let friendly = ctx
            .world
            .get::<UnitFactionTemplate>(ctx.me)
            .is_some_and(|faction| faction.0 == FACTION_FRIENDLY);
        if friendly || !ctx.update_victim() {
            return;
        }
        self.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        while let Some(event) = self.events.execute_event() {
            self.execute_event(ctx, event);
            if ctx.is_casting() {
                return;
            }
        }
    }
}

impl BossScript for KystiaManaheart {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event_id: u32) {
        match event_id {
            EVENT_CHAOS_BARRAGE => {
                // TC ignores cast-in-progress, not cast time/costs/school locks.
                // boss_update_ai already excludes an in-progress cast here.
                ctx.do_cast_at(ctx.victim(), SPELL_CHAOS_BARRAGE, false);
                self.boss.events.repeat(3_500);
            }
            EVENT_MIRROR_IMAGES => {
                ctx.talk(1);
                ctx.do_cast_at(Some(ctx.me), SPELL_MIRROR_IMAGES, false);
                self.boss.events.repeat(30_500);
            }
            EVENT_FEL_NOVA => {
                ctx.talk(2);
                ctx.do_cast(SPELL_FEL_NOVA_SELECTOR);
                self.boss.events.repeat(15_000);
            }
            _ => {}
        }
    }
}

impl CreatureAi for KystiaManaheart {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.do_cast_at(Some(ctx.me), SPELL_FEL_CRAZED, false);
        ctx.do_cast(SPELL_ILLICIT_INFUSION_VISUAL);
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.felshield_count = 0;
    }

    fn channel_finished(&mut self, ctx: &mut Ctx, spell_id: u32) {
        if spell_id != SPELL_ILLICIT_INFUSION_CAST {
            return;
        }
        let amount = 3_u8.wrapping_sub(self.felshield_count);
        let stacks = if amount == 0 { 1 } else { amount };
        cast_felshield_stacks(ctx.world, ctx.me, stacks);
        self.felshield_count = self.felshield_count.wrapping_add(1);
        if let Some(nibbles) = find_instance_actor(ctx, NPC_NIBBLES) {
            ctx.world.entity_mut(nibbles).insert(ModelDisplay {
                display_id: DISPLAY_UNFRIENDLY,
            });
            cast_ignoring_in_progress(ctx.world, nibbles, nibbles, SPELL_ILLICIT_INFUSION);
        }
    }

    fn spell_hit(&mut self, ctx: &mut Ctx, _caster: Entity, spell_id: u32) {
        if spell_id == SPELL_DESTABILIZED {
            ctx.talk(3);
        }
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(0);
        if let Some(nibbles) = find_instance_actor(ctx, NPC_NIBBLES) {
            ctx.do_zone_in_combat(nibbles);
        }
        self.boss.events.schedule(EVENT_CHAOS_BARRAGE, 1_000);
        self.boss.events.schedule(EVENT_MIRROR_IMAGES, 14_000);
        if self.heroic {
            self.boss.events.schedule(EVENT_FEL_NOVA, 12_000);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(5);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        ctx.talk(6);
        self.boss.reset(ctx);
        if let Some(nibbles) = find_instance_actor(ctx, NPC_NIBBLES) {
            crate::creature_aggro::enter_evade_mode(ctx.world, nibbles);
        }
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

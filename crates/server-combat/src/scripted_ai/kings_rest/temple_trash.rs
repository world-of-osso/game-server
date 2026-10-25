//! TC a352b1fa kings_rest.cpp `npc_kings_rest_temple_basic` and its Shadow-Borne
//! warrior (134157), witch doctor (134174) and champion (134158), the Minion of Zul
//! (133943) and its `spell_kings_rest_bound_by_shadow`.
use bevy::prelude::*;

use crate::scripted_ai::{CreatureAi, Ctx, EventMap};

pub(crate) const WARRIOR_SCRIPT: &str = "npc_kings_rest_shadow_borne_warrior";
pub(crate) const WITCH_DOCTOR_SCRIPT: &str = "npc_kings_rest_shadow_borne_witch_doctor";
pub(crate) const CHAMPION_SCRIPT: &str = "npc_kings_rest_shadow_borne_champion";
pub(crate) const MINION_OF_ZUL_SCRIPT: &str = "npc_kings_rest_minion_of_zul";

const SPELL_ZUL_SUMMON_VISUAL: u32 = 269619;
pub(super) const SPELL_BOUND_BY_SHADOW: u32 = 269935;
const SPELL_MINION_OF_ZUL_FIXATE: u32 = 269936;
const SPELL_GUST_SLASH: u32 = 269931;
const SPELL_DEATHLY_CHILL: u32 = 269973;
const SPELL_SHADOW_BOLT_VOLLEY: u32 = 269972;
const SPELL_ANCESTRAL_FURY: u32 = 269976;
const SPELL_VIGILANT_DEFENSE: u32 = 269928;

/// How an event casts its spell.
#[derive(Clone, Copy)]
enum Cast {
    /// `DoCast(spell)`.
    Default,
    /// `DoCastVictim(spell)`.
    Victim,
}

/// One `EventMap` event: first delay from `JustEngagedWith`, then its repeat.
struct Event {
    spell: u32,
    cast: Cast,
    first_ms: u64,
    repeat_ms: u64,
}

struct Schedule {
    events: &'static [Event],
    /// The witch doctor and champion re-check `UNIT_STATE_CASTING` inside the event
    /// loop; the warrior does not.
    stop_loop_while_casting: bool,
}

const WARRIOR: Schedule = Schedule {
    events: &[Event {
        spell: SPELL_GUST_SLASH,
        cast: Cast::Default,
        first_ms: 17_000,
        repeat_ms: 17_000,
    }],
    stop_loop_while_casting: false,
};

const WITCH_DOCTOR: Schedule = Schedule {
    events: &[
        Event {
            spell: SPELL_DEATHLY_CHILL,
            cast: Cast::Victim,
            first_ms: 5_000,
            repeat_ms: 2_500,
        },
        Event {
            spell: SPELL_SHADOW_BOLT_VOLLEY,
            cast: Cast::Default,
            first_ms: 17_000,
            repeat_ms: 16_000,
        },
    ],
    stop_loop_while_casting: true,
};

const CHAMPION: Schedule = Schedule {
    events: &[
        Event {
            spell: SPELL_ANCESTRAL_FURY,
            cast: Cast::Default,
            first_ms: 35_000,
            repeat_ms: 30_000,
        },
        Event {
            spell: SPELL_VIGILANT_DEFENSE,
            cast: Cast::Default,
            first_ms: 21_000,
            repeat_ms: 21_000,
        },
    ],
    stop_loop_while_casting: true,
};

pub(crate) fn is_script(name: &str) -> bool {
    matches!(
        name,
        WARRIOR_SCRIPT | WITCH_DOCTOR_SCRIPT | CHAMPION_SCRIPT | MINION_OF_ZUL_SCRIPT
    )
}

pub(crate) fn creature_ai(name: &str) -> Option<Box<dyn CreatureAi>> {
    let schedule = match name {
        MINION_OF_ZUL_SCRIPT => return Some(Box::new(MinionOfZul)),
        WARRIOR_SCRIPT => &WARRIOR,
        WITCH_DOCTOR_SCRIPT => &WITCH_DOCTOR,
        CHAMPION_SCRIPT => &CHAMPION,
        _ => return None,
    };
    Some(Box::new(ShadowBorne {
        schedule,
        events: EventMap::default(),
    }))
}

struct ShadowBorne {
    schedule: &'static Schedule,
    /// Event ids are 1-based indices into `schedule.events`.
    events: EventMap,
}

/// `npc_kings_rest_temple_basic::JustAppeared`.
fn cast_summon_visual(ctx: &mut Ctx) {
    ctx.do_cast_at(Some(ctx.me), SPELL_ZUL_SUMMON_VISUAL, false);
}

/// 133943: temple_basic's visual only when raised by the TempleEvent, otherwise
/// ScriptedAI's default melee.
struct MinionOfZul;

impl CreatureAi for MinionOfZul {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        let temple_event = ctx
            .world
            .get::<crate::zones::CreatureStringIds>(ctx.me)
            .is_some_and(|ids| ids.has("TempleEvent"));
        if temple_event {
            cast_summon_visual(ctx);
        }
        ctx.do_cast(SPELL_BOUND_BY_SHADOW);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        ctx.do_cast(SPELL_MINION_OF_ZUL_FIXATE);
    }
}

/// `spell_kings_rest_bound_by_shadow` AfterEffectRemove: a shield broken by an enemy
/// (`AURA_REMOVE_BY_ENEMY_SPELL`: absorb used up or dispelled) kills its bearer.
pub(super) fn kill_when_shadow_broken(
    world: &mut World,
    target: Entity,
    _aura: &crate::auras::AuraInstance,
    reason: crate::auras::RemoveReason,
) {
    use crate::auras::RemoveReason::{Consumed, Dispelled};
    if matches!(reason, Consumed | Dispelled) {
        crate::combat::kill_unit(world, target, target);
    }
}

impl CreatureAi for ShadowBorne {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        cast_summon_visual(ctx);
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.events.reset();
    }

    fn just_engaged_with(&mut self, _ctx: &mut Ctx, _who: Entity) {
        for (index, event) in self.schedule.events.iter().enumerate() {
            self.events.schedule(index as u32 + 1, event.first_ms);
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            return;
        }
        self.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        while let Some(id) = self.events.execute_event() {
            let event = &self.schedule.events[id as usize - 1];
            match event.cast {
                Cast::Default => ctx.do_cast(event.spell),
                Cast::Victim => {
                    let victim = ctx.victim();
                    ctx.do_cast_at(victim, event.spell, false);
                }
            }
            self.events.schedule(id, event.repeat_ms);
            if self.schedule.stop_loop_while_casting && ctx.is_casting() {
                return;
            }
        }
    }
}

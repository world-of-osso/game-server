//! TC a352b1fa boss_broggok.cpp: 17380 combat and out-of-combat action clock.
use bevy::prelude::*;
use shared::components::UnitFlags;

use super::boss_ai::BossAi;
use super::{CreatureAi, Ctx, ReactState};

pub(super) const SCRIPT: &str = "boss_broggok";
/// TC DATA_BROGGOK ordinal1 maps to installed encounter1924 completion bit2.
const BROGGOK_BIT: u32 = 2;
const SLIME_SPRAY: u32 = 30913;
const POISON_BOLT: u32 = 30917;
const POISON_CLOUD: u32 = 30916;
const SUMMON_INCOMBAT_TRIGGER: u32 = 26837;
const DESPAWN_INCOMBAT_TRIGGER: u32 = 26838;
const PATH_ROOM: u32 = 1381150;

pub(super) const ACTION_PERFORM_INTRO: i32 = 1738000;
pub(super) const ACTION_RELEASE_BROGGOK: i32 = 1738001;
pub(super) const ACTION_DESPAWN_TRIGGER: i32 = 1738002;

const EVENT_SLIME_SPRAY: u32 = 1;
const EVENT_POISON_BOLT: u32 = 2;
const EVENT_POISON_CLOUD: u32 = 3;
const EVENT_INTRO_1: u32 = 4;
const EVENT_INTRO_2: u32 = 5;
const EVENT_RELEASE_1: u32 = 6;
const EVENT_RELEASE_2: u32 = 7;

pub(super) fn creature_ai() -> Box<dyn CreatureAi> {
    Box::new(Broggok {
        boss: BossAi::new(BROGGOK_BIT),
    })
}

struct Broggok {
    boss: BossAi,
}

impl Broggok {
    fn set_released(ctx: &mut Ctx, released: bool) {
        let mut flags = *ctx.world.get::<UnitFlags>(ctx.me).expect("creature flags");
        if released {
            flags.0 &= !UnitFlags::NOT_SELECTABLE;
        } else {
            flags.0 |= UnitFlags::NOT_SELECTABLE;
        }
        ctx.world.entity_mut(ctx.me).insert(flags);
        ctx.set_react_state(if released {
            ReactState::Aggressive
        } else {
            ReactState::Passive
        });
    }

    fn update_out_of_combat(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.boss.events.update(u64::from(diff_ms));
        while let Some(event) = self.boss.events.execute_event() {
            match event {
                EVENT_INTRO_1 => {
                    ctx.do_cast_self(SUMMON_INCOMBAT_TRIGGER);
                    self.boss.events.schedule(EVENT_INTRO_2, 2_000);
                }
                EVENT_INTRO_2 => ctx.talk(0),
                EVENT_RELEASE_1 => {
                    Self::set_released(ctx, true);
                    self.boss.events.schedule(EVENT_RELEASE_2, 5_000);
                }
                EVENT_RELEASE_2 => {
                    ctx.talk(1);
                    super::waypoint_path::move_path(ctx.world, ctx.me, PATH_ROOM);
                }
                _ => {}
            }
        }
    }

    fn execute_combat_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_SLIME_SPRAY => {
                ctx.do_cast_self(SLIME_SPRAY);
                self.boss.events.repeat_range(10_000, 15_000);
            }
            EVENT_POISON_BOLT => {
                // BROGGOK_POISON_BOLT_30917_ABSENT_70448: no replacement spell.
                ctx.do_cast_self(POISON_BOLT);
                self.boss.events.repeat_range(10_000, 15_000);
            }
            EVENT_POISON_CLOUD => {
                // Native30916 creates AT33896, not historical cloud17662.
                ctx.do_cast_self(POISON_CLOUD);
                self.boss.events.repeat_range(10_000, 25_000);
            }
            _ => {}
        }
    }
}

impl CreatureAi for Broggok {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_appeared(&mut self, ctx: &mut Ctx) {
        Self::set_released(ctx, false);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        // BROGGOK_DESPAWN_TRIGGER_26838_ABSENT_70448.
        ctx.do_cast_self(DESPAWN_INCOMBAT_TRIGGER);
        self.boss
            .events
            .schedule_range(EVENT_SLIME_SPRAY, 5_000, 15_000);
        self.boss.events.schedule_range(EVENT_POISON_BOLT, 0, 5_000);
        self.boss
            .events
            .schedule_range(EVENT_POISON_CLOUD, 5_000, 20_000);
    }

    fn just_summoned(&mut self, _ctx: &mut Ctx, _summon: Entity) {
        // Source explicitly overrides BossAI: no storage, zone-combat or cleanup.
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn waypoint_path_ended(&mut self, ctx: &mut Ctx, _node_id: u32, _path_id: u32) {
        ctx.do_zone_in_combat(ctx.me);
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        match action {
            ACTION_PERFORM_INTRO => self.boss.events.schedule(EVENT_INTRO_1, 0),
            ACTION_RELEASE_BROGGOK => self.boss.events.schedule(EVENT_RELEASE_1, 0),
            ACTION_DESPAWN_TRIGGER => ctx.do_cast_self(DESPAWN_INCOMBAT_TRIGGER),
            _ => {}
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            self.update_out_of_combat(ctx, diff_ms);
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        while let Some(event) = self.boss.events.execute_event() {
            self.execute_combat_event(ctx, event);
            if ctx.is_casting() {
                return;
            }
        }
    }
}

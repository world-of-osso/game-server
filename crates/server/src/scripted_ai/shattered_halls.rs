//! Shattered Halls bosses, TC a352b1fa boss_warbringer_omrogg.cpp.
use bevy::prelude::*;
use shared::components::{EquipmentAppearance, Npc};

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, Hook};
use crate::auras::AuraInstance;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::maps::InMap;

const INSTANCE: &str = "instance_shattered_halls";
const OMROGG: &str = "boss_warbringer_omrogg";
const HEADS: &str = "npc_omrogg_heads";
/// TC DATA_OMROGG: installed DB2 encounter1937, completion bit2.
const DATA_OMROGG: u32 = 2;
const LEFT_HEAD: u32 = 19_523;
const RIGHT_HEAD: u32 = 19_524;

const FEAR: u32 = 30_584;
const THUNDERCLAP: u32 = 30_633;
pub(crate) const BEATDOWN: u32 = 30_618;
const BURNING_MAUL: u32 = 30_598;
const BURNING_MAUL_H: u32 = 36_056;
const BLAST_WAVE: u32 = 30_600;
/// TC OmroggTexts EMOTE_ROAR.
const EMOTE_ROAR: u8 = 0;
/// TC EQUIP_ID_BURNING_MAUL.
const EQUIP_ID_BURNING_MAUL: u8 = 2;
/// TC spell_omrogg_burning_maul OnProc: roll_chance(30).
const BLAST_WAVE_CHANCE: u32 = 30;
/// TC npc_omrogg_heads ACTION_DEATH: DespawnOrUnsummon(4s) of both heads.
const HEADS_DESPAWN_MS: u32 = 4_000;

/// TC OmroggActions.
const ACTION_AGGRO: [i32; 3] = [1, 2, 3];
const ACTION_ATTACK: [i32; 4] = [4, 5, 6, 7];
const ACTION_SLAY: [i32; 2] = [8, 9];
const ACTION_DEATH: i32 = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Head {
    Left,
    Right,
}

/// One head line: who says text `group`, `delay_ms` after the previous line.
type Line = (Head, u8, u32);

/// TC npc_omrogg_heads::DoAction TaskScheduler sequences, keyed by action.
fn sequence(action: i32) -> &'static [Line] {
    use Head::{Left as L, Right as R};
    match action {
        1 => &[(L, 0, 0), (R, 0, 3_000), (L, 1, 3_000)],
        2 => &[(L, 2, 0), (R, 1, 3_000)],
        3 => &[(L, 3, 0), (R, 2, 3_000)],
        4 => &[(L, 4, 0), (R, 3, 3_000), (L, 5, 3_000)],
        5 => &[(L, 6, 0), (R, 4, 3_000)],
        6 => &[(L, 7, 0), (R, 5, 3_000), (L, 8, 3_000)],
        7 => &[(R, 6, 0), (L, 9, 2_000), (R, 7, 2_000)],
        8 => &[(R, 8, 0)],
        9 => &[(L, 10, 0), (R, 9, 3_000)],
        ACTION_DEATH => &[(L, 11, 0), (R, 10, 3_000)],
        _ => &[],
    }
}

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(name, OMROGG | HEADS)
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<InMap>(actor)?;
    let instance = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    if instance != INSTANCE {
        return None;
    }
    let ai: Box<dyn CreatureAi> = if world.get::<Npc>(actor)?.template_id == LEFT_HEAD
        || world.get::<Npc>(actor)?.template_id == RIGHT_HEAD
    {
        Box::new(OmroggHead::default())
    } else {
        Box::new(Omrogg {
            boss: BossAi::new(DATA_OMROGG),
        })
    };
    Some(ai)
}

/// The same-copy creature of `entry` (TC instance->GetCreature(DATA_*_HEAD)).
fn find_in_copy(world: &mut World, map: InMap, entry: u32) -> Option<Entity> {
    world
        .query::<(Entity, &InMap, &Npc)>()
        .iter(world)
        .find_map(|(actor, copy, npc)| (*copy == map && npc.template_id == entry).then_some(actor))
}

fn pick<T: Copy>(choices: &[T]) -> T {
    choices[rand::Rng::gen_range(&mut rand::thread_rng(), 0..choices.len())]
}

/// TC `leftHead->AI()->DoAction(action)`.
fn left_head_action(ctx: &mut Ctx, action: i32) {
    let Some(map) = ctx.map() else {
        return;
    };
    if let Some(head) = find_in_copy(ctx.world, map, LEFT_HEAD) {
        super::dispatch(ctx.world, head, Hook::DoAction(action));
    }
}

struct Omrogg {
    boss: BossAi,
}

impl CreatureAi for Omrogg {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        #[cfg(test)]
        if let Some(seed) = ctx
            .world
            .get::<super::tests::cast_admission::GroundCombatSeed>(ctx.me)
        {
            self.boss.events.seed_rng(seed.0);
        }
        let events = &mut self.boss.events;
        events.schedule_range(FEAR, 20_000, 30_000);
        events.schedule_range(THUNDERCLAP, 15_000, 25_000);
        events.schedule_range(BEATDOWN, 25_000, 30_000);
        events.schedule_range(BURNING_MAUL, 50_000, 60_000);
        left_head_action(ctx, pick(&ACTION_AGGRO));
    }

    fn on_spell_cast(&mut self, ctx: &mut Ctx, spell_id: u32) {
        match spell_id {
            BEATDOWN => left_head_action(ctx, pick(&ACTION_ATTACK)),
            BURNING_MAUL => ctx.talk(EMOTE_ROAR),
            _ => {}
        }
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, _victim: Entity) {
        left_head_action(ctx, pick(&ACTION_SLAY));
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        left_head_action(ctx, ACTION_DEATH);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Omrogg {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let (min, max) = match event {
            FEAR => (15_000, 35_000),
            THUNDERCLAP => (15_000, 30_000),
            BEATDOWN => (25_000, 40_000),
            BURNING_MAUL => (60_000, 70_000),
            _ => return,
        };
        ctx.do_cast_self(event);
        self.boss.events.repeat_range(min, max);
    }
}

/// A pending head sequence: its remaining lines and the wait before the next.
struct Pending {
    lines: &'static [Line],
    wait_ms: u32,
    /// ACTION_DEATH: both heads despawn4s after its last line.
    despawns: bool,
}

/// TC npc_omrogg_heads: the left head runs every sequence; the right head only speaks.
#[derive(Default)]
struct OmroggHead {
    pending: Vec<Pending>,
    despawn_in_ms: Option<u32>,
}

impl OmroggHead {
    fn speak(ctx: &mut Ctx, head: Head, group: u8) {
        let speaker = match head {
            Head::Left => Some(ctx.me),
            Head::Right => ctx
                .map()
                .and_then(|map| find_in_copy(ctx.world, map, RIGHT_HEAD)),
        };
        if let Some(speaker) = speaker {
            super::creature_text::talk(ctx.world, speaker, group);
        }
    }

    /// Speak every due line; true once a death sequence spoke its last line.
    fn advance_sequences(&mut self, ctx: &mut Ctx, diff_ms: u32) -> bool {
        let mut death_finished = false;
        for pending in &mut self.pending {
            pending.wait_ms = pending.wait_ms.saturating_sub(diff_ms);
            while pending.wait_ms == 0 && !pending.lines.is_empty() {
                let (head, group, _) = pending.lines[0];
                Self::speak(ctx, head, group);
                pending.lines = &pending.lines[1..];
                pending.wait_ms = pending.lines.first().map_or(0, |line| line.2);
                death_finished |= pending.despawns && pending.lines.is_empty();
            }
        }
        self.pending.retain(|pending| !pending.lines.is_empty());
        death_finished
    }

    fn despawn_heads(ctx: &mut Ctx) {
        let right = ctx
            .map()
            .and_then(|map| find_in_copy(ctx.world, map, RIGHT_HEAD));
        crate::combat::despawn_creature(ctx.world, ctx.me);
        if let Some(right) = right {
            crate::combat::despawn_creature(ctx.world, right);
        }
    }
}

impl CreatureAi for OmroggHead {
    fn do_action(&mut self, _ctx: &mut Ctx, action: i32) {
        let lines = sequence(action);
        if !lines.is_empty() {
            self.pending.push(Pending {
                lines,
                wait_ms: 0,
                despawns: action == ACTION_DEATH,
            });
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if self.advance_sequences(ctx, diff_ms) {
            self.despawn_in_ms = Some(HEADS_DESPAWN_MS);
        }
        if let Some(remaining) = self.despawn_in_ms {
            let remaining = remaining.saturating_sub(diff_ms);
            self.despawn_in_ms = Some(remaining);
            if remaining == 0 {
                Self::despawn_heads(ctx);
            }
        }
    }
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    for spell in [BURNING_MAUL, BURNING_MAUL_H] {
        scripts.0.insert(
            spell,
            AuraScript {
                on_apply: Some(wield_burning_maul),
                on_remove: Some(restore_original_weapon),
                on_proc: Some(maybe_blast_wave),
                ..Default::default()
            },
        );
    }
}

/// `Creature::LoadEquipment(id)`: the entry's equipment template `id`.
fn load_equipment(world: &mut World, unit: Entity, id: u8) {
    let Some(entry) = world.get::<Npc>(unit).map(|npc| npc.template_id) else {
        return;
    };
    let equipment = world
        .resource::<crate::world_data::WorldData>()
        .equipment
        .get(&entry)
        .and_then(|ids| ids.get(&id))
        .cloned();
    match equipment {
        Some(equipment) => {
            world.entity_mut(unit).insert(equipment);
        }
        None => {
            world.entity_mut(unit).remove::<EquipmentAppearance>();
        }
    }
}

/// `Creature::GetOriginalEquipmentId`: the spawn row's `equipment_id`.
fn original_equipment_id(world: &World, unit: Entity) -> Option<u8> {
    let guid = world.get::<crate::zones::CreatureOrigin>(unit)?.guid;
    let map = world.get::<InMap>(unit)?.map_id as u16;
    let data = world.resource::<crate::world_data::WorldData>();
    data.spawns_by_map
        .get(&map)?
        .iter()
        .map(|&index| &data.spawns[index])
        .find(|spawn| spawn.guid == guid)
        .and_then(|spawn| u8::try_from(spawn.equipment_id).ok())
}

/// TC spell_omrogg_burning_maul OnApply.
fn wield_burning_maul(world: &mut World, target: Entity, _aura: &AuraInstance) {
    load_equipment(world, target, EQUIP_ID_BURNING_MAUL);
}

/// TC spell_omrogg_burning_maul OnRemove.
fn restore_original_weapon(
    world: &mut World,
    target: Entity,
    _aura: &AuraInstance,
    _reason: crate::auras::RemoveReason,
) {
    if let Some(id) = original_equipment_id(world, target) {
        load_equipment(world, target, id);
    }
}

/// TC spell_omrogg_burning_maul OnProc: besides the default36057 trigger.
fn maybe_blast_wave(
    world: &mut World,
    target: Entity,
    _aura: &AuraInstance,
    _other: Option<Entity>,
) {
    let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 0..100);
    if roll < BLAST_WAVE_CHANCE
        && let Err(error) = crate::spell_cast::creature_cast(world, target, BLAST_WAVE, None, false)
    {
        debug!(?target, ?error, "Burning Maul Blast Wave30600 not cast");
    }
}

/// TC spell_omrogg_beatdown AfterCast: ResetAllThreat. Its AttackStart(random
/// player) adds no reference to an engaged creature; the threat victim decides.
pub(crate) fn beatdown_reset_threat(
    world: &mut World,
    caster: Entity,
    _spell: &crate::spell_info::SpellInfo,
) {
    if world.get::<Npc>(caster).is_some() {
        crate::creature_aggro::reset_all_threat(world, caster);
    }
}

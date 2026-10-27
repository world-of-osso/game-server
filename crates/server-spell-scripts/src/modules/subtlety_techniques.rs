//! SimC76f8118 sc_rogue.cpp8130–8211: landed whites store points;
//! builders collect only the capacity remaining after their ordinary grants.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::random::{self, Rng};
use combat::spell_cast::{self, effects, powers, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, PowerType};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterWhiteHit(store_from_white),
    SpellScriptHook::BeforeNativeOnCast(begin_builder),
    SpellScriptHook::ComboPointGain(record_builder_gain),
    SpellScriptHook::AfterForegroundSpecial(finish_builder_launch),
    SpellScriptHook::AfterResolvedHits(collect_after_builder_hits),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

const PASSIVE: u32 = 196912;
const STORAGE: u32 = 196911;
const DANCE: u32 = 185313;
const SHADOWCRAFT: u32 = 426594;
const ANCIENT_ARTS: u32 = 1268939;
const ANCIENT_ARTS_BUFF: u32 = 1269163;

#[derive(Component)]
struct WhiteProgress {
    hits: u32,
    ready_at: f64,
}

#[derive(Component)]
struct BuilderGain {
    spell: u32,
    gained: bool,
    launched: bool,
}

fn owns_proc(spell: u32) -> bool {
    matches!(spell, PASSIVE | STORAGE)
}

fn is_subtlety(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 261)
        && auras::has_aura(world, caster, PASSIVE)
}

fn store_from_white(world: &mut World, caster: Entity, _: bool) {
    if !is_subtlety(world, caster) {
        return;
    }
    let shadowcraft =
        has_talent(world, caster, SHADOWCRAFT) && auras::has_aura(world, caster, DANCE);
    let upper = if shadowcraft { 2 } else { 3 };
    let now = spell_cast::now(world);
    if !advance_white_counter(world, caster, upper, now) {
        return;
    }
    let mut spell = world
        .resource::<SpellStore>()
        .get(STORAGE)
        .expect("native Shadow Techniques storage/energy spell")
        .clone();
    let cooldown = spell
        .aura_options
        .as_ref()
        .expect("native Shadow Techniques ICD")
        .proc_category_recovery_ms;
    world.entity_mut(caster).insert(WhiteProgress {
        hits: 0,
        ready_at: now + f64::from(cooldown) / 1000.0,
    });
    // One cast grants native Energy once, even when Shadowcraft earns two stacks.
    spell.cast_aura_stacks = Some(if shadowcraft { 2 } else { 1 });
    spell_cast::launch_triggered_spell(world, caster, &spell, Some(caster), 1);
}

fn advance_white_counter(world: &mut World, caster: Entity, upper: u32, now: f64) -> bool {
    if world.get::<WhiteProgress>(caster).is_none() {
        // sc_rogue.cpp11632–11634: the initial phase is uniform in [0,4).
        let hits = random::thread_rng().gen_range(0..4);
        world.entity_mut(caster).insert(WhiteProgress {
            hits,
            ready_at: 0.0,
        });
    }
    let mut progress = world.get_mut::<WhiteProgress>(caster).unwrap();
    progress.hits += 1;
    let threshold = progress.hits >= upper || (progress.hits == 2 && random::random::<f64>() < 0.5);
    threshold && now >= progress.ready_at
}

fn read_storage(world: &World, caster: Entity) -> Option<(u32, u8)> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == STORAGE && aura.caster == caster)
        .map(|aura| (aura.instance_id, aura.stacks))
}

fn begin_builder(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if depth == 0 && is_subtlety(world, caster) {
        world.entity_mut(caster).insert(BuilderGain {
            spell: spell.id,
            gained: false,
            launched: false,
        });
    }
}

fn record_builder_gain(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if depth != 0 {
        return;
    }
    if let Some(mut pending) = world.get_mut::<BuilderGain>(caster)
        && pending.spell == spell
    {
        pending.gained = true;
    }
}

fn finish_builder_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if let Some(mut pending) = world.get_mut::<BuilderGain>(caster)
        && pending.spell == spell.id
    {
        pending.launched = true;
    }
    collect_finished_builder(world, caster, spell.id);
}

fn collect_after_builder_hits(world: &mut World, ctx: &effects::EffectCtx<'_>) {
    if ctx.depth == 0 {
        collect_finished_builder(world, ctx.caster, ctx.spell.id);
    }
}

fn collect_finished_builder(world: &mut World, caster: Entity, spell: u32) {
    let ready = world
        .get::<BuilderGain>(caster)
        .is_some_and(|pending| pending.spell == spell && pending.gained && pending.launched);
    if !ready || spell_cast::has_pending_spell_hit(world, caster, spell) {
        return;
    }
    world.entity_mut(caster).remove::<BuilderGain>();
    let Some((instance, stored)) = read_storage(world, caster) else {
        return;
    };
    // SimC76f8118 sc_rogue.cpp2705–2716: all ordinary per-hit grants
    // precede storage collection, even when native hits arrive after launch.
    let (current, maximum) = powers::power_value(world, caster, PowerType::ComboPoints)
        .expect("native Rogue combo-point pool");
    let consumed = stored.min((maximum - current).max(0.0) as u8);
    collect_points(world, caster, instance, consumed);
    grant_ancient_arts(world, caster);
}

fn collect_points(world: &mut World, caster: Entity, instance: u32, consumed: u8) {
    if consumed == 0 {
        return;
    }
    let spell = world.resource::<SpellStore>().get(STORAGE).unwrap().clone();
    let ctx = effects::EffectCtx {
        caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::energize_unit(
        world,
        &ctx,
        caster,
        PowerType::ComboPoints,
        f32::from(consumed),
        false,
    );
    for _ in 0..consumed {
        auras::spend_stack(world, caster, instance);
    }
}

fn grant_ancient_arts(world: &mut World, caster: Entity) {
    if !has_talent(world, caster, ANCIENT_ARTS) {
        return;
    }
    let remaining = read_storage(world, caster).map_or(0, |(_, stacks)| stacks);
    let threshold = spell_cast::scripts::script_effect_value(world, caster, ANCIENT_ARTS, 0);
    if f32::from(remaining) >= threshold {
        spell_cast::cast_triggered(world, caster, ANCIENT_ARTS_BUFF, Some(caster), 1);
    }
}

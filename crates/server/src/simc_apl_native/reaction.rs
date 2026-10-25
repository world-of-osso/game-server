//! Explicit SimC profile reaction policy, observed at native aura-stack boundaries.
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// Seconds from the pinned reference player's reaction fields; no implicit defaults.
#[derive(Clone, Copy)]
pub struct ReactionSettings {
    pub mean: f64,
    pub stddev: f64,
    pub offset: f64,
    pub nu: f64,
    pub maximum: f64,
}

impl ReactionSettings {
    fn validate(self) -> Result<Self, String> {
        let fields = [self.mean, self.stddev, self.offset, self.nu, self.maximum];
        if fields
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("native APL reaction settings must be finite nonnegative seconds".into());
        }
        Ok(self)
    }

    fn sample(self) -> f64 {
        // rng.hpp gauss(timespan) truncates below zero; exgauss adds an exponential.
        let normal = sample_nonnegative_normal(self.mean, self.stddev);
        let uniform = crate::random::random::<f64>();
        let exponential = -(1.0 - uniform).ln() * self.nu;
        (self.offset + normal + exponential).min(self.maximum)
    }
}

fn sample_nonnegative_normal(mean: f64, stddev: f64) -> f64 {
    if stddev == 0.0 {
        return mean;
    }
    loop {
        let radial = 1.0 - crate::random::random::<f64>();
        let angle = std::f64::consts::TAU * crate::random::random::<f64>();
        let sample = mean + stddev * (-2.0 * radial.ln()).sqrt() * angle.cos();
        if sample >= 0.0 {
            return sample;
        }
    }
}

#[derive(Component)]
struct BuffReactions {
    settings: ReactionSettings,
    spells: BTreeSet<u32>,
    deadlines: BTreeMap<u32, Vec<f64>>,
    last_triggers: BTreeMap<u32, f64>,
}

/// Configure before tracked buffs exist: a late query cannot reconstruct acquisition.
pub fn configure(
    world: &mut World,
    actor: Entity,
    settings: ReactionSettings,
    spells: &[u32],
) -> Result<(), String> {
    let settings = settings.validate()?;
    let spells: BTreeSet<_> = spells.iter().copied().collect();
    if world.get::<BuffReactions>(actor).is_some() {
        return Err("native APL buff reaction policy already configured".into());
    }
    let active = world
        .get::<crate::auras::ActiveAuras>(actor)
        .is_some_and(|auras| auras.iter().any(|aura| spells.contains(&aura.spell_id)));
    if active {
        return Err("native APL reaction policy must precede tracked aura acquisition".into());
    }
    world.entity_mut(actor).insert(BuffReactions {
        settings,
        spells,
        deadlines: BTreeMap::new(),
        last_triggers: BTreeMap::new(),
    });
    Ok(())
}

pub(crate) fn applied(world: &mut World, target: Entity, aura: &crate::auras::AuraInstance) {
    let now = crate::spell_cast::now(world);
    let Some(mut reactions) = world.get_mut::<BuffReactions>(target) else {
        return;
    };
    if !reactions.spells.contains(&aura.spell_id) {
        return;
    }
    // SimC76f8118 buff.cpp2179–2189: every successful trigger, including
    // an overcap refresh, updates the clock; removal does not reset it.
    reactions.last_triggers.insert(aura.spell_id, now);
    let settings = reactions.settings;
    let deadlines = reactions.deadlines.entry(aura.instance_id).or_default();
    let stacks = usize::from(aura.stacks.max(1));
    deadlines.truncate(stacks);
    if stacks > deadlines.len() {
        // buff.cpp2697–2703: one draw for all stacks gained by this application;
        // existing stacks keep deadlines even on an overcap duration refresh.
        deadlines.resize(stacks, now + settings.sample());
    }
}

pub(crate) fn removed(world: &mut World, target: Entity, instance: u32) {
    if let Some(mut reactions) = world.get_mut::<BuffReactions>(target) {
        reactions.deadlines.remove(&instance);
    }
}

pub(crate) fn stacks_changed(world: &mut World, target: Entity, instance: u32, stacks: u8) {
    if let Some(mut reactions) = world.get_mut::<BuffReactions>(target)
        && let Some(deadlines) = reactions.deadlines.get_mut(&instance)
    {
        deadlines.truncate(usize::from(stacks));
    }
}

fn active_instances(world: &World, actor: Entity, spell: u32) -> Vec<u32> {
    let now = crate::spell_cast::now(world);
    let Some(auras) = world.get::<crate::auras::ActiveAuras>(actor) else {
        return Vec::new();
    };
    auras
        .iter()
        .filter(|aura| {
            aura.spell_id == spell
                && aura.caster == actor
                && aura.expires_at.is_none_or(|at| at > now)
        })
        .map(|aura| aura.instance_id)
        .collect()
}

/// SimC76f8118 buff.cpp3001–3004: prediction makes existing acquired stacks react now.
pub(crate) fn predict(world: &mut World, actor: Entity, spell: u32) {
    let instances = active_instances(world, actor, spell);
    let now = crate::spell_cast::now(world);
    if let Some(mut reactions) = world.get_mut::<BuffReactions>(actor) {
        for instance in instances {
            if let Some(deadlines) = reactions.deadlines.get_mut(&instance) {
                deadlines.fill(now);
            }
        }
    }
}

pub(crate) fn has_observed_reaction(world: &World, actor: Entity, spell: u32) -> bool {
    let Some(reactions) = world.get::<BuffReactions>(actor) else {
        return false;
    };
    let now = crate::spell_cast::now(world);
    active_instances(world, actor, spell)
        .iter()
        .any(|instance| {
            reactions
                .deadlines
                .get(instance)
                .is_some_and(|deadlines| deadlines.iter().any(|at| *at <= now))
        })
}

pub(super) fn last_trigger(
    world: &World,
    actor: Entity,
    spell: u32,
) -> Result<Option<f64>, String> {
    let reactions = world
        .get::<BuffReactions>(actor)
        .filter(|policy| policy.spells.contains(&spell))
        .ok_or("NATIVE_BUFF_TRIGGER_UNAVAILABLE: configure before native acquisition")?;
    Ok(reactions.last_triggers.get(&spell).copied())
}

pub(super) fn reacted_stacks(
    world: &World,
    actor: Entity,
    spell: u32,
    active: &[&crate::auras::AuraInstance],
) -> Result<f64, String> {
    if active.is_empty() {
        return Ok(0.0);
    }
    let reactions = world.get::<BuffReactions>(actor)
        .filter(|policy| policy.spells.contains(&spell))
        .ok_or("NATIVE_BUFF_REACTION_UNAVAILABLE: configure source/profile settings before acquisition")?;
    let now = crate::spell_cast::now(world);
    let mut reacted = 0;
    for aura in active {
        let deadlines = reactions
            .deadlines
            .get(&aura.instance_id)
            .ok_or("NATIVE_BUFF_REACTION_UNAVAILABLE: native acquisition was not observed")?;
        // buff.cpp2004–2015 searches downward: a later stack can react sooner
        // than an older stack because separate acquisitions draw separate delays.
        let count = deadlines
            .iter()
            .take(usize::from(aura.stacks.max(1)))
            .enumerate()
            .rfind(|(_, deadline)| **deadline <= now)
            .map_or(0, |(index, _)| index + 1);
        reacted = reacted.max(count);
    }
    Ok(reacted as f64)
}

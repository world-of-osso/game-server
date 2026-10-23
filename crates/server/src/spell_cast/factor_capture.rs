//! Opt-in test observations at actual calculation boundaries; never rerolls damage.
use super::{DamageInput, EffectCtx, SpellEffect, SpellInfo};
use bevy::prelude::*;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;

type HitKey = (u32, u64);

#[derive(Clone, Default)]
struct Formula {
    factors: Map<String, Value>,
    sources: Vec<Value>,
}

#[derive(Clone, Copy)]
pub(crate) struct Powers {
    pub attack_power: f32,
    pub spell_power: f32,
    pub ap_coefficient: f32,
    pub sp_coefficient: f32,
    pub ap_term: f32,
    pub sp_term: f32,
}

pub(crate) struct ModifierSource {
    pub op: u8,
    pub aura_spell_id: u32,
    pub effect_index: Option<u8>,
    pub amount: f32,
    pub percent: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Critical {
    pub chance: f32,
    pub before: f32,
    pub after: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct Source {
    pub input: f32,
    pub scripted: f32,
    pub modded: f32,
    pub done: f32,
    pub after: f32,
    pub skipped_caster_modifiers: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Target {
    pub multiplier: f32,
    pub cast_multiplier: f32,
    pub unrounded: f32,
    pub rounded: f32,
}

#[derive(Clone, Copy)]
struct Armor {
    value: f32,
    before: f32,
    after: f32,
}

#[derive(Default)]
struct Hit {
    chance: Option<(f32, f32)>,
    critical: Option<Critical>,
    source: Option<Source>,
    done_components: Vec<Value>,
    target: Option<Target>,
    armor: Option<Armor>,
    mitigated: Option<f32>,
}

#[derive(Default)]
struct State {
    formulas: BTreeMap<(u32, u8), Formula>,
    modifiers: BTreeMap<u32, Vec<Value>>,
    hits: BTreeMap<HitKey, Hit>,
    events: Vec<Value>,
    completed: usize,
}

#[derive(Resource)]
pub(crate) struct FactorCapture {
    actor: Entity,
    target: Entity,
    focus_spell: u32,
    action: String,
    limit: usize,
    state: Mutex<State>,
}

impl FactorCapture {
    pub(crate) fn new(
        actor: Entity,
        target: Entity,
        spell: u32,
        action: &str,
        limit: usize,
    ) -> Self {
        Self {
            actor,
            target,
            focus_spell: spell,
            action: action.into(),
            limit,
            state: Mutex::default(),
        }
    }

    pub(crate) fn export(&self) -> Value {
        let state = self.state.lock().expect("native factor capture poisoned");
        let modifiers = state
            .events
            .iter()
            .filter_map(|event| event["applied_modifiers"].as_array())
            .flatten()
            .map(|row| (row.to_string(), row.clone()))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect::<Vec<_>>();
        json!({"schema_version": 1, "events": state.events, "modifiers": modifiers,
            "summary": {}, "unparsed_relevant_lines": [], "captured_hits": state.completed,
            "limitations": ["Effective products are observed stage ratios, not a second damage formula.",
                "Crit is applied before source/target modifiers in the native pipeline; amount/raw_amount are explicitly crit-normalized observations.",
                "Aggregated native aura-stat/script products without source IDs are labelled unattributed product."]})
    }
}

fn observe(world: &World, actor: Entity, update: impl FnOnce(&FactorCapture, &mut State)) {
    let Some(capture) = world.get_resource::<FactorCapture>() else {
        return;
    };
    if actor != capture.actor {
        return;
    }
    let mut state = capture
        .state
        .lock()
        .expect("native factor capture poisoned");
    if state.completed < capture.limit {
        update(capture, &mut state);
    }
}

fn number(value: f32) -> Value {
    assert!(value.is_finite(), "nonfinite native factor: {value}");
    json!(value)
}

fn ratio(numerator: f32, denominator: f32) -> Option<f32> {
    assert!(
        numerator.is_finite() && denominator.is_finite(),
        "nonfinite native ratio"
    );
    (denominator != 0.0).then(|| {
        let value = numerator / denominator;
        assert!(value.is_finite(), "nonfinite native ratio result");
        value
    })
}

pub(crate) fn begin_formula(world: &World, actor: Entity, spell: &SpellInfo, effect: &SpellEffect) {
    observe(world, actor, |_, state| {
        state
            .formulas
            .insert((spell.id, effect.index), Formula::default());
        state.modifiers.remove(&spell.id);
        state.hits.retain(|(id, _), _| *id != spell.id);
    });
}

pub(crate) fn record_power(
    world: &World,
    actor: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    powers: Powers,
) {
    observe(world, actor, |_, state| {
        let formula = state.formulas.entry((spell.id, effect.index)).or_default();
        for (key, value) in [
            ("a_power", powers.attack_power),
            ("s_power", powers.spell_power),
            ("a_mod", powers.ap_coefficient),
            ("s_mod", powers.sp_coefficient),
            ("ap_term", powers.ap_term),
            ("sp_term", powers.sp_term),
        ] {
            formula.factors.insert(key.into(), number(value));
        }
    });
}

pub(crate) fn record_formula(
    world: &World,
    actor: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    base: f32,
    scaled: f32,
) {
    observe(world, actor, |_, state| {
        let sources = state.modifiers.get(&spell.id).cloned().unwrap_or_default();
        let formula = state.formulas.entry((spell.id, effect.index)).or_default();
        formula.factors.insert("base".into(), number(base));
        formula
            .factors
            .insert("native_scaled_amount".into(), number(scaled));
        formula.sources = sources;
    });
}

pub(crate) fn record_modifier(
    world: &World,
    actor: Entity,
    spell: &SpellInfo,
    source: ModifierSource,
) {
    if world
        .get_resource::<FactorCapture>()
        .is_none_or(|capture| capture.actor != actor)
    {
        return;
    }
    let name = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(source.aura_spell_id))
        .map(|spell| spell.name.clone());
    observe(world, actor, |_, state| {
        let row = json!({"spell_id": spell.id, "source_spell_id": source.aura_spell_id,
            "source_name": name, "effect_index": source.effect_index.map(|index| u32::from(index) + 1),
            "native_effect_index": source.effect_index, "op": source.op, "value": number(source.amount),
            "unit": if source.percent { "percent" } else { "native_operation_units" },
            "application": "native_applied", "index_convention": "effect_index is1-based; native_effect_index is0-based"});
        state.modifiers.entry(spell.id).or_default().push(row);
    });
}

pub(crate) fn record_chance(
    world: &World,
    actor: Entity,
    target: Entity,
    spell: &SpellInfo,
    base: f32,
    target_bonus: f32,
) {
    observe(world, actor, |capture, state| {
        if target == capture.target {
            state
                .hits
                .entry((spell.id, target.to_bits()))
                .or_default()
                .chance = Some((base, target_bonus));
        }
    });
}

pub(crate) fn record_critical(
    world: &World,
    actor: Entity,
    target: Entity,
    spell: &SpellInfo,
    critical: Critical,
) {
    observe(world, actor, |capture, state| {
        if target == capture.target {
            state
                .hits
                .entry((spell.id, target.to_bits()))
                .or_default()
                .critical = Some(critical);
        }
    });
}

pub(crate) fn record_done(
    world: &World,
    ctx: &EffectCtx,
    target: Entity,
    components: &[(&str, f32)],
) {
    observe(world, ctx.caster, |capture, state| {
        if target != capture.target {
            return;
        }
        state.hits.entry((ctx.spell.id, target.to_bits())).or_default().done_components = components.iter()
            .map(|(name, value)| json!({"factor": name, "value": number(*value), "attribution": "unattributed product", "sources": []})).collect();
    });
}

pub(crate) fn record_source(world: &World, ctx: &EffectCtx, target: Entity, source: Source) {
    observe(world, ctx.caster, |capture, state| {
        if target == capture.target {
            state
                .hits
                .entry((ctx.spell.id, target.to_bits()))
                .or_default()
                .source = Some(source);
        }
    });
}

pub(crate) fn record_target(world: &World, ctx: &EffectCtx, target: Entity, terms: Target) {
    observe(world, ctx.caster, |capture, state| {
        if target == capture.target {
            state
                .hits
                .entry((ctx.spell.id, target.to_bits()))
                .or_default()
                .target = Some(terms);
        }
    });
}

pub(crate) fn record_armor(
    world: &World,
    actor: Entity,
    target: Entity,
    spell: &SpellInfo,
    before: f32,
    armor: f32,
    after: f32,
) {
    observe(world, actor, |capture, state| {
        if target == capture.target {
            state
                .hits
                .entry((spell.id, target.to_bits()))
                .or_default()
                .armor = Some(Armor {
                value: armor,
                before,
                after,
            });
        }
    });
}

pub(crate) fn record_mitigated(world: &World, ctx: &EffectCtx, target: Entity, amount: f32) {
    observe(world, ctx.caster, |capture, state| {
        if target == capture.target {
            state
                .hits
                .entry((ctx.spell.id, target.to_bits()))
                .or_default()
                .mitigated = Some(amount);
        }
    });
}

fn unique_sources(formula: &Formula, live: &[Value]) -> Vec<Value> {
    formula
        .sources
        .iter()
        .chain(live)
        .map(|row| (row.to_string(), row.clone()))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect()
}

fn crit_multiplier(hit: &Hit, crit: bool) -> Option<f32> {
    match hit.critical {
        Some(critical) => ratio(critical.after, critical.before),
        None if !crit => Some(1.0),
        None => None,
    }
}

fn observed_versatility(hit: &Hit, source: &Source) -> f32 {
    if source.skipped_caster_modifiers {
        return 1.0;
    }
    hit.done_components
        .iter()
        .find(|row| row["factor"] == "versatility")
        .and_then(|row| row["value"].as_f64())
        .expect("actual native versatility stage missing") as f32
}

fn formula_factors(formula: &Formula, hit: &Hit, critical: Option<f32>) -> Value {
    let mut factors = formula.factors.clone();
    let target = hit
        .target
        .as_ref()
        .expect("actual native target stage missing");
    let amount = critical.and_then(|critical| ratio(target.unrounded, critical));
    let scaled = formula
        .factors
        .get("native_scaled_amount")
        .and_then(Value::as_f64);
    let multiplier = amount
        .zip(scaled)
        .and_then(|(amount, scaled)| ratio(amount, scaled as f32));
    factors.insert("amount".into(), json!(amount));
    factors.insert("initial_amount".into(), json!(amount));
    factors.insert("mult".into(), json!(multiplier));
    json!(factors)
}

fn factor_sources(sources: &[Value]) -> Value {
    let matching = |operations: &[u64]| {
        sources
            .iter()
            .filter(|source| {
                source["op"]
                    .as_u64()
                    .is_some_and(|op| operations.contains(&op))
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    json!({"a_mod": matching(&[24]), "s_mod": matching(&[24]),
        "base": matching(&[8, 3, 12, 23, 32, 33]), "crit": matching(&[7]),
        "crit_bonus": matching(&[15]), "source_product": matching(&[0, 22])})
}

fn result_factors(hit: &Hit, event: &shared::protocol::CombatLogEvent) -> Value {
    let source = hit
        .source
        .as_ref()
        .expect("actual native source stage missing");
    let target = hit
        .target
        .as_ref()
        .expect("actual native target stage missing");
    let versatility = observed_versatility(hit, source);
    let critical = crit_multiplier(hit, event.crit);
    let source_product = ratio(source.after, source.input)
        .and_then(|value| ratio(value * target.cast_multiplier, versatility));
    json!({"result": if event.crit { "crit" } else { "hit" },
        "crit": hit.chance.map(|value| value.0 / 100.0).or(hit.critical.map(|value| value.chance / 100.0)),
        "tgt_crit": hit.chance.map(|value| value.1 / 100.0),
        "crit_bonus": critical.map(|value| value - 1.0), "versatility": versatility,
        "raw_amount": critical.and_then(|critical| ratio(target.unrounded, critical)),
        "total_amount": target.unrounded, "native_rounded_amount": target.rounded,
        "mitigated_amount": hit.mitigated, "actual_amount": event.amount,
        "absorbed_amount": event.absorbed, "blocked_amount": event.blocked,
        "source_product": source_product, "target_product": target.multiplier,
        "target_armor": hit.armor.map(|armor| armor.value),
        "native_script_product": ratio(source.scripted, source.input),
        "native_spell_mod_product": ratio(source.modded, source.scripted),
        "native_done_product": source.done})
}

fn applied_multipliers(hit: &Hit, crit: bool) -> Vec<Value> {
    let source = hit
        .source
        .as_ref()
        .expect("actual native source stage missing");
    let target = hit
        .target
        .as_ref()
        .expect("actual native target stage missing");
    let mut rows = hit.done_components.clone();
    rows.extend(
        [
            ("script_amount", ratio(source.scripted, source.input)),
            ("spell_mod_amount", ratio(source.modded, source.scripted)),
            ("cast_effectiveness", Some(target.cast_multiplier)),
            ("target_damage", Some(target.multiplier)),
            ("critical_damage", crit_multiplier(hit, crit)),
            (
                "armor_remaining",
                hit.armor.and_then(|armor| ratio(armor.after, armor.before)),
            ),
        ]
        .into_iter()
        .map(|(name, value)| {
            json!({"factor": name, "value": value,
        "attribution": "unattributed product", "sources": []})
        }),
    );
    rows
}

fn event_row(
    capture: &FactorCapture,
    ctx: &EffectCtx,
    kind: &str,
    factors: Value,
    time: f64,
) -> Value {
    let action = if ctx.spell.id == capture.focus_spell {
        capture.action.clone()
    } else {
        format!("spell_{}", ctx.spell.id)
    };
    json!({"event": kind, "time": time, "actor": ctx.caster.to_bits().to_string(),
        "target": capture.target.to_bits().to_string(), "action": action,
        "spell_id": ctx.spell.id, "native_spell_name": ctx.spell.name, "native_depth": ctx.depth,
        "factors": factors, "factor_attribution": {"mult": "unattributed product",
            "source_product": "unattributed product", "target_product": "unattributed product",
            "amount": "crit-normalized observed amount", "raw_amount": "crit-normalized observed amount"}})
}

pub(super) fn finish(
    world: &World,
    ctx: &EffectCtx,
    target: Entity,
    damage: &DamageInput,
    event: &shared::protocol::CombatLogEvent,
) {
    observe(world, ctx.caster, |capture, state| {
        if target != capture.target {
            return;
        }
        let time = crate::spell_cast::now(world);
        let hit = state
            .hits
            .remove(&(ctx.spell.id, target.to_bits()))
            .expect("native factor impact stages absent");
        let formula = damage
            .effect
            .and_then(|effect| state.formulas.get(&(ctx.spell.id, effect.index)))
            .cloned()
            .unwrap_or_default();
        let sources = unique_sources(
            &formula,
            &state.modifiers.remove(&ctx.spell.id).unwrap_or_default(),
        );
        let kind = if event.periodic {
            "periodic_amount"
        } else {
            "direct_amount"
        };
        let mut amount = event_row(
            capture,
            ctx,
            kind,
            formula_factors(&formula, &hit, crit_multiplier(&hit, event.crit)),
            time,
        );
        amount["factor_sources"] = factor_sources(&sources);
        amount["applied_modifiers"] = json!(&sources);
        amount["applied_multipliers"] = json!(applied_multipliers(&hit, event.crit));
        state.events.push(amount);
        if let Some(armor) = hit.armor {
            let factors = json!({"armor_fraction": ratio(armor.after, armor.before).map(|value| 1.0 - value), "armor": armor.value});
            state
                .events
                .push(event_row(capture, ctx, "armor", factors, time));
        }
        let mut result = event_row(capture, ctx, "result", result_factors(&hit, event), time);
        result["factor_sources"] = factor_sources(&sources);
        state.events.push(result);
        if ctx.spell.id == capture.focus_spell {
            state.completed += 1;
        }
    });
}

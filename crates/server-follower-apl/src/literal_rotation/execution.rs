//! Native eligibility is sampled before immutable expression evaluation.
use super::equipment::{Equipment, ItemUse};
use bevy::prelude::*;
use server::simc_apl::Action;
pub use server::spell_cast::knowledge::resolve_action;
use shared::components::PowerType;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub enum Execution {
    Snapshot,
    Wait(f64),
    Queue(Box<Execution>, f64),
    CancelCast,
    PickupFragment(server::soul_fragments::PickupPlan),
    AutoAttack(Entity),
    RetargetAutoAttack(Entity),
    Retarget(Entity),
    // Spell, native cast unit, and selected action primary (not inferred from that unit).
    Spell(u32, Entity, Option<Entity>),
    Item(ItemUse, Entity),
    Composite(Vec<Execution>),
}

#[derive(Clone, Copy)]
pub struct NativeAction<'a> {
    pub name: &'a str,
    pub spell: u32,
    pub talent: Option<u32>,
    pub self_target: bool,
}

pub struct NativeComposite<'a> {
    pub name: &'a str,
    pub steps: &'a [NativeAction<'a>],
}

// Keys identify selection contexts; Execution retains the actual native cast target.
// None is the native default, never a substitute for an unknown explicit context.
type ActionKey = (String, Option<Entity>);
type ReadyActions = BTreeMap<ActionKey, Execution>;

#[derive(Clone, Copy)]
struct NativeReadiness {
    ready: bool,
    limited: Option<PowerType>,
    queue_delay: Option<f64>,
}

pub struct Prepared {
    now: f64,
    ready: ReadyActions,
    limits: BTreeMap<ActionKey, PowerType>,
    pickups: BTreeMap<
        (
            server::soul_fragments::FragmentFilter,
            server::soul_fragments::PickupMode,
        ),
        server::soul_fragments::PickupPlan,
    >,
    declared: BTreeSet<String>,
    moving: bool,
    cast_active: bool,
    pickup_cast_usable: bool,
    channel_cast_actions: BTreeSet<String>,
    gcd_active: bool,
    spec_id: u32,
    after_channel: bool,
}

// SimC76f8118 class constructors set use_off_gcd independently of APL options.
// Native readiness still requires the spell's actual GCD, costs and cooldowns.
fn intrinsic_off_gcd(spec: u32, name: &str) -> bool {
    match spec {
        71..=73 => matches!(
            name,
            "ignore_pain"
                | "berserker_rage"
                | "battle_stance"
                | "berserker_stance"
                | "defensive_stance"
                | "spell_reflection"
                | "taunt"
        ),
        1467 | 1468 | 1473 => name == "quell",
        577 | 581 | 1480 => matches!(name, "demon_spikes" | "fiery_brand"),
        250..=252 => name == "dark_command",
        102..=105 => matches!(
            name,
            "skull_bash"
                | "solar_beam"
                | "cancel_form"
                | "growl"
                | "ironfur"
                | "barkskin"
                | "prowl"
                | "survival_instincts"
                | "auto_attack"
        ),
        258 => name == "void_torrent",
        268..=270 => matches!(name, "black_ox_brew" | "purifying_brew"),
        65 => matches!(
            name,
            "divine_protection" | "lay_on_hands" | "hand_of_reckoning" | "shield_of_the_righteous"
        ),
        66 => matches!(
            name,
            "ardent_defender"
                | "guardian_of_ancient_kings"
                | "lay_on_hands"
                | "hand_of_reckoning"
                | "shield_of_the_righteous"
        ),
        70 => matches!(
            name,
            "lay_on_hands" | "hand_of_reckoning" | "shield_of_the_righteous"
        ),
        _ => false,
    }
}

impl Prepared {
    pub fn read<'a>(
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        equipment: &Equipment,
        actions: impl IntoIterator<Item = NativeAction<'a>>,
        needs_snapshot: bool,
    ) -> Result<Self, String> {
        Self::read_with_policy(
            world,
            actor,
            targets,
            equipment,
            actions,
            needs_snapshot,
            false,
        )
    }

    pub fn read_after_channel<'a>(
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        equipment: &Equipment,
        actions: impl IntoIterator<Item = NativeAction<'a>>,
    ) -> Result<Self, String> {
        Self::read_with_policy(world, actor, targets, equipment, actions, false, true)
    }

    fn read_with_policy<'a>(
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        equipment: &Equipment,
        actions: impl IntoIterator<Item = NativeAction<'a>>,
        needs_snapshot: bool,
        after_channel: bool,
    ) -> Result<Self, String> {
        let moving = world
            .get::<crate::networking::Moving>(actor)
            .is_some_and(|moving| moving.0);
        let active_cast = world.get::<shared::casting::CastState>(actor);
        let cast_active = active_cast.is_some();
        let pickup_cast_usable = active_cast
            .is_some_and(|cast| matches!(cast.spell_id, 473662 | 1217610))
            && world
                .get::<shared::components::ActiveSpec>(actor)
                .is_some_and(|spec| spec.0 == 1480);
        let gcd_active = world
            .get::<crate::spell_cast::SpellHistory>(actor)
            .is_some_and(|history| history.has_active_gcd(crate::spell_cast::history_now(world)));
        let targets = native_targets(world, actor, targets);
        let mut prepared = Self {
            now: crate::spell_cast::history_now(world),
            moving,
            cast_active: cast_active && !after_channel,
            pickup_cast_usable,
            channel_cast_actions: BTreeSet::new(),
            gcd_active: gcd_active && !after_channel,
            spec_id: world
                .get::<shared::components::ActiveSpec>(actor)
                .map_or(0, |spec| spec.0),
            after_channel,
            ready: BTreeMap::new(),
            limits: BTreeMap::new(),
            pickups: fragment_pickup_plans(world, actor, targets.first().copied())?,
            declared: [
                "snapshot_stats",
                "auto_attack",
                "retarget_auto_attack",
                "pick_up_fragment",
                "retarget",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        };
        if cast_active {
            prepared
                .ready
                .insert(("cancel_action".into(), None), Execution::CancelCast);
        }
        if needs_snapshot {
            prepared
                .ready
                .insert(("snapshot_stats".into(), None), Execution::Snapshot);
        }
        read_attacks(world, actor, &targets, &mut prepared.ready);
        // SimC76f8118 sc_hunter.cpp7909 binds native Auto Shot75.
        // Knowledge/native admission still decide whether the actor can start it.
        let repeated = NativeAction {
            name: "auto_shot",
            spell: 75,
            talent: None,
            self_target: false,
        };
        read_spells(
            world,
            actor,
            &targets,
            actions.into_iter().chain([repeated]),
            &mut prepared,
        )?;
        if let Some(potion) = world.get::<super::equipment::Potion>(actor).copied() {
            let name = "potion".to_owned();
            prepared.declared.insert(name.clone());
            let state = native_readiness(
                world,
                actor,
                potion.0.on_use.spell_id,
                actor,
                Some(potion.0),
                after_channel,
            )?;
            record_probe(
                &mut prepared,
                (name, None),
                false,
                state,
                Execution::Item(potion.0, actor),
            );
        }
        for item in equipment.uses() {
            read_item(world, actor, &targets, item, &mut prepared)?;
        }
        Ok(prepared)
    }

    /// Suppress every target context for a fixture-specific action policy.
    pub fn suppress_action(&mut self, name: &str) {
        self.ready.retain(|(action, _), _| action != name);
    }

    pub fn now(&self) -> f64 {
        self.now
    }

    pub fn setup_pending(&self) -> bool {
        self.cast_active || self.gcd_active
    }

    pub fn permits_row(&self, action: &Action) -> Result<bool, String> {
        if action.name == "cancel_action" {
            return Ok(true);
        }
        if read_boolean_option(action, "moving")?.is_some_and(|moving| moving != self.moving) {
            return Ok(false);
        }
        let while_casting = read_boolean_option(action, "use_while_casting")?.unwrap_or(false);
        let pickup = action.name == "pick_up_fragment";
        if self.cast_active && pickup && !self.pickup_cast_usable {
            return Ok(false);
        }
        if self.cast_active
            && !while_casting
            && !(pickup && self.pickup_cast_usable)
            && !self.channel_cast_actions.contains(&action.name)
        {
            return Ok(false);
        }
        let off_gcd = read_boolean_option(action, "use_off_gcd")?.unwrap_or(false)
            || intrinsic_off_gcd(self.spec_id, &action.name);
        Ok(!self.gcd_active || off_gcd)
    }

    /// Plans only; the precombat executor owns clock advancement and each native cast.
    pub fn with_composites<'a>(
        mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        composites: impl IntoIterator<Item = NativeComposite<'a>>,
    ) -> Result<Self, String> {
        let enemies = native_targets(world, actor, targets);
        for composite in composites {
            self.declared.insert(composite.name.into());
            let self_only = composite.steps.iter().all(|step| step.self_target);
            let candidates: &[Entity] = if self_only {
                std::slice::from_ref(&actor)
            } else {
                enemies.as_slice()
            };
            for (index, &target) in candidates.iter().enumerate() {
                if let Some(plan) = read_composite_plan(world, actor, target, composite.steps)? {
                    record_ready(&mut self.ready, composite.name, target, index == 0, plan);
                }
            }
        }
        Ok(self)
    }

    /// True only when this selected pool at its native maximum would admit the
    /// action. Other pools/health and every other native check remain unchanged.
    pub fn resource_limited(
        &self,
        action: &Action,
        equipment: &Equipment,
        resource: PowerType,
    ) -> Result<bool, String> {
        let Some(key) = action_key(action, equipment)? else {
            return Ok(false);
        };
        if !self.declared.contains(&key.0) {
            return Err(format!(
                "unknown native resource-query action {}",
                action.name
            ));
        }
        Ok(self
            .limits
            .get(&key)
            .is_some_and(|power| *power == resource))
    }

    pub fn execution_in(
        &self,
        action: &Action,
        equipment: &Equipment,
        program: &server::simc_apl::Program,
    ) -> Result<Option<Execution>, String> {
        if action.name != "use_items" {
            return self.execution(action, equipment);
        }
        for slot in equipment.use_items_slots(action, program)? {
            let singular = Action {
                source_row: None,
                name: "use_item".into(),
                target: action.target,
                options: [("slot".into(), slot.into())].into(),
            };
            if let Some(execution) = self.execution(&singular, equipment)? {
                return Ok(Some(execution));
            }
        }
        Ok(None)
    }

    pub fn execution(
        &self,
        action: &Action,
        equipment: &Equipment,
    ) -> Result<Option<Execution>, String> {
        if action.name == "pick_up_fragment" {
            let (filter, mode) = fragment_pickup_options(action)?;
            return Ok(self
                .pickups
                .get(&(filter, mode))
                .copied()
                .map(Execution::PickupFragment));
        }
        let Some(key) = action_key(action, equipment)? else {
            return Ok(None);
        };
        let execution = self.ready.get(&key).cloned();
        let off_gcd = read_boolean_option(action, "use_off_gcd")?.unwrap_or(false)
            || intrinsic_off_gcd(self.spec_id, &action.name);
        Ok(execution.filter(|execution| !off_gcd || !matches!(execution, Execution::Queue(_, _))))
    }
}

fn fragment_pickup_plans(
    world: &World,
    actor: Entity,
    target: Option<Entity>,
) -> Result<
    BTreeMap<
        (
            server::soul_fragments::FragmentFilter,
            server::soul_fragments::PickupMode,
        ),
        server::soul_fragments::PickupPlan,
    >,
    String,
> {
    use server::soul_fragments::{FragmentFilter as F, PickupMode as M};
    let mut plans = BTreeMap::new();
    if server::soul_fragments::active_count(world, actor, F::Any) == 0 {
        return Ok(plans);
    }
    let Some(target) = target else {
        return Ok(plans);
    };
    if !server::soul_fragments::pickup_in_melee_range(world, actor, target)? {
        return Ok(plans);
    }
    for filter in [F::Any, F::Lesser, F::AnyGreater, F::AnyDemon] {
        for mode in [M::Oldest, M::Newest, M::Nearest] {
            if let Some(plan) = server::soul_fragments::plan_pickup(world, actor, filter, mode) {
                plans.insert((filter, mode), plan);
            }
        }
    }
    Ok(plans)
}

fn fragment_pickup_options(
    action: &Action,
) -> Result<
    (
        server::soul_fragments::FragmentFilter,
        server::soul_fragments::PickupMode,
    ),
    String,
> {
    use server::soul_fragments::{FragmentFilter as F, PickupMode as M};
    let filter = match action
        .options
        .get("type")
        .map(String::as_str)
        .unwrap_or("any")
    {
        "all" | "any" => F::Any,
        "lesser" => F::Lesser,
        "greater" => F::AnyGreater,
        "demon" => F::AnyDemon,
        value => return Err(format!("unsupported fragment type {value}")),
    };
    let mode = match action
        .options
        .get("mode")
        .map(String::as_str)
        .unwrap_or("oldest")
    {
        "old" | "oldest" => M::Oldest,
        "new" | "newest" => M::Newest,
        "close" | "near" | "closest" | "nearest" => M::Nearest,
        value => return Err(format!("unsupported fragment pickup mode {value}")),
    };
    Ok((filter, mode))
}

fn read_composite_plan(
    world: &mut World,
    actor: Entity,
    candidate: Entity,
    steps: &[NativeAction<'_>],
) -> Result<Option<Execution>, String> {
    let mut pending = Vec::new();
    for step in steps {
        let target = if step.self_target { actor } else { candidate };
        if step.self_target && owns_active_aura(world, actor, step.spell) {
            continue;
        }
        if !talent_permits(world, actor, step.talent)? {
            return Ok(None);
        }
        let Some(spell) = resolve_action(world, actor, step.spell)? else {
            return Ok(None);
        };
        if !native_readiness(world, actor, spell, target, None, false)?.ready {
            return Ok(None);
        }
        pending.push(Execution::Spell(
            spell,
            target,
            (candidate != actor).then_some(candidate),
        ));
    }
    Ok((!pending.is_empty()).then_some(Execution::Composite(pending)))
}

fn owns_active_aura(world: &World, actor: Entity, spell: u32) -> bool {
    let now = crate::spell_cast::now(world);
    world
        .get::<crate::auras::ActiveAuras>(actor)
        .is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.spell_id == spell
                    && aura.caster == actor
                    && aura.expires_at.is_none_or(|at| at > now)
            })
        })
}

fn read_boolean_option(action: &Action, name: &str) -> Result<Option<bool>, String> {
    match action.options.get(name).map(String::as_str) {
        None => Ok(None),
        Some("0") => Ok(Some(false)),
        Some("1") => Ok(Some(true)),
        Some(value) => Err(format!(
            "unsupported native literal option {}:{name}={value}; expected0|1",
            action.name
        )),
    }
}

fn action_key(action: &Action, equipment: &Equipment) -> Result<Option<ActionKey>, String> {
    let name = if action.name == "use_item" {
        let Some(slot) = equipment.slot_for_action(action)? else {
            return Ok(None);
        };
        format!("item:{slot}")
    } else {
        action.name.clone()
    };
    let target = action
        .target
        .map(|bits| {
            Entity::try_from_bits(bits)
                .ok_or_else(|| format!("invalid explicit literal target {bits}"))
        })
        .transpose()?;
    Ok(Some((name, target)))
}

fn native_targets(world: &World, actor: Entity, supplied: &[Entity]) -> Vec<Entity> {
    if !supplied.is_empty() {
        let mut targets = supplied.to_vec();
        if let Some(index) = world
            .get::<crate::networking::CombatTarget>(actor)
            .and_then(|target| target.0)
            .and_then(|selected| targets.iter().position(|&target| target == selected))
        {
            targets.swap(0, index);
        }
        return targets;
    }
    // A harmful precast still uses the actor's actual selected enemy, even
    // before the adapter supplies its combat candidate order.
    world
        .get::<crate::networking::CombatTarget>(actor)
        .and_then(|target| target.0)
        .into_iter()
        .collect()
}

fn record_ready(
    ready: &mut ReadyActions,
    name: &str,
    target: Entity,
    default: bool,
    execution: Execution,
) {
    ready.insert((name.into(), Some(target)), execution.clone());
    if default {
        ready.insert((name.into(), None), execution);
    }
}

fn record_target_contexts(
    world: &World,
    actor: Entity,
    prepared: &mut Prepared,
    name: &str,
    contexts: &[Entity],
    state: NativeReadiness,
    spell: &crate::spell_info::SpellInfo,
) {
    for &candidate in contexts {
        let state = filter_primary_range(world, actor, spell, Some(candidate), state);
        record_probe(
            prepared,
            (name.into(), Some(candidate)),
            false,
            state,
            Execution::Spell(spell.id, actor, Some(candidate)),
        );
    }
}

fn filter_primary_range(
    world: &World,
    actor: Entity,
    spell: &crate::spell_info::SpellInfo,
    primary: Option<Entity>,
    state: NativeReadiness,
) -> NativeReadiness {
    if !state.ready && state.limited.is_none() {
        return state;
    }
    // Native enemy selection includes attackable neutral units, not only hostile factions.
    let Some(primary) = primary.filter(|&primary| {
        crate::spell_cast::targets::relation_matches(
            world,
            actor,
            primary,
            crate::spell_cast::targets::Relation::Enemy,
        )
    }) else {
        return state;
    };
    // SimC76f8118 player.cpp9045–9057: Arcane Torrent targets the actor and
    // is non-harmful despite the native area dispel effect.
    if spell.id == 202719 {
        return state;
    }
    let harmful = crate::spell_info::positivity::negative_effects(
        world.resource::<crate::spell_info::SpellStore>(),
        spell,
    ) != 0;
    let returning = super::melee_intent::returning_after_displacement(world, actor);
    let channel = crate::spell_cast::is_channeled_for_actor(world, actor, spell);
    if !harmful && !(returning && channel) {
        return state;
    }
    let dh = world
        .get::<shared::components::ActiveSpec>(actor)
        .is_some_and(|spec| matches!(spec.0, 577 | 581 | 1480));
    let outside_melee = crate::spell_cast::targets::position(world, actor)
        .zip(crate::spell_cast::targets::position(world, primary))
        .zip(crate::scripted_ai::model_reach::melee_range(
            world, actor, primary,
        ))
        .is_some_and(|((from, to), range)| from.distance(to) > range);
    // SimC76f8118 sc_demon_hunter.cpp2477–2486: a DH channel is unavailable
    // while the actor is out of melee, even if its own spell range is longer.
    let unstable_channel = returning
        && channel
        && (super::melee_intent::forced_movement(world, actor) || (dh && outside_melee));
    if !unstable_channel && primary_in_native_range(world, actor, spell, primary) {
        return state;
    }
    NativeReadiness {
        ready: false,
        limited: None,
        queue_delay: None,
    }
}

fn primary_in_native_range(
    world: &World,
    actor: Entity,
    spell: &crate::spell_info::SpellInfo,
    primary: Entity,
) -> bool {
    let (Some(from), Some(to)) = (
        crate::spell_cast::targets::position(world, actor),
        crate::spell_cast::targets::position(world, primary),
    ) else {
        return false;
    };
    // SimC76f8118 action.cpp2418–2430 checks the selected hostile target,
    // even when a caster-centred native spell keeps the actor as its cast unit.
    // Reuse native range and live Range modifiers; do not move either unit.
    let (min, mut max) = crate::spell_cast::targets::spell_range(spell, true);
    if crate::spell_cast::targets::uses_melee_range(spell, true) {
        let Some(range) = crate::scripted_ai::model_reach::melee_range(world, actor, primary)
        else {
            error!(
                ?actor,
                ?primary,
                "Literal melee range requires sourced combat reaches"
            );
            return false;
        };
        max = range;
    }
    let max =
        crate::auras::spell_mods::modify(world, actor, spell, crate::auras::SpellModOp::Range, max);
    let distance = from.distance(to);
    distance >= min && distance <= max
}

fn read_attacks(world: &World, actor: Entity, targets: &[Entity], ready: &mut ReadyActions) {
    let current = world.get::<crate::combat::AutoAttack>(actor);
    for (index, &target) in targets.iter().enumerate() {
        // SimC76f8118 player.cpp10866–10889: retarget changes the actor's selected target.
        record_ready(
            ready,
            "retarget",
            target,
            index == 0,
            Execution::Retarget(target),
        );
        if current.is_none() {
            record_ready(
                ready,
                "auto_attack",
                target,
                index == 0,
                Execution::AutoAttack(target),
            );
        }
        // SimC76f8118 player.cpp10824–10860: only a running, different
        // weapon victim can be retargeted; the actor's selection stays intact.
        if current.is_some_and(|attack| attack.victim != target) {
            record_ready(
                ready,
                "retarget_auto_attack",
                target,
                index == 0,
                Execution::RetargetAutoAttack(target),
            );
        }
    }
}

fn record_probe(
    prepared: &mut Prepared,
    key: ActionKey,
    default: bool,
    state: NativeReadiness,
    execution: Execution,
) {
    if state.ready {
        let execution = match state.queue_delay {
            Some(delay) => Execution::Queue(Box::new(execution), delay),
            None => execution,
        };
        prepared.ready.insert(key.clone(), execution.clone());
        if default {
            prepared.ready.insert((key.0.clone(), None), execution);
        }
    }
    if let Some(power) = state.limited {
        prepared.limits.insert(key.clone(), power);
        if default {
            prepared.limits.insert((key.0, None), power);
        }
    }
}

fn read_spells<'a>(
    world: &mut World,
    actor: Entity,
    enemies: &[Entity],
    actions: impl IntoIterator<Item = NativeAction<'a>>,
    prepared: &mut Prepared,
) -> Result<(), String> {
    for action in actions {
        read_spell(world, actor, enemies, action, prepared)?;
    }
    Ok(())
}

fn read_spell(
    world: &mut World,
    actor: Entity,
    enemies: &[Entity],
    action: NativeAction<'_>,
    prepared: &mut Prepared,
) -> Result<(), String> {
    let NativeAction {
        name,
        spell: id,
        talent,
        self_target,
    } = action;
    prepared.declared.insert(name.into());
    if !talent_permits(world, actor, talent)? {
        return Ok(());
    }
    let Some(id) = resolve_action(world, actor, id)? else {
        return Ok(());
    };
    let resolved = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .expect("resolved native action remains in its catalog");
    if crate::spell_cast::script_registry::allows_cast_during_channel(world, actor, resolved) {
        prepared.channel_cast_actions.insert(name.into());
    }
    let self_target = self_target || crate::spell_cast::targets::targets_caster_only(resolved);
    // SimC76f8118 sc_hunter.cpp7269–7274: start once; native repeats
    // own later shots. This applies to any native autorepeat, not a spell-ID branch.
    if crate::spell_cast::native_apl::auto_repeat_active(world, actor, id) {
        return Ok(());
    }
    let targets = if self_target {
        std::slice::from_ref(&actor)
    } else {
        enemies
    };
    for (index, &target) in targets.iter().enumerate() {
        if interrupt_requires_casting_target(name)
            && world.get::<shared::casting::CastState>(target).is_none()
        {
            continue;
        }
        let native = native_readiness(world, actor, id, target, None, prepared.after_channel)?;
        let resolved = world
            .resource::<crate::spell_info::SpellStore>()
            .get(id)
            .expect("resolved native action remains in its catalog");
        let primary = if self_target {
            enemies.first().copied()
        } else {
            Some(target)
        };
        let state = filter_primary_range(world, actor, resolved, primary, native);
        record_probe(
            prepared,
            (name.into(), Some(target)),
            !self_target && index == 0,
            state,
            Execution::Spell(id, target, (!self_target).then_some(target)),
        );
        if self_target {
            // The default context is the same first native candidate used by
            // ordinary actions. An explicit context retains its own identity.
            record_probe(
                prepared,
                (name.into(), None),
                false,
                state,
                Execution::Spell(id, target, enemies.first().copied()),
            );
            record_target_contexts(world, actor, prepared, name, enemies, native, resolved);
        }
    }
    Ok(())
}

// SimC76f8118 checks the target's casting state in each class's interrupt
// target_ready override (sc_rogue.cpp4682–4687, sc_warrior.cpp4903–4909,
// sc_mage.cpp3672–3678, sc_druid.cpp3573–3578). Skull Bash explicitly
// bypasses that check for fluid-form shifting (sc_druid.cpp8174–8181).
fn interrupt_requires_casting_target(name: &str) -> bool {
    matches!(
        name,
        "kick"
            | "rebuke"
            | "pummel"
            | "mind_freeze"
            | "counterspell"
            | "wind_shear"
            | "spear_hand_strike"
            | "disrupt"
            | "muzzle"
            | "counter_shot"
            | "silence"
            | "quell"
            | "solar_beam"
            | "interrupt"
    )
}

fn read_item(
    world: &mut World,
    actor: Entity,
    targets: &[Entity],
    item: (&str, ItemUse),
    prepared: &mut Prepared,
) -> Result<(), String> {
    let (slot, item) = item;
    let name = format!("item:{slot}");
    prepared.declared.insert(name.clone());
    // SimC76f8118 player.cpp10056–10068 forwards the selected primary to
    // the on-use action. Native ally/self payloads still cast on their owner.
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(item.on_use.spell_id)
        .ok_or_else(|| format!("native literal item spell{} absent", item.on_use.spell_id))?;
    let requirement = crate::spell_cast::targets::explicit_requirement(spell.effects.iter());
    let default_target = match requirement {
        Some(
            crate::spell_cast::targets::Relation::Enemy | crate::spell_cast::targets::Relation::Any,
        ) => targets.first().copied(),
        _ => Some(actor),
    };
    for target in std::iter::once(actor).chain(targets.iter().copied()) {
        let state = native_readiness(
            world,
            actor,
            item.on_use.spell_id,
            target,
            Some(item),
            prepared.after_channel,
        )?;
        record_probe(
            prepared,
            (name.clone(), Some(target)),
            Some(target) == default_target,
            state,
            Execution::Item(item, target),
        );
    }
    Ok(())
}

fn talent_permits(world: &World, actor: Entity, talent: Option<u32>) -> Result<bool, String> {
    let Some(talent) = talent else {
        return Ok(true);
    };
    let traits = world
        .get::<crate::class_progression::TraitLoadout>(actor)
        .ok_or("native selected TraitLoadout absent")?;
    Ok(traits
        .active_talent_spells()
        .any(|spell| spell.spell_id == talent))
}

fn native_readiness(
    world: &mut World,
    actor: Entity,
    id: u32,
    target: Entity,
    item: Option<ItemUse>,
    after_channel: bool,
) -> Result<NativeReadiness, String> {
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .cloned()
        .ok_or_else(|| format!("native literal spell{id} absent"))?;
    spell.cast_destination = action_destination(world, &spell, target)?;
    if let Some(item) = item {
        spell.cast_item_level = Some(item.item_level);
        spell.cast_retail_item_id = Some(item.item_id);
    }
    let admission = if after_channel {
        crate::spell_cast::read_native_cast_after_channel(world, actor, &spell, target)
    } else {
        crate::spell_cast::read_native_cast_start(world, actor, &spell, target)
    };
    match admission {
        Ok(_) => Ok(NativeReadiness {
            ready: true,
            limited: None,
            queue_delay: None,
        }),
        Err(
            crate::spell_cast::CastFailure::OnCooldown | crate::spell_cast::CastFailure::NoCharges,
        ) if item.is_none() => {
            match crate::spell_cast::read_native_cast_for_cooldown_queue(
                world,
                actor,
                &spell,
                target,
                after_channel,
            ) {
                Ok(delay) => Ok(NativeReadiness {
                    ready: true,
                    limited: None,
                    queue_delay: Some(delay),
                }),
                Err(reason) => refused_admission(id, reason),
            }
        }
        Err(crate::spell_cast::CastFailure::NotEnoughPower(_)) if after_channel => {
            Ok(NativeReadiness {
                ready: false,
                limited: None,
                queue_delay: None,
            })
        }
        Err(crate::spell_cast::CastFailure::NotEnoughPower(power)) => {
            match crate::spell_cast::read_native_cast_at_resource_max(
                world, actor, &spell, target, power,
            ) {
                Ok(_) => Ok(NativeReadiness {
                    ready: false,
                    limited: Some(power),
                    queue_delay: None,
                }),
                Err(reason) => refused_admission(id, reason),
            }
        }
        Err(reason) => refused_admission(id, reason),
    }
}

/// TC InitExplicitTargets (Spell.cpp666–678): a cursor action supplies HasDst.
/// The literal selected unit provides that point; native range/LOS still admit it.
fn action_destination(
    world: &World,
    spell: &crate::spell_info::SpellInfo,
    target: Entity,
) -> Result<Option<[f32; 3]>, String> {
    let cursor = spell.effects.iter().any(|effect| {
        effect
            .implicit_targets
            .iter()
            .any(|id| matches!(id, 87 | 138))
    });
    if !cursor {
        return Ok(None);
    }
    crate::spell_cast::targets::position(world, target)
        .map(|point| Some(point.to_array()))
        .ok_or_else(|| format!("native literal cursor target {target:?} lacks a position"))
}

fn refused_admission(
    id: u32,
    reason: crate::spell_cast::CastFailure,
) -> Result<NativeReadiness, String> {
    match reason {
        reason @ (crate::spell_cast::CastFailure::UnknownSpell
        | crate::spell_cast::CastFailure::Unsupported(_)) => Err(format!(
            "NATIVE_LITERAL_ADMISSION_GAP: spell{id}: {reason:?}"
        )),
        _ => Ok(NativeReadiness {
            ready: false,
            limited: None,
            queue_delay: None,
        }),
    }
}

pub fn execute_row(
    world: &mut World,
    actor: Entity,
    row: &Action,
    execution: Execution,
) -> Result<(), String> {
    if let Some(age) = row.options.get("pre_pot_time") {
        if row.name != "potion" {
            return Err("pre_pot_time requires a native potion row".into());
        }
        return super::precombat_potion::execute(world, actor, execution, age);
    }
    let Some(stage) = row.options.get("empower_to") else {
        return execute(world, actor, execution);
    };
    // SimC76f8118 sc_evoker.cpp3595–3599 parses empower_to;
    // selecting a row must retain its requested stage, not the native maximum.
    let stage = stage
        .parse::<u8>()
        .map_err(|_| format!("invalid empower_to={stage}"))?;
    let Execution::Spell(spell, unit, primary) = execution else {
        return Err("empower_to requires an ordinary native spell".into());
    };
    let metadata = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .ok_or_else(|| format!("native empower spell{spell} absent"))?;
    let destination = action_destination(world, metadata, unit)?;
    crate::spell_cast::creature_cast_empowered_with_targets(
        world,
        actor,
        spell,
        crate::spell_cast::CastTargets {
            unit,
            primary,
            destination,
        },
        stage,
    )
    .map_err(|error| format!("native empower{spell} stage{stage} rejected: {error:?}"))
}

pub fn execute(world: &mut World, actor: Entity, action: Execution) -> Result<(), String> {
    match action {
        Execution::Snapshot | Execution::Wait(_) | Execution::Queue(_, _) => {
            Err("snapshot and wait execution belong to the literal driver".into())
        }
        Execution::PickupFragment(plan) => server::soul_fragments::begin_pickup(world, actor, plan),
        Execution::CancelCast => {
            crate::spell_cast::cancel_cast(world, actor);
            Ok(())
        }
        Execution::Composite(_) => {
            Err("composite execution requires an ordered precombat executor".into())
        }
        Execution::AutoAttack(target) => {
            let current = world.get::<crate::combat::AutoAttack>(actor).cloned();
            crate::combat::start_attack(&mut world.commands(), actor, target, current.as_ref());
            world.flush();
            world
                .entity_mut(actor)
                .remove::<crate::creature_ai::FollowOwner>()
                .insert(crate::networking::CombatTarget(Some(target)));
            // Preserve native chase/facing when the literal fixture replaces the
            // kit controller. TC ChaseMovementGenerator::Update SetInFront;
            // without Pursuing, caster-facing cones miss the selected victim.
            if let Some(mut state) = world.get_mut::<crate::creature_ai::CreatureState>(actor) {
                *state = crate::creature_ai::CreatureState::Pursuing {
                    target: target.to_bits(),
                };
            }
            Ok(())
        }
        Execution::Retarget(target) => {
            if world.get_entity(target).is_err() {
                return Err("selected native retarget no longer exists".into());
            }
            world
                .entity_mut(actor)
                .insert(crate::networking::CombatTarget(Some(target)));
            if let Some(mut attack) = world.get_mut::<crate::combat::AutoAttack>(actor) {
                attack.victim = target;
            }
            Ok(())
        }
        Execution::RetargetAutoAttack(target) => {
            if world.get_entity(target).is_err() {
                return Err("selected native AA target no longer exists".into());
            }
            let mut attack = world
                .get_mut::<crate::combat::AutoAttack>(actor)
                .ok_or("native AA stopped after retarget readiness")?;
            attack.victim = target;
            Ok(())
        }
        Execution::Spell(id, target, primary) => {
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(id)
                .ok_or_else(|| format!("native literal spell{id} absent"))?;
            let targets = crate::spell_cast::CastTargets {
                unit: target,
                primary,
                destination: action_destination(world, spell, target)?,
            };
            let result = if world.get::<shared::components::Player>(actor).is_some() {
                crate::spell_cast::cast_for_player_with_targets(world, actor, id, targets)
            } else {
                crate::spell_cast::creature_cast_with_targets(world, actor, id, targets)
            };
            result
                .map_err(|reason| format!("selected native literal spell{id} rejected: {reason:?}"))
        }
        Execution::Item(item, target) => {
            crate::spell_cast::cast_item_spell(
                world,
                actor,
                item.on_use.spell_id,
                Some(target),
                item.on_use.cooldown,
                item.item_level,
                Some(item.item_id),
            )
            .map_err(|reason| {
                format!(
                    "selected native literal item{} rejected: {reason:?}",
                    item.item_id
                )
            })?;
            if world
                .get::<super::equipment::Potion>(actor)
                .is_some_and(|potion| potion.0.item_id == item.item_id)
            {
                world.entity_mut(actor).remove::<super::equipment::Potion>();
            }
            Ok(())
        }
    }
}

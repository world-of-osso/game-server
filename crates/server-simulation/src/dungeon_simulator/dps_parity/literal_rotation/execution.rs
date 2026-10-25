//! Native eligibility is sampled before immutable expression evaluation.
use super::equipment::{Equipment, ItemUse};
use bevy::prelude::*;
use server::simc_apl::Action;
pub(crate) use server::spell_cast::knowledge::resolve_action;
use shared::components::PowerType;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub(crate) enum Execution {
    Snapshot,
    CancelCast,
    AutoAttack(Entity),
    RetargetAutoAttack(Entity),
    // Spell, native cast unit, and selected action primary (not inferred from that unit).
    Spell(u32, Entity, Option<Entity>),
    Item(ItemUse, Entity),
    Composite(Vec<Execution>),
}

#[derive(Clone, Copy)]
pub(crate) struct NativeAction<'a> {
    pub name: &'a str,
    pub spell: u32,
    pub talent: Option<u32>,
    pub self_target: bool,
}

pub(crate) struct NativeComposite<'a> {
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
}

pub(crate) struct Prepared {
    ready: ReadyActions,
    limits: BTreeMap<ActionKey, PowerType>,
    declared: BTreeSet<String>,
    moving: bool,
    cast_active: bool,
    gcd_active: bool,
    after_channel: bool,
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
        let cast_active = world.get::<shared::casting::CastState>(actor).is_some();
        let gcd_active = world
            .get::<crate::spell_cast::SpellHistory>(actor)
            .is_some_and(|history| history.has_active_gcd(crate::spell_cast::history_now(world)));
        let targets = native_targets(world, actor, targets);
        let mut prepared = Self {
            moving,
            cast_active: cast_active && !after_channel,
            gcd_active: gcd_active && !after_channel,
            after_channel,
            ready: BTreeMap::new(),
            limits: BTreeMap::new(),
            declared: ["snapshot_stats", "auto_attack", "retarget_auto_attack"]
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
        if let Some(potion) = world
            .get::<super::super::profile_effects::Potion>(actor)
            .copied()
        {
            let name = "potion".to_owned();
            prepared.declared.insert(name.clone());
            let state = native_readiness(
                world,
                actor,
                potion.0.on_use.spell_id,
                actor,
                Some(potion.0),
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
        if self.cast_active && !while_casting {
            return Ok(false);
        }
        let off_gcd = read_boolean_option(action, "use_off_gcd")?.unwrap_or(false);
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
        let Some(key) = action_key(action, equipment)? else {
            return Ok(None);
        };
        Ok(self.ready.get(&key).cloned())
    }
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
        return supplied.to_vec();
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
    prepared: &mut Prepared,
    name: &str,
    contexts: &[Entity],
    state: NativeReadiness,
    spell: u32,
    cast_unit: Entity,
) {
    for &candidate in contexts {
        record_probe(
            prepared,
            (name.into(), Some(candidate)),
            false,
            state,
            Execution::Spell(spell, cast_unit, Some(candidate)),
        );
    }
}

fn read_attacks(world: &World, actor: Entity, targets: &[Entity], ready: &mut ReadyActions) {
    let current = world.get::<crate::combat::AutoAttack>(actor);
    for (index, &target) in targets.iter().enumerate() {
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
    // SimC76f8118 sc_hunter.cpp7269–7274: start once; native repeats
    // own later shots. This applies to any native autorepeat, not a spell-ID branch.
    if crate::spell_cast::test_harness::read_native_auto_repeat(world, actor, id) {
        return Ok(());
    }
    let targets = if self_target {
        std::slice::from_ref(&actor)
    } else {
        enemies
    };
    for (index, &target) in targets.iter().enumerate() {
        let state = native_readiness(world, actor, id, target, None, prepared.after_channel)?;
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
            record_target_contexts(prepared, name, enemies, state, id, target);
        }
    }
    Ok(())
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
            target == actor,
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
        }),
        Err(crate::spell_cast::CastFailure::NotEnoughPower(_)) if after_channel => {
            Ok(NativeReadiness {
                ready: false,
                limited: None,
            })
        }
        Err(crate::spell_cast::CastFailure::NotEnoughPower(power)) => {
            match crate::spell_cast::read_native_cast_at_resource_max(
                world, actor, &spell, target, power,
            ) {
                Ok(_) => Ok(NativeReadiness {
                    ready: false,
                    limited: Some(power),
                }),
                Err(reason) => refused_admission(id, reason),
            }
        }
        Err(reason) => refused_admission(id, reason),
    }
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
        }),
    }
}

pub(crate) fn execute(world: &mut World, actor: Entity, action: Execution) -> Result<(), String> {
    match action {
        Execution::Snapshot => Err("snapshot execution belongs to the native stat recorder".into()),
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
            let targets = crate::spell_cast::CastTargets {
                unit: target,
                primary,
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
                .get::<super::super::profile_effects::Potion>(actor)
                .is_some_and(|potion| potion.0.item_id == item.item_id)
            {
                world
                    .entity_mut(actor)
                    .remove::<super::super::profile_effects::Potion>();
            }
            Ok(())
        }
    }
}

//! Demon Hunter soul-fragment lifecycle, shared by native producers and literal pickup.
//! SimC76f8118 sc_demon_hunter.cpp1656–1908,5069–5325,12540–12620.
use super::combat;
use bevy::prelude::*;
use combat::random::Rng;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::AfterCastHook;
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, Position};

const FRAGMENT: u32 = 204255;
const APPETITE: u32 = 206478;
const SPLITTER: u32 = 1266330;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentKind {
    Greater,
    Lesser,
    GreaterDemon,
    EmpoweredDemon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentFilter {
    Any,
    Lesser,
    Greater,
    Demon,
}

impl FragmentFilter {
    fn matches(self, kind: FragmentKind) -> bool {
        match self {
            Self::Any => true,
            Self::Lesser => kind == FragmentKind::Lesser,
            Self::Greater => kind != FragmentKind::Lesser,
            Self::Demon => matches!(
                kind,
                FragmentKind::GreaterDemon | FragmentKind::EmpoweredDemon
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupMode {
    Oldest,
    Newest,
    Nearest,
}

#[derive(Clone, Copy)]
struct Fragment {
    id: u64,
    kind: FragmentKind,
    position: Vec3,
    active_at: f64,
    expires_at: f64,
    activated: bool,
    consume_on_activation: bool,
}

#[derive(Clone, Copy)]
struct Pickup {
    id: u64,
    due: f64,
}
#[derive(Clone, Copy)]
struct Incoming {
    kind: FragmentKind,
    due: f64,
}
#[derive(Component, Default)]
struct Fragments {
    items: Vec<Fragment>,
    pickup: Option<Pickup>,
    incoming: Vec<Incoming>,
    next_id: u64,
}

pub(crate) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::OwnsPolicyProc(|spell| matches!(spell, APPETITE | SPLITTER)),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 162794 | 201427).then_some(appetite)
}

fn appetite(world: &mut World, owner: Entity, _: &SpellInfo) {
    if world
        .get::<ActiveSpec>(owner)
        .is_none_or(|spec| spec.0 != 577)
    {
        return;
    }
    if roll_owned(world, owner, APPETITE) {
        if let Err(error) = spawn(world, owner, FragmentKind::Lesser, false) {
            error!(%error, "Demonic Appetite failed");
        }
    }
}

fn roll_owned(world: &mut World, owner: Entity, spell: u32) -> bool {
    let id = world
        .get::<combat::auras::ActiveAuras>(owner)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == owner && aura.spell_id == spell)
        })
        .map(|aura| aura.instance_id);
    id.is_some_and(|id| combat::auras::procs::try_native_policy_proc(world, owner, id))
}

fn spec(world: &World, owner: Entity) -> Result<u32, String> {
    match world.get::<ActiveSpec>(owner).map(|s| s.0) {
        Some(value @ (577 | 581 | 1480)) => Ok(value),
        _ => Err("soul fragment owner has no Demon Hunter specialization".into()),
    }
}

fn native(world: &World, spell: u32) -> Result<&SpellInfo, String> {
    world
        .resource::<SpellStore>()
        .get(spell)
        .ok_or_else(|| format!("native soul spell {spell} missing"))
}

fn travel_speed(world: &World, spec: u32) -> Result<f32, String> {
    let spell = match spec {
        577 => 178963,
        581 => 210042,
        1480 => 1223423,
        _ => unreachable!(),
    };
    Ok(native(world, spell)?
        .misc
        .as_ref()
        .ok_or("soul travel metadata missing")?
        .speed)
}

fn activation_delay(spec: u32, speed: f32) -> f64 {
    if speed == 0.0 {
        return 0.0;
    }
    match spec {
        577 => 1.0,
        581 | 1480 => {
            let (mean, deviation) = if spec == 581 {
                (0.76, 0.12)
            } else {
                (0.23, 0.07)
            };
            let mut rng = combat::random::thread_rng();
            let u = rng.r#gen::<f64>().max(f64::MIN_POSITIVE);
            let v = rng.r#gen::<f64>();
            (mean + deviation * (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()).max(0.0)
        }
        _ => unreachable!(),
    }
}

/// Spawn one real fragment for an owner. Producers must call this only after their native trigger.
pub fn spawn(
    world: &mut World,
    owner: Entity,
    kind: FragmentKind,
    consume_on_activation: bool,
) -> Result<(), String> {
    spawn_one(world, owner, kind, consume_on_activation)?;
    if roll_owned(world, owner, SPLITTER) {
        spawn_one(world, owner, FragmentKind::Lesser, consume_on_activation)?;
    }
    Ok(())
}

fn spawn_one(
    world: &mut World,
    owner: Entity,
    kind: FragmentKind,
    consume_on_activation: bool,
) -> Result<(), String> {
    let specialization = spec(world, owner)?;
    let speed = travel_speed(world, specialization)?;
    let duration = native(world, FRAGMENT)?
        .misc
        .as_ref()
        .and_then(|m| m.duration)
        .ok_or("native fragment duration missing")?
        .duration_ms;
    if duration <= 0 || speed < 0.0 {
        return Err("invalid native fragment duration or speed".into());
    }
    let position = *world
        .get::<Position>(owner)
        .ok_or("fragment owner position missing")?;
    let next_id = world
        .get::<Fragments>(owner)
        .map_or(0, |state| state.next_id);
    let base = match specialization {
        577 => 4.6066,
        581 => 10.6066,
        1480 => 5.6066,
        _ => unreachable!(),
    };
    let mut rng = combat::random::thread_rng();
    let distance = if kind == FragmentKind::EmpoweredDemon {
        6.5
    } else {
        (21.0 * rng.r#gen::<f32>() + 4.0).sqrt()
    };
    let theta = std::f32::consts::TAU * rng.r#gen::<f32>();
    let offset = if kind == FragmentKind::EmpoweredDemon {
        Vec3::ZERO
    } else {
        Vec3::new(if next_id % 2 == 0 { base } else { -base }, 0.0, base)
    };
    let position = Vec3::new(position.x, position.y, position.z)
        + offset
        + Vec3::new(distance * theta.cos(), 0.0, distance * theta.sin());
    let now = combat::auras::aura_now(world);
    let active_at = now + activation_delay(specialization, speed);
    let fragment = Fragment {
        id: next_id,
        kind,
        position,
        active_at,
        expires_at: active_at + f64::from(duration) / 1000.0,
        activated: false,
        consume_on_activation,
    };
    let mut state = world
        .entity_mut(owner)
        .entry::<Fragments>()
        .or_default()
        .into_mut();
    state.next_id += 1;
    state.items.push(fragment);
    Ok(())
}

/// Count active, unexpired fragments of the requested source-defined group.
pub fn active_count(world: &World, owner: Entity, filter: FragmentFilter) -> usize {
    let now = combat::auras::aura_now(world);
    world.get::<Fragments>(owner).map_or(0, |state| {
        state
            .items
            .iter()
            .filter(|f| f.active_at <= now && now < f.expires_at && filter.matches(f.kind))
            .count()
    })
}

fn position(world: &World, owner: Entity) -> Option<Vec3> {
    world
        .get::<Position>(owner)
        .map(|p| Vec3::new(p.x, p.y, p.z))
}

fn movement_time(world: &World, owner: Entity, fragment: Fragment) -> Option<f64> {
    let spec = spec(world, owner).ok()?;
    let radius = if fragment.kind == FragmentKind::EmpoweredDemon {
        6.0
    } else if fragment.kind == FragmentKind::GreaterDemon {
        if spec == 577 { 10.0 } else { 6.0 }
    } else if spec == 577 {
        8.0
    } else {
        4.0
    };
    let speed = 7.0
        * world
            .get::<combat::auras::AuraStatMods>(owner)
            .map_or(1.0, |mods| mods.movement_speed);
    if speed <= 0.0 {
        return None;
    }
    Some(f64::from(
        (fragment.position.distance(position(world, owner)?) - radius).max(0.0) / speed,
    ))
}

#[derive(Clone, Copy)]
pub struct PickupPlan {
    id: u64,
    due: f64,
}

/// Select an active fragment that survives the pickup travel time.
pub fn plan_pickup(
    world: &World,
    owner: Entity,
    filter: FragmentFilter,
    mode: PickupMode,
) -> Option<PickupPlan> {
    let state = world.get::<Fragments>(owner)?;
    if state.pickup.is_some() {
        return None;
    }
    let now = combat::auras::aura_now(world);
    let mut candidates: Vec<_> = state
        .items
        .iter()
        .copied()
        .filter_map(|f| {
            let travel = movement_time(world, owner, f)?;
            (filter.matches(f.kind) && f.active_at <= now && f.expires_at > now + travel)
                .then_some((f, travel))
        })
        .collect();
    candidates.sort_by(|a, b| match mode {
        PickupMode::Oldest => a.0.expires_at.total_cmp(&b.0.expires_at),
        PickupMode::Newest => b.0.expires_at.total_cmp(&a.0.expires_at),
        PickupMode::Nearest => a.1.total_cmp(&b.1),
    });
    candidates.first().map(|(f, travel)| PickupPlan {
        id: f.id,
        due: now + travel,
    })
}

pub fn begin_pickup(world: &mut World, owner: Entity, plan: PickupPlan) -> Result<(), String> {
    let mut state = world
        .get_mut::<Fragments>(owner)
        .ok_or("fragment state absent")?;
    if state.pickup.is_some() || !state.items.iter().any(|f| f.id == plan.id) {
        return Err("fragment pickup no longer ready".into());
    }
    state.pickup = Some(Pickup {
        id: plan.id,
        due: plan.due,
    });
    Ok(())
}

pub fn pickup_due(world: &World, owner: Entity) -> bool {
    let now = combat::auras::aura_now(world);
    world
        .get::<Fragments>(owner)
        .and_then(|s| s.pickup)
        .is_some_and(|pickup| pickup.due <= now)
}

/// Complete a pickup only if the literal row still permits it at arrival.
pub fn finish_pickup(world: &mut World, owner: Entity, permitted: bool) -> Result<(), String> {
    let now = combat::auras::aura_now(world);
    let Some(mut state) = world.entity_mut(owner).take::<Fragments>() else {
        return Ok(());
    };
    if let Some(pickup) = state.pickup.take()
        && permitted
        && let Some(index) = state
            .items
            .iter()
            .position(|f| f.id == pickup.id && f.active_at <= now && now < f.expires_at)
    {
        consume(world, owner, &mut state, index, now)?;
    }
    world.entity_mut(owner).insert(state);
    Ok(())
}

fn consume(
    world: &World,
    owner: Entity,
    state: &mut Fragments,
    index: usize,
    now: f64,
) -> Result<(), String> {
    let fragment = state.items.remove(index);
    let speed = travel_speed(world, spec(world, owner)?)?;
    let distance = fragment
        .position
        .distance(position(world, owner).ok_or("fragment owner position missing")?);
    let delay = if speed == 0.0 {
        0.0
    } else {
        f64::from(distance / speed)
    };
    state.incoming.push(Incoming {
        kind: fragment.kind,
        due: now + delay,
    });
    Ok(())
}

pub(crate) fn update(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let owners: Vec<Entity> = world
        .query_filtered::<Entity, With<Fragments>>()
        .iter(world)
        .collect();
    for owner in owners {
        if let Err(error) = update_owner(world, owner, now) {
            error!(?owner, %error, "Soul fragment update failed");
        }
    }
}

fn update_owner(world: &mut World, owner: Entity, now: f64) -> Result<(), String> {
    let specialization = spec(world, owner)?;
    let mut state = world
        .entity_mut(owner)
        .take::<Fragments>()
        .ok_or("fragment state absent")?;
    state.items.retain(|f| f.expires_at > now);
    let mut index = 0;
    while index < state.items.len() {
        let f = state.items[index];
        if !f.activated && f.active_at <= now {
            state.items[index].activated = true;
            let auto = specialization == 1480
                && f.position
                    .distance(position(world, owner).ok_or("fragment owner position missing")?)
                    <= 4.0;
            if f.consume_on_activation || auto {
                consume(world, owner, &mut state, index, now)?;
                continue;
            }
            let cap = match specialization {
                577 => 5,
                581 => 6,
                1480 => 10,
                _ => unreachable!(),
            };
            if f.kind == FragmentKind::Lesser
                && state
                    .items
                    .iter()
                    .filter(|item| item.kind == FragmentKind::Lesser && item.activated)
                    .count()
                    > cap
            {
                let oldest = state
                    .items
                    .iter()
                    .position(|item| item.kind == FragmentKind::Lesser && item.activated)
                    .expect("active overflow fragment");
                consume(world, owner, &mut state, oldest, now)?;
                state.pickup = None;
                if oldest <= index {
                    index -= 1;
                }
            }
        }
        index += 1;
    }
    let mut due = Vec::new();
    state.incoming.retain(|item| {
        if item.due <= now {
            due.push(item.kind);
            false
        } else {
            true
        }
    });
    world.entity_mut(owner).insert(state);
    for kind in due {
        cast_consume(world, owner, specialization, kind);
    }
    Ok(())
}

fn cast_consume(world: &mut World, owner: Entity, specialization: u32, kind: FragmentKind) {
    let lesser = kind == FragmentKind::Lesser;
    let spell = match (specialization, lesser) {
        (577, true) => 228532,
        (577, false) => 178963,
        (581, true) => 203794,
        (581, false) => 210042,
        (1480, _) => 1266301,
        _ => unreachable!(),
    };
    combat::spell_cast::cast_triggered(world, owner, spell, Some(owner), 1);
    if specialization == 1480 {
        combat::spell_cast::cast_triggered(world, owner, 1223628, Some(owner), 1);
    }
}

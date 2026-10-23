//! TC a352b1fa Spell.cpp1690–1738: TargetA modifies the destination before TargetB.
//! SPELL_DEST_DETOUR_RAYCAST_ABSENT: Object.cpp2853–2866; use existing baked/live
//! direct collision, not a claim of Detour path parity.
use super::*;

const FORCE_DEST_LOCATION: u32 = 0x1;
const NO_MOVEMENT_RADIUS_BONUS: u32 = 0x4000_0000;
const RUNNING_RANGE_BONUS: f32 = 2.0;
const FORCE_LOCATION_GAP: &str = "SPELL_DEST_FORCE_LOCATION_MAX_HEIGHT_ABSENT";

pub(in crate::spell_cast) fn check_destination_support(
    spell: &SpellInfo,
) -> Result<(), super::super::CastFailure> {
    let needs_force_height = super::super::attributes(spell, 9) & FORCE_DEST_LOCATION != 0
        && spell.effects.iter().any(|effect| {
            effect
                .implicit_targets
                .iter()
                .any(|&id| matches!(id, 73 | 74 | 148))
        });
    if needs_force_height {
        // Spell.cpp9286–9292 selects Object.cpp2802–2826 MAX_HEIGHT/ten-step queries.
        return Err(super::super::CastFailure::Unsupported(
            FORCE_LOCATION_GAP.into(),
        ));
    }
    Ok(())
}

pub(super) fn sets_destination(selector: Selector) -> bool {
    matches!(
        selector,
        Selector::CenterCaster
            | Selector::ExistingDest
            | Selector::DestTowardsCaster
            | Selector::CasterRandom
            | Selector::CasterRadius
            | Selector::DestTargetRandom
            | Selector::DestRandom
            | Selector::DestOffset(_)
            | Selector::ForwardLeap
            | Selector::CenterExplicit(_)
            | Selector::DestTargetOffset(_)
            | Selector::ChannelDestination
            | Selector::CasterOffset(_)
    )
}

pub(in crate::spell_cast) fn effect_destination(
    world: &mut World,
    caster: Entity,
    explicit: Option<Entity>,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> Result<Option<Dest>, super::super::CastFailure> {
    check_destination_support(spell)?;
    let mut destination = spell.cast_destination.map(|point| Dest {
        position: Vec3::from_array(point),
        orientation: orientation(world, caster),
    });
    for slot in 0..effect.implicit_targets.len() {
        destination = select_slot(world, caster, explicit, spell, effect, slot, destination)?;
    }
    Ok(destination)
}

pub(super) fn select_slot(
    world: &mut World,
    caster: Entity,
    explicit: Option<Entity>,
    spell: &SpellInfo,
    effect: &SpellEffect,
    slot: usize,
    previous: Option<Dest>,
) -> Result<Option<Dest>, super::super::CastFailure> {
    check_destination_support(spell)?;
    let id = effect.implicit_targets[slot];
    if id == 73 {
        return select_caster_radius(world, caster, spell, effect, slot).map(Some);
    }
    let destination = effect_dest_slot(world, caster, explicit, effect, slot, id, previous);
    match id {
        74 => {
            let target = explicit
                .filter(|target| same_map(world, caster, *target))
                .ok_or(super::super::CastFailure::InvalidTarget)?;
            let start = position(world, target).ok_or(super::super::CastFailure::InvalidTarget)?;
            let destination = destination.ok_or(super::super::CastFailure::InvalidTarget)?;
            move_destination(world, target, start, destination, 74).map(Some)
        }
        148 | 78 | 79 => {
            let start = previous
                .map(|dest| dest.position)
                .or_else(|| position(world, explicit.unwrap_or(caster)))
                .ok_or(super::super::CastFailure::InvalidTarget)?;
            let destination = destination.ok_or(super::super::CastFailure::InvalidTarget)?;
            move_destination(world, caster, start, destination, 148).map(Some)
        }
        62 | 138 => ground_destination(world, caster, effect, destination),
        _ => Ok(destination),
    }
}

/// Spell.cpp1611–1642:73 retains CalcRadius extent, then applies caster reach.
fn select_caster_radius(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    slot: usize,
) -> Result<Dest, super::super::CastFailure> {
    let start = position(world, caster).ok_or(super::super::CastFailure::InvalidTarget)?;
    let reach = crate::scripted_ai::model_reach::combat_reach(world, caster).ok_or_else(|| {
        super::super::CastFailure::Unsupported("target73 requires caster combat reach".into())
    })?;
    let distance = caster_radius_extent(world, caster, spell, effect, slot).max(reach);
    let angle = rand::Rng::gen_range(&mut rand::thread_rng(), 0.0..std::f32::consts::TAU);
    let destination = Dest {
        position: start + Vec3::new(distance * angle.cos(), 0.0, -distance * angle.sin()),
        orientation: orientation(world, caster),
    };
    move_destination(world, caster, start, destination, 73)
}

/// CalcRadius(m_caster): absent B radius inherits A's entry; a missing entry is0.
fn caster_radius_extent(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    slot: usize,
) -> f32 {
    let source_slot = radius_slot(effect, slot);
    let Some(entry) = effect.radius[source_slot] else {
        return 0.0;
    };
    let level = world
        .get::<shared::components::Level>(caster)
        .map(|level| level.0)
        .or_else(|| {
            world
                .get::<shared::components::UnitLevel>(caster)
                .map(|level| level.0)
        });
    let extent = level.map_or(entry.radius_max, |level| {
        (entry.radius + entry.radius_per_level * f32::from(level)).min(entry.radius_max)
    });
    let bonus = if super::super::attributes(spell, 9) & NO_MOVEMENT_RADIUS_BONUS == 0
        && has_running_range_bonus(world, caster)
    {
        RUNNING_RANGE_BONUS
    } else {
        0.0
    };
    let extent = extent + bonus;
    if matches!(effect.implicit_targets[source_slot], 72 | 74 | 86) {
        let minimum = (entry.radius_min - bonus).max(0.0);
        let fraction = rand::Rng::gen_range(&mut rand::thread_rng(), 0.0_f32..1.0);
        (extent - minimum) * fraction.sqrt()
    } else {
        extent
    }
}

/// CanIncreaseRangeByMovement: forward/strafe/falling, never walk mode.
fn has_running_range_bonus(world: &World, caster: Entity) -> bool {
    use shared::components::PlayerMotion;
    let running = PlayerMotion::FORWARD
        | PlayerMotion::STRAFE_LEFT
        | PlayerMotion::STRAFE_RIGHT
        | PlayerMotion::FALLING;
    world
        .get::<PlayerMotion>(caster)
        .is_some_and(|motion| motion.0 & running != 0 && !motion.contains(PlayerMotion::WALKING))
}

/// Supported direct-ray branch of Object.cpp2833–2920, using the same collision
/// and dry/non-hover allowed-Z services as native pull destinations.
fn move_destination(
    world: &mut World,
    reference: Entity,
    start: Vec3,
    mut destination: Dest,
    selector: u32,
) -> Result<Dest, super::super::CastFailure> {
    let map = world
        .get::<crate::maps::InMap>(reference)
        .copied()
        .ok_or(super::super::CastFailure::InvalidTarget)?;
    let los = world
        .get_resource::<crate::line_of_sight::LineOfSight>()
        .ok_or_else(|| {
            super::super::CastFailure::Unsupported(format!(
                "target{selector} requires collision data"
            ))
        })?;
    destination.position =
        super::super::teleport::clamp_to_wall(los, map, start, destination.position);
    let can_fly = caster_can_fly(world, reference);
    let mut ground = world
        .get_resource_mut::<crate::ground::GroundMap>()
        .ok_or_else(|| {
            super::super::CastFailure::Unsupported(format!("target{selector} requires ground data"))
        })?;
    let crate::ground::GroundProbe::Supported(height) =
        ground.creature_ground_in(map, destination.position)
    else {
        return Err(super::super::CastFailure::InvalidTarget);
    };
    destination.position.y = if can_fly {
        destination.position.y.max(height)
    } else {
        height
    };
    Ok(destination)
}

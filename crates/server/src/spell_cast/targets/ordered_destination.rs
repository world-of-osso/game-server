//! TC a352b1fa Spell.cpp1690–1738: TargetA modifies the destination before TargetB.
//! SPELL_DEST_DETOUR_RAYCAST_ABSENT: Object.cpp2853–2866; use existing baked/live
//! direct collision, not a claim of Detour path parity.
use super::*;

const FORCE_DEST_LOCATION: u32 = 0x1;
const FORCE_LOCATION_GAP: &str = "SPELL_DEST_FORCE_LOCATION_MAX_HEIGHT_ABSENT";

pub(in crate::spell_cast) fn check_destination_support(
    spell: &SpellInfo,
) -> Result<(), super::super::CastFailure> {
    let needs_force_height = super::super::attributes(spell, 9) & FORCE_DEST_LOCATION != 0
        && spell
            .effects
            .iter()
            .any(|effect| effect.implicit_targets.contains(&148));
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
    let destination = effect_dest_slot(world, caster, explicit, effect, slot, id, previous);
    match id {
        148 | 78 | 79 => {
            let start = previous
                .map(|dest| dest.position)
                .or_else(|| position(world, explicit.unwrap_or(caster)))
                .ok_or(super::super::CastFailure::InvalidTarget)?;
            let destination = destination.ok_or(super::super::CastFailure::InvalidTarget)?;
            move_destination(world, caster, start, destination).map(Some)
        }
        62 | 138 => ground_destination(world, caster, effect, destination),
        _ => Ok(destination),
    }
}

/// Supported direct-ray branch of Object.cpp2833–2920, using the same collision
/// and dry/non-hover allowed-Z services as native pull destinations.
fn move_destination(
    world: &mut World,
    caster: Entity,
    start: Vec3,
    mut destination: Dest,
) -> Result<Dest, super::super::CastFailure> {
    let map = world
        .get::<crate::maps::InMap>(caster)
        .copied()
        .ok_or(super::super::CastFailure::InvalidTarget)?;
    let los = world
        .get_resource::<crate::line_of_sight::LineOfSight>()
        .ok_or_else(|| {
            super::super::CastFailure::Unsupported("target148 requires collision data".into())
        })?;
    destination.position =
        super::super::teleport::clamp_to_wall(los, map, start, destination.position);
    let can_fly = caster_can_fly(world, caster);
    let mut ground = world
        .get_resource_mut::<crate::ground::GroundMap>()
        .ok_or_else(|| {
            super::super::CastFailure::Unsupported("target148 requires ground data".into())
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

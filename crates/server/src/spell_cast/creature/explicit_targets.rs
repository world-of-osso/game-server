//! TC a352b1fa Spell.cpp621–678 removes redundant unit objects before CheckCast,
//! while retaining the original object position for a required destination.
//! Classification below follows SpellInfo.cpp's implicit/effect static tables,
//! GetMissingTargetMask and _InitializeExplicitTargetMask; no SpellID exceptions.
use bevy::prelude::*;

use super::super::targets;
use crate::spell_info::{SpellEffect, SpellInfo};

// SpellDefines.h SpellCastTargetFlags (unit mask plus the two corpse flags).
const UNIT_OR_CORPSE_FLAGS: u32 = 0x0011_878e;
const DESTINATION_FLAG: u32 = 0x40;

// SpellInfo.cpp959ff: EFFECT_IMPLICIT_TARGET_EXPLICIT / TARGET_OBJECT_TYPE_UNIT.
const EXPLICIT_UNIT_EFFECTS: &[u32] = &[
    1, 2, 6, 7, 8, 9, 10, 11, 14, 16, 17, 19, 24, 31, 35, 36, 38, 40, 41, 44, 45, 52, 55, 57, 58,
    59, 62, 63, 65, 66, 67, 68, 70, 71, 73, 75, 80, 81, 82, 90, 91, 92, 95, 96, 98, 100, 102, 103,
    105, 107, 108, 111, 112, 113, 114, 115, 117, 119, 120, 121, 123, 124, 125, 126, 128, 129, 130,
    132, 133, 136, 137, 138, 139, 140, 141, 142, 143, 146, 147, 150, 153, 154, 157, 159, 160, 161,
    162, 164, 165, 167, 168, 170, 174, 176, 177, 184, 187, 189, 190, 191, 193, 199, 202, 203, 204,
    205, 207, 208, 210, 211, 212, 214, 215, 216, 217, 220, 221, 222, 224, 225, 226, 227, 230, 231,
    232, 236, 237, 238, 239, 240, 242, 244, 249, 250, 251, 253, 255, 265, 268, 270, 271, 272, 276,
    277, 279, 281, 282, 283, 284, 285, 286, 288, 289, 290, 291, 292, 293, 294, 298, 303, 304, 306,
    311, 316, 317, 335, 336, 337, 338, 341,
];
const EXPLICIT_UNIT_DEST_EFFECTS: &[u32] = &[13, 15, 29, 43, 69, 83, 144, 145, 252];
const EXPLICIT_DEST_EFFECTS: &[u32] = &[27, 28, 50, 56, 72, 76, 104, 106, 109, 135, 149, 188];

pub(super) fn normalize_unit_target(
    world: &World,
    spell: &mut SpellInfo,
    target: Option<Entity>,
) -> Option<Entity> {
    let unit = target?;
    let is_unit = world.get::<shared::components::Health>(unit).is_some()
        && world
            .get::<shared::protocol::GameObjectInfo>(unit)
            .is_none();
    if !is_unit || needs_explicit_unit(spell) {
        return target;
    }
    if spell.cast_destination.is_none() && needs_explicit_destination(spell) {
        spell.cast_destination = targets::position(world, unit).map(|position| position.to_array());
    }
    None
}

fn needs_explicit_unit(spell: &SpellInfo) -> bool {
    let flags = spell
        .target_restrictions
        .as_ref()
        .map_or(0, |row| row.targets);
    if flags & UNIT_OR_CORPSE_FLAGS != 0 {
        return true;
    }
    let has_range = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .is_some_and(|range| range.max_hostile > 0.0 || range.max_friendly > 0.0);
    spell
        .effects
        .iter()
        .filter(|effect| effect.effect != 0)
        .any(|effect| {
            effect
                .implicit_targets
                .into_iter()
                .any(references_explicit_unit)
                || (has_range && missing_unit_for_effect(effect))
        })
}

fn references_explicit_unit(target: u32) -> bool {
    // TARGET_REFERENCE_TYPE_TARGET with UNIT, UNIT_AND_DEST or DEST object type.
    matches!(
        target,
        6 | 21
            | 25
            | 35
            | 45
            | 57
            | 61
            | 90
            | 95
            | 118
            | 53
            | 63
            | 64
            | 65
            | 66
            | 67
            | 68
            | 69
            | 70
            | 71
            | 74
            | 75
            | 113
            | 132
    )
}

fn provides_unit(target: u32) -> bool {
    // TARGET_OBJECT_TYPE_UNIT or UNIT_AND_DEST (including non-explicit areas).
    matches!(
        target,
        1 | 2
            | 3
            | 4
            | 5
            | 6
            | 7
            | 8
            | 11
            | 15
            | 16
            | 20
            | 21
            | 24
            | 25
            | 27
            | 30
            | 31
            | 33
            | 34
            | 35
            | 37
            | 38
            | 45
            | 54
            | 56
            | 57
            | 58
            | 59
            | 60
            | 61
            | 77
            | 90
            | 92
            | 94
            | 95
            | 96
            | 97
            | 98
            | 99
            | 100
            | 101
            | 102
            | 103
            | 104
            | 105
            | 110
            | 115
            | 116
            | 118
            | 120
            | 122
            | 123
            | 124
            | 126
            | 128
            | 129
            | 130
            | 133
            | 134
            | 135
            | 136
            | 150
            | 151
    )
}

fn provides_corpse(target: u32) -> bool {
    matches!(target, 93 | 119 | 121)
}

fn missing_unit_for_effect(effect: &SpellEffect) -> bool {
    let corpse_provided = effect.implicit_targets.into_iter().any(provides_corpse);
    if matches!(effect.effect, 18 | 116 | 172) {
        return !corpse_provided;
    }
    let needs_unit = EXPLICIT_UNIT_EFFECTS.contains(&effect.effect)
        || EXPLICIT_UNIT_DEST_EFFECTS.contains(&effect.effect);
    let unit_provided = effect.implicit_targets.into_iter().any(provides_unit);
    needs_unit && !unit_provided && !corpse_provided
}

fn sets_destination(target: u32) -> bool {
    matches!(
        target,
        9 | 17
            | 18
            | 28
            | 29
            | 32
            | 36
            | 39
            | 41
            | 42
            | 43
            | 44
            | 46
            | 47
            | 48
            | 49
            | 50
            | 53
            | 55
            | 62
            | 63
            | 64
            | 65
            | 66
            | 67
            | 68
            | 69
            | 70
            | 71
            | 72
            | 73
            | 74
            | 75
            | 76
            | 78
            | 79
            | 80
            | 81
            | 82
            | 83
            | 84
            | 85
            | 86
            | 87
            | 88
            | 89
            | 91
            | 106
            | 107
            | 112
            | 113
            | 116
            | 125
            | 127
            | 131
            | 132
            | 137
            | 138
            | 140
            | 142
            | 148
            | 149
    )
}

fn references_destination(target: u32) -> bool {
    matches!(
        target,
        8 | 16
            | 28
            | 29
            | 31
            | 34
            | 52
            | 78
            | 79
            | 80
            | 81
            | 82
            | 83
            | 84
            | 85
            | 86
            | 87
            | 88
            | 89
            | 91
            | 133
            | 134
            | 135
            | 138
            | 148
    )
}

fn needs_explicit_destination(spell: &SpellInfo) -> bool {
    if spell
        .target_restrictions
        .as_ref()
        .is_some_and(|row| row.targets & DESTINATION_FLAG != 0)
    {
        return true;
    }
    let has_range = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .is_some_and(|range| range.max_hostile > 0.0 || range.max_friendly > 0.0);
    let mut destination_set = false;
    for effect in spell.effects.iter().filter(|effect| effect.effect != 0) {
        for target in effect.implicit_targets {
            if references_destination(target) && !destination_set {
                return true;
            }
            destination_set |= sets_destination(target);
        }
        let needs_destination = EXPLICIT_DEST_EFFECTS.contains(&effect.effect)
            || EXPLICIT_UNIT_DEST_EFFECTS.contains(&effect.effect);
        if has_range && needs_destination && !destination_set {
            return true;
        }
    }
    false
}

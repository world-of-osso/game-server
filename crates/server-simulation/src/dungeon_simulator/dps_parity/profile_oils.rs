//! SimC76f8118 player.cpp1784–1877/item.cpp155–179: native temporary enchant selection.
use bevy::prelude::*;
use server::simc_apl::Expression;
use shared::protocol::EquipmentSlot;

const PHOENIX_OIL_QUALITY2: u32 = 8052;

struct Oil {
    slot: EquipmentSlot,
    condition: Option<Expression>,
}

fn parse(entry: &str) -> Result<Option<Oil>, String> {
    let (value, options) = entry
        .split_once(',')
        .map_or((entry, None), |(v, o)| (v, Some(o)));
    let (slot, name) = value
        .split_once(':')
        .ok_or("temporary enchant lacks slot")?;
    if name != "thalassian_phoenix_oil_2" {
        return Ok(None);
    }
    let condition = options
        .map(|options| {
            let expression = options
                .strip_prefix("if=")
                .ok_or("unsupported temporary enchant option")?;
            Expression::parse(expression)
        })
        .transpose()?;
    Ok(Some(Oil {
        slot: super::inputs::slot(slot)?,
        condition,
    }))
}

pub(super) fn supported(value: &str) -> bool {
    value
        .split('/')
        .all(|entry| matches!(parse(entry), Ok(Some(_))))
}

fn condition_active(world: &World, actor: Entity, expression: &Expression) -> Result<bool, String> {
    use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
    // The shipped condition names the native talent318038, not its weapon buff.
    let bindings = Bindings {
        talents: [("flametongue_weapon".into(), 318038)].into(),
        ..Default::default()
    };
    let facts = NativeFacts::new(
        world,
        actor,
        &bindings,
        Encounter::Dungeon {
            started_at: crate::spell_cast::now(world),
        },
        0,
    );
    Ok(expression.evaluate(&facts)? != 0.0)
}

pub(super) fn install(world: &mut World, actor: Entity, value: &str) -> Result<(), String> {
    let mut oils = Vec::new();
    for entry in value.split('/') {
        let Some(oil) = parse(entry)? else {
            continue;
        };
        let equipped = world
            .get::<crate::stats::EquippedItems>(actor)
            .ok_or("temporary enchant owner lacks native equipment")?;
        // item.cpp2237–2245: inactive slots do not initialize enchants.
        if !equipped.items.iter().any(|(slot, _, _)| *slot == oil.slot) {
            continue;
        }
        if let Some(condition) = oil.condition {
            if !condition_active(world, actor, &condition)? {
                continue;
            }
        }
        // SimC temporary_enchant.inc116 binds quality2 to native enchant8052.
        oils.push((oil.slot, PHOENIX_OIL_QUALITY2));
    }
    world
        .get_mut::<crate::stats::EquippedItemModifiers>(actor)
        .ok_or("temporary enchant owner lacks native equipment")?
        .temporary_enchants = oils;
    Ok(())
}

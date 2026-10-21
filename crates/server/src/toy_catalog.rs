//! Toy.db2 identities, joined to Retail item and spell tables in world.db.
use crate::spell_info::{SpellEffect, SpellStore};
use bevy::prelude::*;
use rusqlite::Connection;
use shared::protocol::ToySnapshot;
use std::collections::{BTreeMap, BTreeSet};

#[path = "toy_interaction.rs"]
mod interaction;

#[derive(Clone, Debug)]
pub(crate) struct Toy {
    pub snapshot: ToySnapshot,
    pub item_cooldown_ms: i32,
    pub category: u32,
    pub category_cooldown_ms: i32,
    pub condition_id: u32,
}

#[derive(Resource, Default)]
pub(crate) struct ToyCatalog(pub BTreeMap<u32, Toy>);

pub(crate) fn load(conn: &Connection) -> rusqlite::Result<ToyCatalog> {
    let interaction_gaps = interaction::load(conn)?;
    let mut query = conn.prepare(
        "SELECT t.ItemID, s.Display_lang, i.IconFileDataID, s.ExpansionID,
                t.Flags, t.SourceTypeEnum, t.SourceText_lang, e.SpellID,
                e.CoolDownMSec, e.SpellCategoryID, e.CategoryCoolDownMSec,
                e.PlayerConditionID, e.ChrSpecializationID, c.ID
         FROM toy t
         LEFT JOIN item i ON i.ID=t.ItemID
         LEFT JOIN item_sparse s ON s.ID=t.ItemID
         LEFT JOIN content_item c ON c.ID=t.ItemID
         LEFT JOIN (
             SELECT x.ItemID,e.*,ROW_NUMBER() OVER (
                 PARTITION BY x.ItemID ORDER BY e.LegacySlotIndex,e.ID) AS slot
             FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID
             WHERE e.TriggerType=0 AND e.SpellID>0
         ) e ON e.ItemID=t.ItemID AND e.slot=1
         ORDER BY t.ItemID",
    )?;
    let rows = query
        .query_map([], |r| {
            let name: Option<String> = r.get(1)?;
            let icon: Option<u32> = r.get(2)?;
            let spell_id: Option<u32> = r.get(7)?;
            let spec: Option<u32> = r.get(12)?;
            let unavailable_reason = if name.is_none() || icon.is_none() {
                Some("unresolved Item/ItemSparse row".into())
            } else if spell_id.is_none() {
                Some("unresolved ItemEffect on-use spell".into())
            } else if r.get::<_, Option<u32>>(13)?.is_none() {
                Some("item absent from runtime content_item catalog".into())
            } else if spec.is_some_and(|id| id != 0) {
                Some("toy specialization restriction not supported".into())
            } else {
                interaction::unavailable(&interaction_gaps, r.get(0)?, spell_id)
            };
            Ok(Toy {
                snapshot: ToySnapshot {
                    item_id: r.get(0)?,
                    name: name.unwrap_or_default(),
                    icon_file_data_id: icon.unwrap_or_default(),
                    expansion_id: r.get::<_, Option<i32>>(3)?.unwrap_or(0),
                    flags: r.get(4)?,
                    source_type: r.get(5)?,
                    source_text: r.get(6)?,
                    spell_id,
                    requires_target_witness: false,
                    learned: false,
                    favourite: false,
                    unavailable_reason,
                },
                item_cooldown_ms: r.get::<_, Option<i32>>(8)?.unwrap_or(-1),
                category: r.get::<_, Option<u32>>(9)?.unwrap_or(0),
                category_cooldown_ms: r.get::<_, Option<i32>>(10)?.unwrap_or(-1),
                condition_id: r.get::<_, Option<u32>>(11)?.unwrap_or(0),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ToyCatalog(
        rows.into_iter()
            .map(|toy| (toy.snapshot.item_id, toy))
            .collect(),
    ))
}

/// Registry-backed census and preflight, including reachable triggered spells.
/// Script aura types without a toy script are not implemented merely by replication.
pub(crate) fn unsupported_types(world: &World, spell_id: u32) -> BTreeSet<String> {
    classify_types(world, spell_id).0
}

/// Reasons plus supported aura types, including reachable trigger-spell effects.
pub(crate) fn classify_types(world: &World, spell_id: u32) -> (BTreeSet<String>, BTreeSet<u32>) {
    let mut reasons = BTreeSet::new();
    let mut supported = BTreeSet::new();
    classify(
        world,
        spell_id,
        0,
        &mut BTreeSet::new(),
        &mut reasons,
        &mut supported,
    );
    (reasons, supported)
}

fn classify(
    world: &World,
    spell_id: u32,
    depth: u8,
    visiting: &mut BTreeSet<u32>,
    reasons: &mut BTreeSet<String>,
    supported_auras: &mut BTreeSet<u32>,
) {
    if !visiting.insert(spell_id) {
        reasons.insert("trigger cycle".into());
        return;
    }
    let Some(spell) = world.resource::<SpellStore>().get(spell_id) else {
        reasons.insert("missing spell".into());
        return;
    };
    if let Some(reason) = crate::spell_cast::toy_scripts::unsupported(world, spell) {
        reasons.insert(reason);
    }
    if spell.effects.is_empty() {
        reasons.insert("missing effects".into());
    }
    for effect in &spell.effects {
        if effect.effect == 0 {
            continue;
        }
        let supported = supports_effect(world, spell.id, effect)
            || crate::spell_cast::cosmetic::is_visual_aura(spell, effect);
        if supported && crate::auras::is_aura_effect(effect) {
            supported_auras.insert(effect.aura);
        }
        if !crate::spell_cast::cosmetic::is_visual_only(spell) && !supported {
            let kind = if crate::auras::is_aura_effect(effect) {
                "aura"
            } else {
                "effect"
            };
            let id = if crate::auras::is_aura_effect(effect) {
                effect.aura
            } else {
                effect.effect
            };
            reasons.insert(format!("{kind} {id}"));
        }
        if effect.aura == crate::auras::handlers::LINKED_SUMMON {
            if let Some(reason) =
                crate::auras::linked_summon::unsupported_reason(world, effect.trigger_spell)
            {
                reasons.insert(reason);
            }
        }
        if effect.aura == crate::auras::handlers::OVERRIDE_SUMMONED_OBJECT {
            reasons.insert(format!("summoned-object override {} -> {} requires transmitted-object/fishing runtime and object template", effect.misc_values[0], effect.misc_values[1]));
        }
        if effect.aura == crate::auras::handlers::PERIODIC_DUMMY {
            if let Err(reason) = crate::auras::periodic_dummy_trigger(world, effect) {
                reasons.insert(reason.into());
            }
        }
        if effect.effect == crate::game_objects::summoned::EFFECT_TRANSMITTED {
            if let Some(reason) = crate::game_objects::summoned::unsupported_reason(world, effect) {
                reasons.insert(reason);
            }
        }
        if effect.effect == crate::game_objects::summoned::EFFECT_SUMMON_OBJECT_WILD {
            if let Some(reason) =
                crate::game_objects::summoned::wild_unsupported_reason(world, effect)
            {
                reasons.insert(reason);
            }
        }
        let summons_object = [
            crate::game_objects::summoned::EFFECT_TRANSMITTED,
            crate::game_objects::summoned::EFFECT_SUMMON_OBJECT_WILD,
        ]
        .contains(&effect.effect);
        if summons_object {
            // A spellcaster object's use is the toy's action: its spell must work too.
            let entry = effect.misc_values[0] as u32;
            if let Some(object_spell) =
                crate::game_objects::summoned::spellcaster_spell(world, entry)
            {
                classify(
                    world,
                    object_spell,
                    depth + 1,
                    visiting,
                    reasons,
                    supported_auras,
                );
            }
        }
        if effect.effect == crate::spell_cast::summon::EFFECT_SUMMON {
            if let Some(reason) = crate::spell_cast::summon::unsupported_reason(world, effect) {
                reasons.insert(reason);
            }
        }
        let source_owned_flag_target =
            spell.id == 51640 && crate::spell_cast::toy_scripts::supports_effect(spell.id, effect);
        if !source_owned_flag_target
            && let Some(target) = crate::spell_cast::targets::unsupported_target(effect)
        {
            reasons.insert(format!("target {target}"));
        }
        let script_owns_trigger = crate::spell_cast::toy_scripts::owns_triggered_spell(spell.id);
        if effect.trigger_spell != 0 && !script_owns_trigger {
            if depth >= crate::spell_cast::effects::MAX_TRIGGER_DEPTH {
                reasons.insert("trigger depth limit".into());
            } else {
                classify(
                    world,
                    effect.trigger_spell,
                    depth + 1,
                    visiting,
                    reasons,
                    supported_auras,
                );
            }
        }
    }
    visiting.remove(&spell_id);
}

fn supports_effect(world: &World, spell_id: u32, effect: &SpellEffect) -> bool {
    use crate::auras::handlers::{AuraKind, TRANSFORM};
    if effect.aura == TRANSFORM {
        if effect.misc_values[0] == 0 {
            return crate::auras::supports_zero_entry_costume(spell_id);
        }
        return effect.misc_values[0] > 0
            && world
                .get_resource::<crate::world_data::WorldData>()
                .is_some_and(|data| data.templates.contains_key(&(effect.misc_values[0] as u32)));
    }
    if effect.aura == crate::auras::handlers::OVERRIDE_SPELLS {
        return effect.misc_values[0] > 0
            && crate::auras::override_spell_data::supported(world, effect.misc_values[0] as u32);
    }
    if effect.aura == crate::auras::handlers::LINKED_SUMMON {
        return crate::auras::linked_summon::unsupported_reason(world, effect.trigger_spell)
            .is_none();
    }
    if effect.aura == crate::auras::handlers::PERIODIC_DUMMY {
        return crate::auras::periodic_dummy_trigger(world, effect).is_ok();
    }
    if effect.aura == crate::auras::handlers::MOD_RATING {
        // Other rating masks need distinct replicated ratings, not a guessed merge.
        return effect.misc_values[0] == crate::auras::handlers::VERSATILITY_RATING_MASK;
    }
    if effect.aura == crate::auras::handlers::SCREEN_EFFECT {
        return effect.misc_values[0] > 0;
    }
    if crate::auras::is_aura_effect(effect) {
        return crate::auras::handlers::kind(effect.aura)
            .is_some_and(|kind| kind != AuraKind::Script);
    }
    if effect.effect == crate::game_objects::summoned::EFFECT_TRANSMITTED {
        return crate::game_objects::summoned::unsupported_reason(world, effect).is_none();
    }
    if effect.effect == crate::game_objects::summoned::EFFECT_SUMMON_OBJECT_WILD {
        return crate::game_objects::summoned::wild_unsupported_reason(world, effect).is_none();
    }
    if effect.effect == crate::spell_cast::summon::EFFECT_SUMMON {
        return crate::spell_cast::summon::unsupported_reason(world, effect).is_none();
    }
    if effect.effect == crate::spell_cast::area_trigger::EFFECT_CREATE_AREATRIGGER {
        // Lifecycle-only area triggers do not implement a toy's actual action.
        return effect.trigger_spell != 0
            && effect
                .radius
                .iter()
                .flatten()
                .any(|radius| radius.radius > 0.0 || radius.radius_max > 0.0);
    }
    crate::spell_cast::effects::is_handled(effect)
        || crate::spell_cast::toy_scripts::supports_effect(spell_id, effect)
}

pub(crate) fn unavailable(toy: &Toy, world: &World) -> Option<String> {
    if let Some(reason) = &toy.snapshot.unavailable_reason {
        return Some(reason.clone());
    }
    if toy.condition_id != 0 {
        return Some(format!(
            "toy PlayerCondition {} not supported",
            toy.condition_id
        ));
    }
    let types = unsupported_types(world, toy.snapshot.spell_id?);
    (!types.is_empty()).then(|| {
        format!(
            "unsupported toy spell: {}",
            types.into_iter().collect::<Vec<_>>().join(", ")
        )
    })
}

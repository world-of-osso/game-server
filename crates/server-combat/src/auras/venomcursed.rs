//! SimC76f8118 unique_gear_midnight.cpp5600–5614/6600: armor stat procs.
//! Native equipped drivers own eligibility, rates, item scaling and both signed values.
use bevy::prelude::*;

use super::{
    AuraApplication, AuraInstance, AuraSource, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};
use crate::random::{self, Rng};
use crate::spell_info::{SpellInfo, SpellStore};

const DRIVERS: &[u32] = &[1307906, 1307923, 1307928];
const EMPOWERING_VENOM: u32 = 1293316;
const DEBILITATING_VENOM: u32 = 1295123;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        EMPOWERING_VENOM,
        AuraScript {
            on_remove: Some(expire_empowering_venom),
            ..Default::default()
        },
    );
    for &id in DRIVERS {
        scripts.0.insert(
            id,
            AuraScript {
                on_proc: Some(apply_stat_proc),
                ..Default::default()
            },
        );
    }
    scripts.0.insert(
        1317582,
        AuraScript {
            on_proc: Some(apply_random_stat_proc),
            ..Default::default()
        },
    );
}

// INFERRED: 70448 Spell.csv describes one random secondary stat; each tooltip
// conditional identifies one effect. WM76 (70688) found no DB2 selection flag.
// Upgrade or retire on server or live-tooltip evidence. SimC76f8118
// unique_gear_midnight.cpp3564–3606 independently rerolls each aura.
pub(super) fn select_random_effect_mask(
    application: &AuraApplication,
    spell: &SpellInfo,
) -> Option<u32> {
    if !matches!(spell.id, EMPOWERING_VENOM | DEBILITATING_VENOM) {
        return None;
    }
    let mut candidates: Vec<_> = spell
        .effects
        .iter()
        .filter(|effect| {
            effect.effect == 6
                && effect.aura == super::handlers::MOD_RATING
                && application.effect_mask & (1 << effect.index) != 0
        })
        .map(|effect| effect.index)
        .collect();
    candidates.sort_unstable();
    assert!(
        !candidates.is_empty(),
        "Venom requires a native rating effect"
    );
    let index = candidates[random::thread_rng().gen_range(0..candidates.len())];
    Some(1 << index)
}

fn expire_empowering_venom(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    if reason != RemoveReason::Expired {
        return;
    }
    let store = world.resource::<SpellStore>();
    let parent = store
        .get(EMPOWERING_VENOM)
        .expect("native Empowering Venom");
    let child = parent
        .effects
        .iter()
        .find(|effect| effect.index == 5)
        .expect("native Empowering Venom expiry trigger")
        .trigger_spell;
    let mut debuff = store.get(child).expect("native Debilitating Venom").clone();
    debuff.cast_item_level = aura.cast_item_level;
    debuff.cast_item_inventory_type = aura.cast_item_inventory_type;
    let effect_mask = super::aura_effect_mask(&debuff);
    super::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: child,
            effect_mask,
            source: AuraSource::Cast,
        },
        &debuff,
    );
}

fn build_stat_buff(world: &World, driver: &AuraInstance) -> SpellInfo {
    let positive = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Venomcursed driver retains its positive stat effect");
    let mut buff = world
        .resource::<SpellStore>()
        .get(positive.trigger_spell)
        .expect("native Venomcursed stat buff")
        .clone();
    buff.cast_item_level = driver.cast_item_level;
    buff.cast_item_inventory_type = driver.cast_item_inventory_type;
    for index in 0..2 {
        let amount = driver
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native Venomcursed driver retains both signed stat amounts")
            .amount;
        buff.effects
            .iter_mut()
            .find(|effect| effect.index == index)
            .expect("native Venomcursed buff retains both rating masks")
            .base_points = amount;
    }
    buff
}

fn apply_random_stat_proc(
    world: &mut World,
    unit: Entity,
    driver: &AuraInstance,
    _other: Option<Entity>,
) {
    // SimC76f8118 unique_gear_midnight.cpp5618–5670: reroll on every proc, including refreshes.
    let positive = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let negative = driver
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap();
    let mut buff = world
        .resource::<SpellStore>()
        .get(positive.trigger_spell)
        .unwrap()
        .clone();
    buff.cast_item_level = driver.cast_item_level;
    buff.cast_item_inventory_type = driver.cast_item_inventory_type;
    buff.effects.sort_by_key(|effect| effect.index);
    let selected = random::thread_rng().gen_range(0..buff.effects.len());
    for (index, effect) in buff.effects.iter_mut().enumerate() {
        effect.base_points = if index == selected {
            positive.amount
        } else {
            negative.amount
        };
    }
    let effect_mask = super::aura_effect_mask(&buff);
    super::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: buff.id,
            effect_mask,
            source: AuraSource::Cast,
        },
        &buff,
    );
}

fn apply_stat_proc(world: &mut World, unit: Entity, driver: &AuraInstance, _other: Option<Entity>) {
    let buff = build_stat_buff(world, driver);
    super::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: buff.id,
            effect_mask: 3,
            source: AuraSource::Cast,
        },
        &buff,
    );
}

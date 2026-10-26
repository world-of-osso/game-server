//! SimC76f8118 unique_gear_midnight.cpp614–671/995–1122: native stat-child dispatch.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs::AuraEvent;
use combat::auras::scripts::{AuraScripts, ProcHook};
use combat::auras::{self, AuraApplication, AuraInstance, AuraSource};
use combat::random::{self, Rng};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;
use combat::stats::{EquippedItemModifiers, NativeItemInstances};
use std::collections::BTreeSet;

const LOA: u32 = 1251904;
const RITE: u32 = 1297654;
const CAPYBARA: u32 = 1252524;
const AKILZON: u32 = 1252818;
const PERIDOT: u32 = 14262;
const LOA_COLORS: &[u32] = &[PERIDOT, 14276, 14277, 14278];
const DUMMY_AURA: u32 = 4;
const RATING_AURA: u32 = 189;
const RITE_FAVORS_HIGHEST: f32 = 0.6; // Pinned player.hpp827; not a stat amount.

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];
const CALLBACKS: &[(u32, ProcHook)] = &[(LOA, proc_loa), (RITE, proc_rite)];

fn register(scripts: &mut AuraScripts) {
    for &(id, callback) in CALLBACKS {
        let script = scripts.0.entry(id).or_default();
        script.on_proc = Some(callback);
        script.check_proc = Some(item_callback_in_combat);
    }
}

fn item_callback_in_combat(world: &World, aura: &AuraInstance, _: &AuraEvent) -> bool {
    // action_callback.cpp79–82: the linked Cast-source Rite driver is still an item callback.
    combat::combat_refs::active_opponents(world, aura.caster)
        .next()
        .is_some()
}

fn read_amount(driver: &AuraInstance, index: u8) -> f32 {
    driver
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("native item proc retains its amount effect")
        .amount
}

fn apply_stat_buff(
    world: &mut World,
    unit: Entity,
    driver: &AuraInstance,
    id: u32,
    amount: Option<f32>,
) {
    let mut buff = world
        .resource::<SpellStore>()
        .get(id)
        .expect("native item stat child")
        .clone();
    buff.cast_item_level = driver.cast_item_level;
    if let Some(amount) = amount {
        buff.effects
            .iter_mut()
            .find(|effect| effect.index == 0)
            .expect("native item stat child retains its amount effect")
            .base_points = amount;
    }
    let effect_mask = auras::aura_effect_mask(&buff);
    auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: id,
            effect_mask,
            source: AuraSource::Cast,
        },
        &buff,
    );
}

#[derive(Clone, Copy)]
enum LoaChoice {
    Primary,
    Peridot,
    Unqualified(u32),
}

fn read_loa_choices(world: &World, unit: Entity) -> Vec<LoaChoice> {
    let equipped = world
        .get::<EquippedItemModifiers>(unit)
        .expect("native equipped gem instances");
    let native = world.resource::<NativeItemInstances>();
    let colors: BTreeSet<_> = equipped
        .items
        .iter()
        .flat_map(|(_, item)| item.gems.iter())
        .map(|id| {
            *native
                .enchants
                .gem_descriptions
                .get(id)
                .expect("equipped gem has native ItemNameDescriptionID")
        })
        .collect();
    std::iter::once(LoaChoice::Primary)
        .chain(
            LOA_COLORS
                .iter()
                .copied()
                .filter(|id| colors.contains(id))
                .map(|id| {
                    if id == PERIDOT {
                        LoaChoice::Peridot
                    } else {
                        LoaChoice::Unqualified(id)
                    }
                }),
        )
        .collect()
}

fn proc_loa(world: &mut World, unit: Entity, driver: &AuraInstance, _: Option<Entity>) {
    let choices = read_loa_choices(world, unit);
    let index = random::thread_rng().gen_range(0..choices.len());
    match choices[index] {
        LoaChoice::Primary => {
            apply_stat_buff(world, unit, driver, CAPYBARA, Some(read_amount(driver, 1)))
        }
        LoaChoice::Peridot => apply_stat_buff(world, unit, driver, AKILZON, None),
        // Ret equips only Peridot. Preserve other colors' selection weight, but their
        // non-qualified branches are outside this stat-dispatch slice (including268).
        LoaChoice::Unqualified(description) => warn!(
            description,
            "Loa gem branch is outside qualified stat dispatch"
        ),
    }
}

fn read_rite_children(world: &World, driver: &AuraInstance) -> Vec<u32> {
    world
        .resource::<SpellStore>()
        .get(driver.spell_id)
        .expect("native Rite driver")
        .effects
        .iter()
        .filter(|effect| effect.aura == DUMMY_AURA && effect.trigger_spell != 0)
        .map(|effect| effect.trigger_spell)
        .collect()
}

fn read_rating_mask(world: &World, id: u32) -> i32 {
    world
        .resource::<SpellStore>()
        .get(id)
        .expect("native Rite stat child")
        .effects
        .iter()
        .find(|effect| effect.aura == RATING_AURA)
        .expect("native Rite child owns its rating mask")
        .misc_values[0]
}

fn roll_rite_child(world: &World, unit: Entity, driver: &AuraInstance) -> u32 {
    let children = read_rite_children(world, driver);
    assert!(!children.is_empty(), "native Rite has no rating children");
    // player.hpp827 / unique_gear_midnight.cpp638–643: pinned default favors
    // the current highest secondary60%; otherwise choose uniformly. Not a stat value.
    if random::random::<f32>() >= RITE_FAVORS_HIGHEST {
        return children[random::thread_rng().gen_range(0..children.len())];
    }
    let highest = combat::consumables::read_secondary_ratings(world, unit)
        .into_iter()
        .reduce(|best, candidate| {
            if candidate.1 > best.1 {
                candidate
            } else {
                best
            }
        })
        .expect("four native secondary ratings")
        .0;
    children
        .into_iter()
        .find(|&id| read_rating_mask(world, id) & highest != 0)
        .expect("native Rite retains the highest-secondary child")
}

fn proc_rite(world: &mut World, unit: Entity, driver: &AuraInstance, _: Option<Entity>) {
    let child = roll_rite_child(world, unit, driver);
    // Native285 already carries the equipped rank's scaled amount into linked BP0;
    // both weapons share this one Cast-source driver, matching SimC callback dedup.
    apply_stat_buff(world, unit, driver, child, Some(read_amount(driver, 0)));
}

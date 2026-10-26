//! SimC76f8118 unique_gear_midnight.cpp614–671/995–1122/1385–1428: native stat-child dispatch.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs::AuraEvent;
use combat::auras::scripts::{AuraScripts, ProcHook};
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use combat::random::{self, Rng};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;
use combat::stats::{EquippedItemModifiers, NativeItemInstances};
use std::collections::BTreeSet;

const LOA: u32 = 1251904;
const RITE: u32 = 1297654;
const RITUAL_STONE: u32 = 1295897;
// Pinned cpp1395/1421–1423 supplies this behavior; native DUMMY has Trigger0.
const RITUAL_CHILDREN: &[u32] = &[1295898, 1295899, 1295900, 1295901];
const CAPYBARA: u32 = 1252524;
const AKILZON: u32 = 1252818;
const PERIDOT: u32 = 14262;
const LOA_COLORS: &[u32] = &[PERIDOT, 14276, 14277, 14278];
const DUMMY_AURA: u32 = 4;
const RATING_AURA: u32 = 189;
const RITE_FAVORS_HIGHEST: f32 = 0.6; // Pinned player.hpp827; not a stat amount.

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];
const CALLBACKS: &[(u32, ProcHook)] = &[
    (LOA, proc_loa),
    (RITE, proc_rite),
    (RITUAL_STONE, proc_ritual_stone),
];

fn register(scripts: &mut AuraScripts) {
    for &(id, callback) in CALLBACKS {
        let script = scripts.0.entry(id).or_default();
        script.on_proc = Some(callback);
        script.check_proc = Some(item_callback_in_combat);
    }
}

fn item_callback_in_combat(world: &World, aura: &AuraInstance, _: &AuraEvent) -> bool {
    // action_callback.cpp79–82: linked Cast-source drivers are still item callbacks.
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
    source: &AuraInstance,
    id: u32,
    amount: Option<f32>,
) {
    let mut buff = world
        .resource::<SpellStore>()
        .get(id)
        .expect("native item stat child")
        .clone();
    buff.cast_item_level = source.cast_item_level;
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

fn read_gem_descriptions(world: &World, unit: Entity) -> BTreeSet<u32> {
    let equipped = world
        .get::<EquippedItemModifiers>(unit)
        .expect("native equipped gem instances");
    let native = world.resource::<NativeItemInstances>();
    equipped
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
        .collect()
}

fn read_loa_choices(world: &World, unit: Entity) -> Vec<LoaChoice> {
    let colors = read_gem_descriptions(world, unit);
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

fn find_linked_equipment_root(world: &World, unit: Entity, proc_spell: u32) -> AuraInstance {
    world
        .get::<ActiveAuras>(unit)
        .expect("native equipped item auras")
        .iter()
        .find(|root| {
            root.caster == unit
                && matches!(root.source, AuraSource::Equipment(_))
                && root
                    .effects
                    .iter()
                    .any(|effect| effect.aura_type == 285 && effect.trigger_spell == proc_spell)
        })
        .expect("native stat proc retains its linked equipment root")
        .clone()
}

fn read_ritual_amount(world: &World, unit: Entity, root: &AuraInstance) -> f32 {
    // cpp1392: the equipped ROOT's second effect, not the linked DUMMY BP0.
    let mut native = world
        .resource::<SpellStore>()
        .get(root.spell_id)
        .expect("native Ritual Stone root spell")
        .clone();
    native.cast_item_level = Some(root.cast_item_level.expect("native equipped item level"));
    let effect = native
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Ritual Stone root retains its stat amount effect");
    combat::spell_cast::effects::modded_base_points(world, unit, &native, effect)
}

fn proc_ritual_stone(world: &mut World, unit: Entity, driver: &AuraInstance, _: Option<Entity>) {
    let root = find_linked_equipment_root(world, unit, driver.spell_id);
    let amount = read_ritual_amount(world, unit, &root);
    // cpp1408–1428: one linked native callback, uniform among all four buffs.
    let child = RITUAL_CHILDREN[random::thread_rng().gen_range(0..RITUAL_CHILDREN.len())];
    apply_stat_buff(world, unit, &root, child, Some(amount));
}

fn proc_rite(world: &mut World, unit: Entity, driver: &AuraInstance, _: Option<Entity>) {
    let child = roll_rite_child(world, unit, driver);
    // Native285 already carries the equipped rank's scaled amount into linked BP0;
    // both weapons share this one Cast-source driver, matching SimC callback dedup.
    apply_stat_buff(world, unit, driver, child, Some(read_amount(driver, 0)));
}

//! TC a352b1fa Item.cpp76–242: apply/remove the sole ItemProcInputs catalog's bonuses.
use super::*;
use crate::class_progression::{ActiveSpec, TraitLoadout};
use crate::profession::PlayerProfessions;

type ChangedSetOwners = QueryState<
    Entity,
    (
        With<EquippedItems>,
        With<EquippedItemModifiers>,
        Or<(
            Changed<EquippedItems>,
            Changed<ActiveSpec>,
            Changed<TraitLoadout>,
            Changed<PlayerProfessions>,
        )>,
    ),
>;

pub(crate) fn sync_item_sets(world: &mut World, changed: &mut ChangedSetOwners) {
    let owners: Vec<_> = changed.iter(world).collect();
    for unit in owners {
        let desired = read_set_spells(world, unit);
        crate::auras::remove_auras(world, unit, RemoveReason::Unlearned, |aura| {
            let AuraSource::ItemSet(set) = aura.source else {
                return false;
            };
            !desired
                .iter()
                .any(|(id, spell)| *id == set && spell.id == aura.spell_id)
        });
        for (set, spell) in desired {
            super::apply_missing_equipment_aura(world, unit, AuraSource::ItemSet(set), &spell);
        }
    }
}

pub(crate) fn read_set_spells(world: &World, unit: Entity) -> Vec<(u32, SpellInfo)> {
    let equipped = world
        .get::<EquippedItems>(unit)
        .expect("set owner equipment");
    if equipped.items.is_empty() {
        return Vec::new();
    }
    let counts = crate::item_proc_data::equipped_counts(world, unit)
        .unwrap_or_else(|error| panic!("native set owner{unit:?}: {error}"));
    if counts.is_empty() {
        return Vec::new();
    }
    world
        .resource::<crate::item_proc_data::ItemProcInputs>()
        .active_set_spells(world, unit, &counts)
        .unwrap_or_else(|error| panic!("native set owner{unit:?}: {error}"))
        .into_iter()
        .map(|(set, spell)| (set, read_set_spell(world, spell)))
        .collect()
}

fn read_set_spell(world: &World, id: u32) -> SpellInfo {
    world
        .resource::<SpellStore>()
        .get(id)
        .cloned()
        .unwrap_or_else(|| panic!("native ItemSetSpell{id} absent"))
}

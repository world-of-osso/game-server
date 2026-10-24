//! Profile-selected summon_pet resolves a learned native spell and its native entry.
use super::super::inputs::Profile;
use super::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use bevy::prelude::*;
use shared::components::{Health, Npc};

pub(in super::super) struct ConfiguredPet {
    spell: u32,
    entry: u32,
}

impl ConfiguredPet {
    pub fn read(world: &World, actor: Entity, profile: &Profile) -> Result<Option<Self>, String> {
        let class = profile
            .options
            .get("class")
            .and_then(serde_json::Value::as_str)
            .ok_or("reference profile lacks its class namespace")?;
        let key = format!("{class}.default_pet");
        let Some(value) = profile.options.get(&key) else {
            return Ok(None);
        };
        let name = value
            .as_str()
            .ok_or_else(|| format!("reference {key} must be a pet name"))?;
        read_known_summon(world, actor, name).map(Some)
    }

    pub fn bind(&self, bindings: &mut Bindings) -> Result<(), String> {
        if bindings
            .actions
            .get("summon_pet")
            .is_some_and(|&id| id != self.spell)
        {
            return Err("configured default pet conflicts with summon_pet native binding".into());
        }
        bindings.actions.insert("summon_pet".into(), self.spell);
        Ok(())
    }

    pub fn read_action(
        &self,
        world: &World,
        actor: Entity,
    ) -> Result<Option<NativeAction<'static>>, String> {
        if let Some(pet) = crate::hunter_pet::summoned_pet(world, actor) {
            let npc = world
                .get::<Npc>(pet)
                .ok_or("native current pet lacks Npc identity")?;
            let health = world
                .get::<Health>(pet)
                .ok_or("native current pet lacks Health")?;
            if npc.template_id == self.entry && health.current > 0.0 {
                return Ok(None);
            }
        }
        Ok(Some(NativeAction {
            name: "summon_pet",
            spell: self.spell,
            talent: None,
            self_target: true,
        }))
    }
}

fn read_known_summon(world: &World, actor: Entity, name: &str) -> Result<ConfiguredPet, String> {
    let expected = format!("Summon {name}");
    let known = world
        .get::<crate::class_progression::KnownSpells>(actor)
        .ok_or("default-pet actor lacks its native spellbook")?;
    let store = world.resource::<crate::spell_info::SpellStore>();
    let spells: Vec<_> = known
        .iter()
        .filter_map(|id| store.get(id))
        .filter(|spell| spell.name.eq_ignore_ascii_case(&expected))
        .collect();
    let [spell] = spells.as_slice() else {
        return Err(format!(
            "default_pet {name:?} resolves {} learned native summons",
            spells.len()
        ));
    };
    let entries: Vec<_> = spell
        .effects
        .iter()
        .filter(|effect| effect.effect == crate::hunter_pet::EFFECT_SUMMON_PET)
        .map(|effect| effect.misc_values[0])
        .collect();
    let [entry] = entries.as_slice() else {
        return Err(format!(
            "default_pet spell{} lacks one native SUMMON_PET entry",
            spell.id
        ));
    };
    let entry = u32::try_from(*entry)
        .ok()
        .filter(|&entry| entry != 0)
        .ok_or_else(|| format!("default_pet spell{} has no native creature entry", spell.id))?;
    Ok(ConfiguredPet {
        spell: spell.id,
        entry,
    })
}

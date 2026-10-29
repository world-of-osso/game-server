//! Soulcoiler Ritual Vessel: native item channel and fractional barriers.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use combat::random::seq::SliceRandom;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;
use shared::components::{Health, Player};

const EQUIPPED: u32 = 1291885;
const CHANNEL: u32 = 1291894;
const BARRIER: u32 = 1307578;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(CHANNEL).or_default().on_periodic = Some(grant_barrier);
}

fn grant_barrier(world: &mut World, owner: Entity, aura: &AuraInstance, index: u8) {
    if aura.caster != owner || index != 0 {
        return;
    }
    let Some(target) = select_recipient(world, owner) else {
        return;
    };
    let root = world
        .get::<ActiveAuras>(owner)
        .and_then(|auras| {
            auras.iter().find(|aura| {
                aura.spell_id == EQUIPPED
                    && aura.caster == owner
                    && matches!(aura.source, AuraSource::Equipment(_))
            })
        })
        .expect("Soulcoiler channel requires its equipped item");
    let amount = root
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Soulcoiler item amount")
        .amount;
    let item_level = root.cast_item_level;
    // 1291894's 70448 role variable reduces non-healer barriers to 66%.
    let non_healer = [
        137008, 137006, 137007, 212612, 212613, 137013, 137011, 137010, 396186, 356809, 137015,
        137016, 137017, 137021, 137019, 137020, 137023, 137025, 137028, 137027, 137033, 137037,
        137036, 137035, 137040, 137041, 137043, 137044, 137046, 137049, 137050, 137048,
    ]
    .iter()
    .any(|id| auras::has_aura(world, owner, *id));
    let mut spell = world
        .resource::<SpellStore>()
        .get(BARRIER)
        .expect("Soulcoil Barrier")
        .clone();
    spell.cast_item_level = item_level;
    spell.cast_effect0_amount = Some(amount * if non_healer { 0.66 } else { 1.0 });
    auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: owner,
            target,
            spell_id: BARRIER,
            effect_mask: auras::aura_effect_mask(&spell),
            source: AuraSource::Cast,
        },
        &spell,
    );
}

fn select_recipient(world: &mut World, owner: Entity) -> Option<Entity> {
    let map = *world.get::<combat::maps::InMap>(owner)?;
    let mut candidates: Vec<_> = world
        .query::<(
            Entity,
            &Health,
            &combat::maps::InMap,
            Option<&Player>,
            Option<&combat::followers::Follower>,
        )>()
        .iter(world)
        .filter(|(_, health, unit_map, player, follower)| {
            health.current > 0.0 && **unit_map == map && (player.is_some() || follower.is_some())
        })
        .map(|(unit, _, _, _, _)| unit)
        .filter(|unit| {
            combat::spell_cast::targets::relation_matches(
                world,
                owner,
                *unit,
                combat::spell_cast::targets::Relation::Ally,
            )
        })
        .collect();
    candidates.shuffle(&mut combat::random::thread_rng());
    candidates
        .iter()
        .copied()
        .find(|unit| {
            !world.get::<ActiveAuras>(*unit).is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == BARRIER && aura.caster == owner)
            })
        })
        .or_else(|| candidates.last().copied())
}

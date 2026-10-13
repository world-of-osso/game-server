//! TC a352b1fa Player.cpp:3400–3418 / SpellHistory.h:136–153.
//! Reset authored spell/category durations below10min, except attr6 bit0x2.
//! ResetCooldowns does NOT reset charges, GCDs or school lockouts.
use super::*;

const ARENA_RESET_LIMIT_MS: u32 = 600_000;
const DO_NOT_RESET_IN_ARENA: u32 = 0x2;

pub(crate) fn reset_arena_cooldowns(world: &mut World, player: Entity) {
    clear_cooldowns(world, player, ResetScope::Arena);
    let pet = world
        .get::<crate::hunter_pet::PetStable>(player)
        .and_then(|stable| stable.summoned);
    if let Some(pet) = pet {
        clear_cooldowns(world, pet, ResetScope::All);
    }
}

enum ResetScope {
    Arena,
    All,
}

fn eligible_for_arena(spell: &SpellInfo, item: Option<&ItemCooldownSource>) -> bool {
    let (recovery_ms, category_ms) = match item {
        Some(item) => (item.recovery_ms, item.category_ms),
        None => spell.cooldowns.as_deref().map_or((0, 0), |source| {
            (source.recovery_ms, source.category_recovery_ms)
        }),
    };
    let excluded = spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.attributes[6] & DO_NOT_RESET_IN_ARENA != 0);
    recovery_ms < ARENA_RESET_LIMIT_MS && category_ms < ARENA_RESET_LIMIT_MS && !excluded
}

fn find_reset_cooldowns(world: &World, unit: Entity, scope: ResetScope) -> Option<Vec<(u32, u32)>> {
    let history = world.get::<SpellHistory>(unit)?;
    let store = world.resource::<SpellStore>();
    Some(
        history
            .spell_ready_at
            .keys()
            .filter_map(|id| {
                let spell = store
                    .get(*id)
                    .expect("tracked cooldown must have source spell metadata");
                let item = history.item_cooldown_sources.get(id);
                let eligible = matches!(scope, ResetScope::All) || eligible_for_arena(spell, item);
                let category = match item {
                    Some(item) => item.category,
                    None => spell
                        .categories
                        .as_ref()
                        .map_or(0, |category| category.category),
                };
                eligible.then_some((*id, category))
            })
            .collect(),
    )
}

fn clear_cooldowns(world: &mut World, unit: Entity, scope: ResetScope) {
    let Some(cleared) = find_reset_cooldowns(world, unit, scope) else {
        return;
    };
    let mut history = world
        .get_mut::<SpellHistory>(unit)
        .expect("cooldown history");
    for &(id, category) in &cleared {
        history.spell_ready_at.remove(&id);
        history.category_ready_at.remove(&category);
        history.item_cooldown_sources.remove(&id);
    }
    for (spell_id, category) in cleared {
        send_to_owner(
            world,
            unit,
            SpellCooldownUpdate {
                spell_id,
                category,
                duration_ms: 0,
                remaining_ms: 0,
                is_gcd: false,
            },
        );
    }
}

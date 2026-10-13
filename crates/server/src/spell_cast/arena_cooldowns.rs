//! TC a352b1fa Player.cpp:3400–3418 / SpellHistory.h:136–153.
//! Reset authored spell/category durations below10min, except attr6 bit0x2.
//! ResetCooldowns does NOT reset charges, GCDs or school lockouts.
use super::*;

const ARENA_RESET_LIMIT_MS: u32 = 600_000;
const DO_NOT_RESET_IN_ARENA: u32 = 0x2;

pub(crate) fn reset_arena_cooldowns(world: &mut World, player: Entity) {
    clear_cooldowns(world, player, eligible_for_arena);
    let pet = world
        .get::<crate::hunter_pet::PetStable>(player)
        .and_then(|stable| stable.summoned);
    if let Some(pet) = pet {
        clear_cooldowns(world, pet, |_| true);
    }
}

fn eligible_for_arena(spell: &SpellInfo) -> bool {
    let durations = spell.cooldowns.as_deref().cloned().unwrap_or_default();
    let excluded = spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.attributes[6] & DO_NOT_RESET_IN_ARENA != 0);
    durations.recovery_ms < ARENA_RESET_LIMIT_MS
        && durations.category_recovery_ms < ARENA_RESET_LIMIT_MS
        && !excluded
}

fn clear_cooldowns(world: &mut World, unit: Entity, eligible: fn(&SpellInfo) -> bool) {
    let Some(history) = world.get::<SpellHistory>(unit) else {
        return;
    };
    let store = world.resource::<SpellStore>();
    let cleared: Vec<_> = history
        .spell_ready_at
        .keys()
        .filter_map(|id| {
            let spell = store
                .get(*id)
                .expect("tracked cooldown must have source spell metadata");
            eligible(spell).then_some((*id, spell.categories.as_ref().map_or(0, |c| c.category)))
        })
        .collect();
    let mut history = world
        .get_mut::<SpellHistory>(unit)
        .expect("cooldown history");
    for &(id, category) in &cleared {
        history.spell_ready_at.remove(&id);
        history.category_ready_at.remove(&category);
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

use super::*;
use crate::spell_info::{SpellCategories, SpellCooldownInfo, SpellMisc};

fn cooldown_spell(id: u32, recovery: u32, category_recovery: u32, excluded: bool) -> SpellInfo {
    SpellInfo {
        id,
        misc: Some(SpellMisc {
            attributes: {
                let mut flags = [0; 17];
                if excluded {
                    flags[6] = 0x2;
                }
                flags
            },
            ..Default::default()
        }),
        cooldowns: Some(Box::new(SpellCooldownInfo {
            recovery_ms: recovery,
            category_recovery_ms: category_recovery,
            ..Default::default()
        })),
        categories: Some(Box::new(SpellCategories {
            category: id,
            ..Default::default()
        })),
        ..Default::default()
    }
}

#[test]
fn arena_item_category_reset_uses_item_duration_and_sends_native_clear() {
    for duration in [120_000, 600_000] {
        let mut h = super::test_harness::harness(Vec::new(), &[498], 2).unwrap();
        cast_item_spell(
            h.app.world_mut(),
            h.caster,
            498,
            Some(h.caster),
            ItemCooldown {
                category: 9999,
                category_ms: duration,
            },
            90,
            None,
        )
        .unwrap();
        h.advance(1);
        assert!(
            h.app
                .world()
                .get::<SpellHistory>(h.caster)
                .unwrap()
                .category_ready_at[&9999]
                > 0.0
        );
        h.inbox.cooldowns.clear();
        reset_arena_cooldowns(h.app.world_mut(), h.caster);
        h.advance(1);
        let history = h.app.world().get::<SpellHistory>(h.caster).unwrap();
        if duration < 600_000 {
            assert!(
                !history.category_ready_at.contains_key(&9999),
                "item-use category not present in source spell metadata must clear"
            );
            assert!(h.inbox.cooldowns.iter().any(|update| update.spell_id == 498
                && update.category == 9999
                && update.remaining_ms == 0));
        } else {
            assert!(
                history.category_ready_at.contains_key(&9999),
                "authored10min item category must remain even on short source spell"
            );
        }
    }
}

#[test]
fn arena_item_cast_time_category_survives_completion_then_resets() {
    let mut h = super::test_harness::harness(Vec::new(), &[133], 8).unwrap();
    cast_item_spell(
        h.app.world_mut(),
        h.caster,
        133,
        Some(h.npc),
        ItemCooldown {
            category: 9999,
            category_ms: 120_000,
        },
        90,
        None,
    )
    .unwrap();
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h.advance(50);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(
        h.health(h.npc) < 5000.0,
        "real cast-time damage must complete"
    );
    let history = h.app.world().get::<SpellHistory>(h.caster).unwrap();
    assert!(history.category_ready_at[&9999] > history_now(h.app.world()));
    h.inbox.cooldowns.clear();
    reset_arena_cooldowns(h.app.world_mut(), h.caster);
    h.advance(1);
    assert!(
        !h.app
            .world()
            .get::<SpellHistory>(h.caster)
            .unwrap()
            .category_ready_at
            .contains_key(&9999)
    );
    assert!(h.inbox.cooldowns.iter().any(|update| update.spell_id == 133
        && update.category == 9999
        && update.remaining_ms == 0));
}

#[test]
fn arena_entry_cooldown_reset_uses_authored_durations_and_preserves_charges_gcd_and_exclusions() {
    let spells = vec![
        cooldown_spell(1, 599_999, 0, false),
        cooldown_spell(2, 600_000, 0, false),
        cooldown_spell(3, 10_000, 600_000, false),
        cooldown_spell(4, 10_000, 0, true),
    ];
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.insert_resource(SpellStore::fixture(spells.clone()));
    let mut history = SpellHistory::default();
    for spell in &spells {
        history.spell_ready_at.insert(spell.id, 5.0);
        history.category_ready_at.insert(spell.id, 5.0);
    }
    history.gcd_ready_at.insert(133, 5.0);
    history.school_locked_until[2] = 5.0;
    history.charges.insert(
        10,
        ChargeState {
            spell_id: 1,
            category: 10,
            max: 2,
            recovery_ms: 15_000.0,
            recovery_mult: 1.0,
            pending: std::collections::VecDeque::from([super::ChargeRecovery {
                recovery_ms: 15_000.0,
                ready_at: 15.0,
            }]),
            fully_ready_at: 15.0,
        },
    );
    let actor = world.spawn(history).id();
    reset_arena_cooldowns(&mut world, actor);
    let history = world.get::<SpellHistory>(actor).unwrap();
    assert!(!history.on_cooldown(&spells[0], 0.0));
    for spell in &spells[1..] {
        assert!(
            history.on_cooldown(spell, 0.0),
            "authored long/excluded spell {} must remain even with only5s left",
            spell.id
        );
    }
    assert_eq!(
        history.charges[&10].used(),
        1,
        "TC cooldown reset does not reset charges"
    );
    assert_eq!(history.gcd_ready_at[&133], 5.0);
    assert_eq!(history.school_locked_until[2], 5.0);
}

#[test]
fn arena_entry_preserves_long_category_while_resetting_item_override() {
    let mut item_spell = cooldown_spell(1, 120_000, 120_000, false);
    item_spell.categories.as_mut().unwrap().category = 2;
    let long_spell = cooldown_spell(2, 600_000, 600_000, false);
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.insert_resource(SpellStore::fixture(vec![item_spell, long_spell.clone()]));
    let mut history = SpellHistory::default();
    history.spell_ready_at.insert(1, 120.0);
    history.spell_ready_at.insert(2, 600.0);
    history.category_ready_at.insert(2, 600.0);
    history.category_ready_at.insert(9999, 120.0);
    history.item_cooldown_sources.insert(
        1,
        ItemCooldownSource {
            recovery_ms: 120_000,
            category_ms: 120_000,
            category: 9999,
        },
    );
    let player = world.spawn(history).id();
    reset_arena_cooldowns(&mut world, player);
    let history = world.get::<SpellHistory>(player).unwrap();
    assert!(!history.category_ready_at.contains_key(&9999));
    assert_eq!(
        history.category_ready_at[&2], 600.0,
        "item's unused base category must not clear another spell's long timer"
    );
    assert!(history.on_cooldown(&long_spell, 0.0));
}

#[test]
fn arena_entry_resets_active_pet_even_for_long_excluded_cooldowns() {
    let spell = cooldown_spell(1, 600_000, 600_000, true);
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.insert_resource(SpellStore::fixture(vec![spell.clone()]));
    let mut history = SpellHistory::default();
    history.spell_ready_at.insert(1, 600.0);
    history.category_ready_at.insert(1, 600.0);
    let pet = world.spawn(history).id();
    let player = world
        .spawn(crate::hunter_pet::PetStable {
            character_id: Some(1),
            stored: Default::default(),
            summoned: Some(pet),
        })
        .id();
    reset_arena_cooldowns(&mut world, player);
    let history = world.get::<SpellHistory>(pet).unwrap();
    assert!(!history.on_cooldown(&spell, 0.0));
    assert!(history.category_ready_at.is_empty());
}

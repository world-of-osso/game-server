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
    history.charges.insert(
        10,
        ChargeState {
            spell_id: 1,
            category: 10,
            max: 2,
            used: 1,
            recharge_secs: 15.0,
            next_ready_at: 15.0,
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
        history.charges[&10].used, 1,
        "TC cooldown reset does not reset charges"
    );
    assert_eq!(history.gcd_ready_at[&133], 5.0);
}

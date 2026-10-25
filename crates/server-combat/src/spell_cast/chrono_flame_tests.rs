//! Native431442 ->431443 override and declared Living-Flame/Chrono effect rows.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{ActiveSpec, Level, UnitLevel};
use shared::protocol::CombatLogKind;

fn chrono_harness(spec: u32, optional: &[u32]) -> Harness {
    let mut ids = vec![361469, 361500, 361509, 431442, 431443, 431483, 431583];
    ids.extend_from_slice(optional);
    let mut h = harness(Vec::new(), &ids, 13).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80), ActiveSpec(spec)));
    h.app.world_mut().entity_mut(h.npc).insert((
        Level(80),
        UnitLevel(80),
        shared::components::Health {
            current: 20_000.0,
            max: 20_000.0,
        },
    ));
    h.advance(3);
    h
}

fn seed_damage(h: &mut Harness, actor: Entity, target: Entity, amount: f32) {
    let spell = world_db().unwrap().store.get(431583).unwrap().clone();
    let ctx = super::effects::EffectCtx {
        caster: actor,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_copied_damage(h.app.world_mut(), &ctx, target, amount);
}

fn cast_repeat(h: &mut Harness, target: Entity) {
    h.advance(1);
    h.inbox = Inbox::default();
    h.cast_and_deliver(361469, Some(target));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

fn damage(h: &Harness, id: u32) -> i32 {
    h.inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(id))
        .unwrap_or_else(|| panic!("missing native damage {id}: {:?}", h.inbox.log))
        .amount
}

#[test]
fn class_completeness_native_chrono_flames_resolves_fire_and_arcane_repeat() {
    let mut h = chrono_harness(1467, &[]);
    h.cast_and_deliver(361469, Some(h.npc));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 431443));
    let fire = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(361500))
        .expect("native Chrono replacement must retain Living Flame's fire branch");
    let arcane = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(431583))
        .expect("native Chrono Flame repeat must land as the Arcane child");
    let percent = world_db()
        .unwrap()
        .store
        .get(431442)
        .unwrap()
        .effects
        .iter()
        .find(|e| e.index == 2)
        .unwrap()
        .base_points;
    assert_eq!(arcane.school_mask, 64);
    assert_eq!(arcane.target, Some(h.npc.to_bits()));
    assert_eq!(
        arcane.amount,
        (fire.amount as f32 * percent / 100.0).round() as i32
    );
}

#[test]
fn class_completeness_native_chrono_history_is_owned_and_target_specific() {
    let mut h = chrono_harness(1467, &[]);
    let other = spawn_player(&mut h.app, 13, 1.0);
    h.app
        .world_mut()
        .entity_mut(other)
        .insert(crate::class_progression::KnownSpells::fixture(vec![431442]));
    let other_target = spawn_npc(&mut h.app, 6.0);
    h.app.world_mut().entity_mut(other_target).insert((
        Level(80),
        UnitLevel(80),
        shared::components::Health {
            current: 20_000.0,
            max: 20_000.0,
        },
    ));
    h.advance(3);
    let caster = h.caster;
    let target = h.npc;
    seed_damage(&mut h, caster, target, 500.0);
    seed_damage(&mut h, other, target, 3000.0);
    seed_damage(&mut h, caster, other_target, 2000.0);
    cast_repeat(&mut h, target);
    assert_eq!(
        damage(&h, 431583),
        ((500.0 + damage(&h, 361500) as f32) * 0.25).round() as i32
    );
}

#[test]
fn class_completeness_native_chrono_history_expires_at_the_authored_window() {
    let mut h = chrono_harness(1467, &[]);
    let caster = h.caster;
    let target = h.npc;
    seed_damage(&mut h, caster, target, 2000.0);
    h.advance(120); // Six real seconds exceeds native field1's five seconds.
    cast_repeat(&mut h, target);
    assert_eq!(
        damage(&h, 431583),
        (damage(&h, 361500) as f32 * 0.25).round() as i32
    );
}

#[test]
fn class_completeness_native_chrono_caps_and_overclock_use_native_variables() {
    for (optional, expected) in [(vec![], 250), (vec![1260647], 350)] {
        let mut h = chrono_harness(1467, &optional);
        let caster = h.caster;
        let target = h.npc;
        seed_damage(&mut h, caster, target, 8000.0);
        cast_repeat(&mut h, target);
        // Native variable852 at100SP:2.5SP; Overclock40% yields3.5SP.
        assert_eq!(damage(&h, 431583), expected);
    }
}

#[test]
fn class_completeness_native_chrono_preservation_repeats_effective_healing_not_overheal() {
    let mut h = chrono_harness(1468, &[]);
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(shared::components::Health {
            current: 4990.0,
            max: 5000.0,
        });
    let spell = world_db().unwrap().store.get(431483).unwrap().clone();
    let ctx = super::effects::EffectCtx {
        caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::apply_fixed_heal(h.app.world_mut(), &ctx, caster, 1000.0); // Only10 effective.
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(caster)
        .unwrap()
        .current = 4000.0;
    cast_repeat(&mut h, caster);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(361509))
        .unwrap();
    let repeat = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(431483))
        .unwrap();
    assert_eq!(repeat.school_mask, 64);
    assert_eq!(
        repeat.amount,
        ((10 + heal.amount - heal.overflow) as f32 * 0.15).round() as i32
    );
}

#[test]
fn class_completeness_native_chrono_missing_child_rejects_before_payment() {
    let mut h = chrono_harness(1467, &[]);
    let ids = [361469, 361500, 361509, 431442, 431443, 431483];
    let spells = ids
        .into_iter()
        .map(|id| world_db().unwrap().store.get(id).unwrap().clone())
        .collect();
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(spells));
    let before = h.health(h.npc);
    let mana = h
        .app
        .world()
        .get::<shared::components::Mana>(h.caster)
        .unwrap()
        .current;
    h.cast_and_deliver(361469, Some(h.npc));
    h.advance(50);
    assert!(h.inbox.go.is_empty());
    assert_eq!(h.health(h.npc), before);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Mana>(h.caster)
            .unwrap()
            .current,
        mana
    );
    assert_eq!(
        h.inbox.failed[0].detail.as_deref(),
        Some("Chrono Flame missing native child 431583")
    );
}

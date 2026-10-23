//! Native label-point modifiers must recalculate existing learned passive amounts.
//! TC a352b1fa SpellAuraEffects.cpp1194–1249, SpellInfo.cpp CalcValue point mods.
use super::*;
use crate::class_progression::KnownSpells;
use crate::spell_cast::test_harness::{Harness, harness};

fn amounts(h: &Harness, spell: u32) -> [f32; 2] {
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .expect("actual learned passive aura");
    [0, 1].map(|index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .amount
    })
}

fn set_known(h: &mut Harness, spells: Vec<u32>) {
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(spells));
    h.advance(3);
}

fn late_modifier(class: u8, target: u32, modifier: u32, base: [f32; 2], boosted: [f32; 2]) {
    // Both unmodified native definitions are in the store; knowledge changes
    // apply/remove the real passives through ordinary sync_passive_auras.
    let mut h = harness(Vec::new(), &[target, modifier], class).unwrap();
    set_known(&mut h, vec![target]);
    assert_eq!(
        amounts(&h, target),
        base,
        "without modifier, native base points apply"
    );
    let original = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == target)
        .unwrap()
        .clone();
    set_known(&mut h, vec![target, modifier]);
    let store = h.app.world().resource::<crate::spell_info::SpellStore>();
    let native = store.get(target).unwrap();
    let calculated = [0, 1].map(|index| {
        crate::spell_cast::effects::modded_base_points(
            h.app.world(),
            h.caster,
            native,
            native
                .effects
                .iter()
                .find(|effect| effect.index == index)
                .unwrap(),
        )
    });
    assert_eq!(
        calculated, boosted,
        "native label matching/CalcValue must already see the actual modifier"
    );
    assert_eq!(
        amounts(&h, target),
        boosted,
        "late modifier must recalculate the existing passive, not leave its old snapshot"
    );
    let current = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == target)
        .unwrap();
    assert_eq!(
        (current.instance_id, current.expires_at, current.stacks),
        (original.instance_id, original.expires_at, original.stacks),
        "amount recalculation preserves aura lifetime and identity"
    );
    set_known(&mut h, vec![target]);
    assert_eq!(
        amounts(&h, target),
        base,
        "unlearning modifier must restore the native unmodified points"
    );
}

#[test]
#[ignore = "native label point recalculation RED before generic implementation"]
fn native_label_points_late_monk_modifier_recalculates_efficient_training() {
    // Native450989 effects0/1=20;1258122 effects15/16 add20 by label5146.
    late_modifier(10, 450989, 1258122, [20.0, 20.0], [40.0, 40.0]);
}

#[test]
#[ignore = "native non-Monk label point recalculation RED before generic implementation"]
fn native_label_points_late_paladin_modifier_recalculates_second_sunrise() {
    // Native431474 effects0/1=15/30;1258011 effects19/20 add5/20 by label4716.
    late_modifier(2, 431474, 1258011, [15.0, 30.0], [20.0, 50.0]);
}

#[test]
#[ignore = "native percent-label all-effect points RED"]
fn native_label_points_percent_warlock_modifier_recalculates_flames_of_xoroth() {
    // Native429657 effects0/1=4;137044 effect12 applies -1% by label7018/op8.
    late_modifier(9, 429657, 137044, [4.0, 4.0], [3.96, 3.96]);
}

#[test]
#[ignore = "native early modifier and removal order control"]
fn native_label_points_early_modifier_and_unlearn_restore_the_same_amounts() {
    let mut h = harness(Vec::new(), &[1258122, 450989], 10).unwrap();
    h.advance(3);
    assert_eq!(
        amounts(&h, 450989),
        [40.0, 40.0],
        "source-first applies native points at construction"
    );
    set_known(&mut h, vec![450989]);
    assert_eq!(
        amounts(&h, 450989),
        [20.0, 20.0],
        "source removal recalculates without reapplying target"
    );
}

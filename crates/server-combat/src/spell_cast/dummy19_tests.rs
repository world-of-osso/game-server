//! Source-bound Mainline counts, healing totals, dispel response and owned copies.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn fixture(ids: &[u32], class: u8) -> Harness {
    let spells = ids
        .iter()
        .map(|id| {
            let mut spell = world_db().unwrap().store.get(*id).unwrap().clone();
            for effect in &mut spell.effects {
                effect.variance = 0.0;
            }
            spell
        })
        .collect();
    harness(spells, &[], class).unwrap()
}
fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
fn execute(h: &mut Harness, id: u32, target: Entity) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, Some(target)).unwrap();
}
#[test]
fn dummy19_flurry_native_three_not_four_bolts() {
    let mut h = fixture(&[44614, 228596, 228354], 8);
    let npc = h.npc;
    execute(&mut h, 44614, npc);
    h.advance(80);
    let hits: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(228354) && e.kind == CombatLogKind::Damage)
        .collect();
    assert_eq!(hits.len(), 3);
}
#[test]
fn dummy19_mutilate_variable394_half_offhand() {
    let mut h = fixture(&[1329, 5374, 27576, SUNWELL_RADIANCE], 4);
    apply_native_enemy_dodge_reduction(&mut h);
    let npc = h.npc;
    execute(&mut h, 1329, npc);
    h.advance(4);
    let mh = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(5374) && e.kind == CombatLogKind::Damage)
        .unwrap();
    let oh = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(27576) && e.kind == CombatLogKind::Damage)
        .unwrap();
    // Native27576 is exclusive offhand: TC Unit.cpp6880–6891 selects OFF_ATTACK;
    // Unit.cpp9991–9995 halves its AP once. Tooltip variable394 describes that
    // result, not a second per-spell multiplier after native coefficient scaling.
    assert_eq!(mh.amount, 115);
    assert_eq!(oh.amount, 58);
}
fn dispersion_heals(rank: Option<u32>, ticks: usize, total: i32) {
    let mut h = fixture(&[47585, 288733, 453729], 5);
    let caster = h.caster;
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 100.0;
    if let Some(rank) = rank {
        apply(&mut h, caster, caster, rank);
    }
    apply(&mut h, caster, caster, 47585);
    h.advance(180);
    let heals: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(47585) && e.kind == CombatLogKind::Heal)
        .collect();
    assert_eq!(heals.len(), ticks);
    assert_eq!(heals.iter().map(|e| e.amount).sum::<i32>(), total);
}
#[test]
fn dummy19_dispersion_native_total_not_per_tick() {
    dispersion_heals(None, 7, 252);
}
#[test]
fn dummy19_intangibility_adds_native_total() {
    dispersion_heals(Some(288733), 7, 497);
}
#[test]
fn dummy19_altered_form_extends_healing_clock() {
    dispersion_heals(Some(453729), 9, 333);
}
#[test]
fn dummy19_vampiric_touch_real_dispel_horrifies_dispeller_only() {
    let mut h = fixture(&[34914, 87204, 527], 5);
    let caster = h.caster;
    let ally = spawn_player(&mut h.app, 5, 3.0);
    let enemy = h.npc;
    h.advance(1);
    apply(&mut h, enemy, ally, 34914);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(527)
        .unwrap()
        .clone();
    let ctx = super::effects::EffectCtx {
        caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let effect = spell.effects.iter().find(|e| e.effect == 38).unwrap();
    super::effects::handler(effect.effect).unwrap()(h.app.world_mut(), &ctx, effect, ally);
    assert!(!crate::auras::has_aura(h.app.world(), ally, 34914));
    assert!(crate::auras::has_aura(h.app.world(), caster, 87204));
    assert!(!crate::auras::has_aura(h.app.world(), ally, 87204));
    assert!(!crate::auras::has_aura(h.app.world(), enemy, 87204));
}
#[test]
fn dummy19_havoc_owned_direct_single_target_copy() {
    let mut h = fixture(&[80240, 686, 172, 146739, 133, 45438], 9);
    let caster = h.caster;
    let primary = h.npc;
    let marked = spawn_npc(&mut h.app, 4.0);
    h.advance(1);
    apply(&mut h, caster, marked, 80240);
    execute(&mut h, 686, primary);
    h.advance(80);
    let primary_hit = h
        .inbox
        .log
        .iter()
        .find(|e| {
            e.spell_id == Some(686)
                && e.target == Some(primary.to_bits())
                && e.kind == CombatLogKind::Damage
        })
        .unwrap();
    let copy = h
        .inbox
        .log
        .iter()
        .find(|e| {
            e.spell_id == Some(686)
                && e.target == Some(marked.to_bits())
                && e.kind == CombatLogKind::Damage
        })
        .expect("owned Havoc copy");
    assert_eq!(
        copy.amount,
        (primary_hit.amount as f32 * 0.5).round() as i32
    );
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(686) && e.kind == CombatLogKind::Damage)
            .count(),
        2
    );
}
#[test]
fn dummy19_ardent_defender_spell_fatal_hit_restores_native_health_once() {
    let mut h = fixture(&[31850, 133], 2);
    let caster = h.caster;
    let npc = h.npc;
    apply(&mut h, caster, caster, 31850);
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 1.0;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let ctx = super::effects::EffectCtx {
        caster: npc,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, caster, 100.0, false, false);
    assert_eq!(h.app.world().get::<Health>(caster).unwrap().current, 200.0);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 31850));
    super::effects::deal_damage(h.app.world_mut(), &ctx, caster, 1000.0, false, false);
    assert_eq!(h.app.world().get::<Health>(caster).unwrap().current, 0.0);
}

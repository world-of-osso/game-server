//! TC a352b1fa Spell.cpp:2849–2871: hit positivity follows damage/healing
//! and landed effects, never caster/recipient faction relation.
use shared::components::{Health, UnitFactionTemplate};
use shared::protocol::CombatLogKind;

use super::test_harness::{Harness, harness, spawn_player};
use crate::auras::{ActiveAuras, has_aura};
use crate::spell_info::SpellStore;

fn assert_native_proc(h: &Harness, passive: u32, chance: u32, mask: u32) {
    let spell = h.app.world().resource::<SpellStore>().get(passive).unwrap();
    let options = spell.aura_options.as_ref().unwrap();
    assert_eq!(options.proc_chance, chance);
    assert_eq!(options.proc_type_mask, [mask, 0]);
    assert_eq!(options.proc_category_recovery_ms, 0);
    assert!(has_aura(h.app.world(), h.caster, passive));
    eprintln!("HIT_POSITIVITY_NATIVE passive={passive} chance={chance} mask={mask:#x}");
}

#[test]
fn hit_positivity_native_harmful_proc_on_neutral_attackable_target() {
    // TDB 39440 permits normal, critical and absorbed hits; 34749 requires
    // full resist. Lava Burst 51505 launches its damage child 285452.
    let mut h = harness(Vec::new(), &[39440, 39441, 34749, 34747, 51505, 285452], 7).unwrap();
    h.advance(3);
    assert_native_proc(&h, 39440, 100, 0x10000);
    assert_native_proc(&h, 34749, 100, 0x10000);
    let store = h.app.world().resource::<SpellStore>();
    assert_eq!(
        store
            .get(39440)
            .unwrap()
            .aura_options
            .as_ref()
            .unwrap()
            .proc_hit_mask,
        0x403
    );
    assert_eq!(
        store
            .get(34749)
            .unwrap()
            .aura_options
            .as_ref()
            .unwrap()
            .proc_hit_mask,
        0x8
    );
    // FactionTemplate7 "Creature", native Northshire neutral creature faction.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(UnitFactionTemplate(7));
    assert!(!super::targets::is_hostile(h.app.world(), h.caster, h.npc));
    assert!(super::targets::relation_matches(
        h.app.world(),
        h.caster,
        h.npc,
        super::targets::Relation::Enemy,
    ));
    eprintln!("HIT_POSITIVITY_NATIVE target_faction=7 hostile=false attackable=true");
    assert!(!has_aura(h.app.world(), h.caster, 39441));
    assert!(!has_aura(h.app.world(), h.caster, 34747));
    let before = h.health(h.npc);
    h.cast_and_deliver(51505, Some(h.npc));
    h.advance(160);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(h.npc) < before, "{:?}", h.inbox.log);
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.amount > 0
            && event.target == Some(h.npc.to_bits())
    }));
    assert!(
        has_aura(h.app.world(), h.caster, 39441),
        "native harmful proc must fire on a neutral attackable target"
    );
    assert!(
        !has_aura(h.app.world(), h.caster, 34747),
        "full-resist proc must not fire on a successful hit"
    );
}

#[test]
fn hit_positivity_native_friendly_heal_proc_control() {
    // Native36488 guarantees a helpful spell/ability proc of36483.
    let mut h = harness(Vec::new(), &[36488, 36483, 2061], 5).unwrap();
    h.advance(3);
    assert_native_proc(&h, 36488, 100, 0x4400);
    let ally = spawn_player(&mut h.app, 5, 2.0);
    h.app.world_mut().get_mut::<Health>(ally).unwrap().current = 100.0;
    assert!(!super::targets::is_hostile(h.app.world(), h.caster, ally));
    assert!(!has_aura(h.app.world(), ally, 36483));
    h.cast_and_deliver(2061, Some(ally));
    h.advance(100);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(ally) > 100.0, "{:?}", h.inbox.log);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(ally)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 36483)
        .expect("native helpful proc must remain eligible after friendly healing");
    assert_eq!(aura.caster, h.caster);
}

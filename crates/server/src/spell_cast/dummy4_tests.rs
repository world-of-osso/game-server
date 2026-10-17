//! Fourth batch: bounded paths from local Mainline tooltip/effect rows.
use super::test_harness::*;
use shared::components::{Health, PowerType};

fn aura(h: &mut Harness, caster: bevy::prelude::Entity, target: bevy::prelude::Entity, id: u32) {
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

#[test]
fn dummy4_light_of_dawn_heals_only_five_injured_allies_in_radius() {
    let mut h = harness(Vec::new(), &[85222], 2).expect("real catalog");
    h.set_power(PowerType::HolyPower, 5.0);
    let allies: Vec<_> = (1..=7)
        .map(|x| spawn_player(&mut h.app, 2, x as f32))
        .collect();
    let healthy = spawn_player(&mut h.app, 2, 0.5);
    let far = spawn_player(&mut h.app, 2, 45.0);
    for &unit in allies.iter().chain([&far]) {
        h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 100.0;
    }
    h.cast_and_deliver(85222, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(allies.iter().filter(|&&u| h.health(u) > 100.0).count(), 5);
    assert_eq!(h.health(healthy), 1000.0);
    assert_eq!(h.health(far), 100.0);
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn dummy4_steady_shot_generates_catalog_twenty_focus() {
    let mut h = harness(Vec::new(), &[56641, 77443], 3).expect("real catalog");
    h.set_power(PowerType::Focus, 0.0);
    h.cast_and_deliver(56641, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(h.npc) < 5000.0);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(77443) && e.amount == 20)
    );
}

#[test]
fn dummy4_kill_shot_rejects_twenty_percent_before_spending_focus() {
    let mut h = harness(Vec::new(), &[53351], 3).expect("real catalog");
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1000.0;
    h.set_power(PowerType::Focus, 100.0);
    h.cast_and_deliver(53351, Some(h.npc));
    h.advance(5);
    assert_eq!(h.inbox.failed.len(), 1);
    assert!(h.inbox.go.is_empty());
    assert_eq!(h.health(h.npc), 1000.0);
    assert_eq!(h.power(PowerType::Focus), 100.0);
}

#[test]
fn dummy4_kill_shot_is_usable_strictly_below_twenty_percent() {
    let mut h = harness(Vec::new(), &[53351], 3).expect("real catalog");
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 999.0;
    h.cast_and_deliver(53351, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(h.npc) < 999.0);
}

fn mangle(with_rank: bool, bleeding: bool) -> f32 {
    let mut ids = vec![33917, 5487, 703];
    if with_rank {
        ids.push(231064);
    }
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 11).expect("real catalog");
    let caster = h.caster;
    let npc = h.npc;
    aura(&mut h, caster, caster, 5487);
    if with_rank {
        aura(&mut h, caster, caster, 231064);
    }
    if bleeding {
        aura(&mut h, caster, npc, 703);
    }
    h.cast_and_deliver(33917, Some(npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(33917) && e.kind == shared::protocol::CombatLogKind::Damage)
        .unwrap()
        .amount as f32
}

#[test]
fn dummy4_mangle_rank_increases_damage_against_bleeding_target() {
    let base = mangle(false, true);
    let bonus = mangle(true, true);
    assert!((bonus / base - 1.2).abs() < 0.07, "{base} -> {bonus}");
}

#[test]
fn dummy4_mangle_rank_without_bleed_keeps_base_damage() {
    let base = mangle(false, false);
    let ranked = mangle(true, false);
    assert!((ranked / base - 1.0).abs() < 0.06);
}

fn vivify(with_rank: bool) {
    let mut ids = vec![116670, 119611, 425804];
    if with_rank {
        ids.push(274586);
    }
    let mut h = harness(Vec::new(), &ids, 10).expect("real catalog");
    let primary = spawn_player(&mut h.app, 10, 2.0);
    let owned = spawn_player(&mut h.app, 10, 4.0);
    let foreign = spawn_player(&mut h.app, 10, 5.0);
    let unmarked = spawn_player(&mut h.app, 10, 6.0);
    let far = spawn_player(&mut h.app, 10, 150.0);
    let caster = h.caster;
    if with_rank {
        aura(&mut h, caster, caster, 274586);
    }
    aura(&mut h, caster, owned, 119611);
    aura(&mut h, caster, far, 119611);
    aura(&mut h, foreign, foreign, 119611);
    for u in [primary, owned, foreign, unmarked, far] {
        h.app.world_mut().get_mut::<Health>(u).unwrap().current = 100.0;
    }
    h.cast_and_deliver(116670, Some(primary));
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(primary) > 100.0);
    let splash = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(425804))
        .count();
    assert_eq!(splash, usize::from(with_rank));
    assert_eq!(h.health(unmarked), 100.0);
    if with_rank {
        assert!(h.health(owned) > 400.0);
    } else {
        assert!(h.health(owned) < 200.0);
    }
    // Only HoT ticks, never the323-point Vivify splash, reach foreign/far marks.
    assert!(h.health(foreign) < 200.0);
    assert!(h.health(far) < 200.0);
}

#[test]
fn dummy4_vivify_rank_heals_owned_renewing_mist_only() {
    vivify(true);
}
#[test]
fn dummy4_vivify_without_rank_does_not_splash() {
    vivify(false);
}

#[test]
fn dummy4_renewing_mist_cast_applies_catalog_hot_and_expires() {
    let mut h = harness(Vec::new(), &[115151, 119611], 10).expect("real catalog");
    let target = spawn_player(&mut h.app, 10, 3.0);
    let unmarked = spawn_player(&mut h.app, 10, 5.0);
    for unit in [target, unmarked] {
        h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 100.0;
    }
    h.cast_and_deliver(115151, Some(target));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), target, 119611));
    let before = h.health(target);
    h.advance(42);
    assert!(varied(19.665).contains(&(h.health(target) - before)));
    assert_eq!(h.health(unmarked), 100.0);
    h.advance(410);
    assert!(!crate::auras::has_aura(h.app.world(), target, 119611));
}

fn chain_lightning(with_rank: bool) {
    let mut ids = vec![188443];
    if with_rank {
        ids.push(343725);
    }
    let mut h = harness(Vec::new(), &ids, 7).expect("real catalog");
    let near = spawn_npc(&mut h.app, 6.0);
    let far = spawn_npc(&mut h.app, 80.0);
    let caster = h.caster;
    if with_rank {
        aura(&mut h, caster, caster, 343725);
    }
    h.set_power(PowerType::Maelstrom, 0.0);
    h.cast_and_deliver(188443, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(near) < 5000.0);
    assert_eq!(h.health(far), 5000.0);
    assert_eq!(
        h.power(PowerType::Maelstrom),
        if with_rank { 4.0 } else { 0.0 }
    );
}
#[test]
fn dummy4_chain_lightning_generates_two_maelstrom_per_hit_with_rank() {
    chain_lightning(true);
}
#[test]
fn dummy4_chain_lightning_without_rank_does_not_generate_maelstrom() {
    chain_lightning(false);
}

#[test]
fn dummy4_howling_blast_applies_frost_fever_to_each_hit_enemy() {
    let mut h = harness(Vec::new(), &[49184, 55095], 6).expect("real catalog");
    let near = spawn_npc(&mut h.app, 5.0);
    let far = spawn_npc(&mut h.app, 80.0);
    let ally = spawn_player(&mut h.app, 6, 4.0);
    h.cast_and_deliver(49184, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    for u in [h.npc, near] {
        assert!(crate::auras::has_aura(h.app.world(), u, 55095));
    }
    for u in [far, ally] {
        assert!(!crate::auras::has_aura(h.app.world(), u, 55095));
    }
    let before = h.health(near);
    h.advance(65);
    assert!(h.health(near) < before);
}

#[test]
fn dummy4_prayer_reduces_running_sanctify_recharge_by_six_seconds() {
    let mut h = harness(Vec::new(), &[596, 98367, 34861], 5).expect("real catalog");
    // Sanctify is ground-targeted: send its authentic destination through IPC.
    use lightyear::prelude::*;
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::SpellCastIntent {
            spell_id: Some(34861),
            spell: String::new(),
            target_entity: None,
            target_item_guid: None,
            witness: None,
            destination: Some([0.0, 0.0, 0.0]),
        });
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let category = world_db()
        .unwrap()
        .store
        .get(34861)
        .unwrap()
        .categories
        .as_ref()
        .unwrap()
        .charge_category;
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&category]
        .next_ready_at();
    h.cast_and_deliver(596, Some(h.caster));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let after = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&category]
        .next_ready_at();
    assert_eq!(after, before - 6.0);
    assert!(h.inbox.charges.iter().any(|e| e.spell_id == 34861));
}

#[test]
fn dummy4_eruption_extends_only_owned_active_ebon_might_with_rank() {
    let mut h = harness(Vec::new(), &[395160, 395153, 395152, 395296], 13).expect("real catalog");
    let ally = spawn_player(&mut h.app, 13, 4.0);
    let foreign = spawn_player(&mut h.app, 13, 5.0);
    let caster = h.caster;
    aura(&mut h, caster, caster, 395153);
    aura(&mut h, caster, caster, 395296);
    aura(&mut h, caster, ally, 395152);
    aura(&mut h, foreign, foreign, 395152);
    let expiry = |h: &Harness, u| {
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(u)
            .unwrap()
            .iter()
            .find(|a| [395152, 395296].contains(&a.spell_id))
            .unwrap()
            .expires_at
            .unwrap()
    };
    let own = expiry(&h, caster);
    let other = expiry(&h, ally);
    let untouched = expiry(&h, foreign);
    h.set_power(PowerType::Essence, 5.0);
    h.cast_and_deliver(395160, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(expiry(&h, caster), own + 1.0);
    assert_eq!(expiry(&h, ally), other + 1.0);
    assert_eq!(expiry(&h, foreign), untouched);
}

#[test]
fn dummy4_prayer_does_not_start_ready_sanctify_cooldown() {
    let mut h = harness(Vec::new(), &[596, 98367, 34861], 5).expect("real catalog");
    h.cast_and_deliver(596, Some(h.caster));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!h.inbox.charges.iter().any(|event| event.spell_id == 34861));
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|event| event.spell_id == 34861)
    );
}

#[test]
fn dummy4_eruption_without_rank_keeps_owned_buff_duration() {
    let mut h = harness(Vec::new(), &[395160, 395296], 13).expect("real catalog");
    let caster = h.caster;
    aura(&mut h, caster, caster, 395296);
    let before = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 395296)
        .unwrap()
        .expires_at;
    h.set_power(PowerType::Essence, 5.0);
    h.cast_and_deliver(395160, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let after = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 395296)
        .unwrap()
        .expires_at;
    assert_eq!(before, after);
}

#[test]
fn dummy4_light_of_dawn_mainline_radius_includes_allies_behind_caster() {
    let mut h = harness(Vec::new(), &[85222], 2).expect("real catalog");
    let front = spawn_player(&mut h.app, 2, 3.0);
    let behind = spawn_player(&mut h.app, 2, -3.0);
    for unit in [front, behind] {
        h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 100.0;
    }
    h.set_power(PowerType::HolyPower, 5.0);
    h.cast_and_deliver(85222, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(front) > 100.0);
    // Current Mainline tooltip says "radius around you", not a cone.
    assert!(h.health(behind) > 100.0);
}

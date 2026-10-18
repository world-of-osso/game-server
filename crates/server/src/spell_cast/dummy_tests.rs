//! Scripted class spells through the player cast pipeline, using real world.db IDs.

use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

use super::test_harness::*;

#[test]
fn dummy_priest_penance_channels_three_damage_bolts() {
    let mut h = harness(Vec::new(), &[47540, 47758, 47757, 47666, 47750], 5)
        .expect("real world.db required");
    h.cast_and_deliver(47540, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let cast = h
        .app
        .world()
        .get::<shared::casting::CastState>(h.caster)
        .expect("Penance channel");
    assert_eq!(cast.spell_id, 47758);
    assert_eq!(cast.cast_type, shared::casting::CastType::Channel);
    h.advance(50);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_none()
    );
    let bolts: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(47666))
        .collect();
    assert_eq!(bolts.len(), 3);
    assert!(
        bolts
            .iter()
            .all(|event| varied(93.2).contains(&(event.amount as f32)))
    );
    assert!(varied(93.2 * 3.0).contains(&(5000.0 - h.health(h.npc))));
}

#[test]
fn dummy_channel_destination_uses_live_channel_not_unrelated_explicit_unit() {
    let mut h =
        harness(Vec::new(), &[47540, 47758, 47666, 387152], 5).expect("real world.db required");
    let child = world_db().unwrap().store.get(387152).unwrap().clone();
    let effect = child
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let destination = |h: &Harness, explicit| {
        super::targets::effect_dest(h.app.world(), h.caster, explicit, effect, None)
    };
    assert!(
        destination(&h, Some(h.caster)).is_none(),
        "no destination without a channel"
    );
    h.cast_and_deliver(47540, Some(h.npc));
    let target_position = super::targets::position(h.app.world(), h.npc).unwrap();
    assert_eq!(
        destination(&h, Some(h.caster)).unwrap().position,
        target_position
    );
    // An actual channel destination takes priority over its unit target.
    let requested = super::targets::position(h.app.world(), h.caster).unwrap();
    assert_ne!(
        requested, target_position,
        "distinct existing native unit positions"
    );
    h.app
        .world_mut()
        .get_mut::<super::PendingCastSpell>(h.caster)
        .unwrap()
        .spell
        .cast_destination = Some(requested.to_array());
    assert_eq!(destination(&h, Some(h.npc)).unwrap().position, requested);
    let caster = h.caster;
    super::cancel_cast(h.app.world_mut(), caster);
    assert!(
        destination(&h, Some(h.caster)).is_none(),
        "ended channels supply no destination"
    );
}

#[test]
fn dummy_priest_penance_channels_three_healing_bolts() {
    let mut h = harness(Vec::new(), &[47540, 47758, 47757, 47666, 47750], 5)
        .expect("real world.db required");
    let ally = spawn_player(&mut h.app, 1, 2.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 100.0,
        max: 5000.0,
    };
    h.cast_and_deliver(47540, Some(ally));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .expect("Penance channel")
            .spell_id,
        47757
    );
    h.advance(50);
    let bolts: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(47750))
        .collect();
    assert_eq!(bolts.len(), 3);
    assert!(
        bolts
            .iter()
            .all(|event| varied(401.1).contains(&(event.amount as f32)))
    );
    assert!(varied(401.1 * 3.0).contains(&(h.health(ally) - 100.0)));
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn dummy_priest_stopping_penance_stops_subsequent_bolts() {
    use lightyear::prelude::*;
    use shared::protocol::{CombatChannel, StopSpellCast};
    let mut h = harness(Vec::new(), &[47540, 47758, 47757, 47666, 47750], 5)
        .expect("real world.db required");
    h.cast_and_deliver(47540, Some(h.npc));
    h.advance(6); // Initial bolt in flight has landed, second hasn't launched.
    assert!(h.health(h.npc) < 5000.0);
    h.app
        .world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    h.advance(12);
    let after_stop = h.health(h.npc);
    h.advance(45);
    assert_eq!(h.health(h.npc), after_stop);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(47666))
            .count(),
        1
    );
}

#[test]
fn dummy_druid_barkskin_with_brambles_pulses_catalog_damage_each_second() {
    let mut h = harness(Vec::new(), &[22812, 203953, 213709], 11).expect("real world.db required");
    let ally = spawn_player(&mut h.app, 1, 2.0);
    h.cast_and_deliver(22812, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    // Attributes5 has EXTRA_INITIAL_PERIOD (0x200): one pulse on apply,
    // then one per second. Keep the catalog timing instead of inventing delay.
    assert!(varied(7.5).contains(&(5000.0 - h.health(h.npc))));
    let first = h.health(h.npc);
    h.advance(10);
    assert_eq!(h.health(h.npc), first, "no second pulse before one second");
    h.advance(12);
    assert!(varied(7.5).contains(&(first - h.health(h.npc))));
    h.advance(20);
    assert!(h.health(h.npc) < first - 8.0, "third pulse");
    let pulses: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(213709))
        .collect();
    assert_eq!(pulses.len(), 3);
    assert!(
        pulses
            .iter()
            .all(|event| varied(7.5).contains(&(event.amount as f32)))
    );
    assert_eq!(h.health(ally), 1000.0, "friendly unit is not hit");
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 22812,
    );
    let after = h.health(h.npc);
    h.advance(30);
    assert_eq!(
        h.health(h.npc),
        after,
        "pulses stop when Barkskin is removed"
    );
}

#[test]
fn dummy_druid_barkskin_without_brambles_does_not_pulse() {
    let mut h = harness(Vec::new(), &[22812, 213709], 11).expect("real world.db required");
    h.cast_and_deliver(22812, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(45);
    assert_eq!(h.health(h.npc), 5000.0);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(213709))
    );
}

#[test]
fn dummy_paladin_holy_shock_damages_enemy_and_generates_holy_power() {
    let mut h = harness(Vec::new(), &[20473, 25912, 25914], 2).expect("real world.db required");
    h.cast_and_deliver(20473, Some(h.npc));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(216.0).contains(&(5000.0 - h.health(h.npc))));
    assert_eq!(h.power(PowerType::HolyPower), 1.0);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(25912)
                && event.target == Some(h.npc.to_bits()))
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal)
    );
}

#[test]
fn dummy_paladin_holy_shock_heals_ally_and_generates_holy_power() {
    let mut h = harness(Vec::new(), &[20473, 25912, 25914], 2).expect("real world.db required");
    let ally = spawn_player(&mut h.app, 1, 2.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 100.0,
        max: 5000.0,
    };
    h.cast_and_deliver(20473, Some(ally));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(1117.0).contains(&(h.health(ally) - 100.0)));
    assert_eq!(h.power(PowerType::HolyPower), 1.0);
    assert_eq!(h.health(h.npc), 5000.0);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(25914)
                && event.target == Some(ally.to_bits()))
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage)
    );
}

#[test]
fn dummy_paladin_holy_shock_heals_self() {
    let mut h = harness(Vec::new(), &[20473, 25912, 25914], 2).expect("real world.db required");
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h.cast_and_deliver(20473, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.health(h.caster), 1000.0);
    assert_eq!(h.power(PowerType::HolyPower), 1.0);
}

#[test]
fn dummy_warrior_shield_block_applies_six_second_block_aura() {
    let mut h = harness(Vec::new(), &[2565, 132404], 1).expect("real world.db required");
    h.set_power(PowerType::Rage, 1000.0);
    h.cast_and_deliver(2565, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 132404));
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap();
    let block = auras.iter().find(|aura| aura.spell_id == 132404).unwrap();
    assert_eq!(block.duration_ms, 6000);
    assert_eq!(
        block
            .effects
            .iter()
            .find(|effect| effect.aura_type == 51)
            .unwrap()
            .amount,
        100.0
    );
    h.advance(125);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 132404));
}

#[test]
fn dummy_warrior_bloodthirst_damages_generates_rage_and_heals_three_percent() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[23881, 117313, 184362], 1)
        .expect("real world.db required");
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 500.0;
    h.set_power(PowerType::Rage, 0.0);
    h.cast_and_deliver(23881, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(424.203).contains(&(5000.0 - h.health(h.npc))));
    assert_eq!(h.power(PowerType::Rage), 80.0);
    assert_eq!(h.health(h.caster), 530.0);
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(117313)
                && event.amount == 30)
    );
}

#[test]
fn dummy_warrior_bloodthirst_enrage_uses_effect_two_chance() {
    // Controlled chance endpoints on a real-ID catalog clone, not Retail's
    // 30% random roll. The unmodified-data test above proves the base cast.
    let original = world_db().unwrap().store.get(23881).unwrap();
    assert_eq!(
        original
            .effects
            .iter()
            .find(|effect| effect.index == 2)
            .unwrap()
            .base_points,
        30.0
    );
    for (chance, expected) in [(0.0, false), (100.0, true)] {
        let mut spell = original.clone();
        spell
            .effects
            .iter_mut()
            .find(|effect| effect.index == 2)
            .unwrap()
            .base_points = chance;
        let mut h =
            harness_with_native_enemy_dodge_reduction(vec![spell], &[117313, 184362], 1).unwrap();
        h.cast_and_deliver(23881, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            crate::auras::has_aura(h.app.world(), h.caster, 184362),
            expected
        );
    }
}

#[test]
fn dummy_warrior_rampage_spends_rage_hits_and_enrages() {
    let ids = [184367, 184707, 184709, 201364, 201363, 184362];
    // Every sourced child must hit here; avoidance has separate native tests.
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 1)
        .expect("real world.db required");
    h.set_power(PowerType::Rage, 1000.0);
    h.cast_and_deliver(184367, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Rage), 200.0);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 184362));
    h.advance(3);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && matches!(event.spell_id, Some(184709 | 201364 | 201363))),
        "native later hits wait300ms and900ms"
    );
    h.advance(16);
    for id in [184707, 184709, 201364, 201363] {
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(id)
                    && event.target == Some(h.npc.to_bits())),
            "missing {id}"
        );
    }
}

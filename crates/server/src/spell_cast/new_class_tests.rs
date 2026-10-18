//! Monk, Demon Hunter and Evoker core rotation spells on the real world.db:
//! each lands and generates or spends its resource.

use shared::components::{Npc, Position, PowerType, UnitFactionTemplate};
use shared::protocol::CombatLogKind;

use super::test_harness::*;

const MONK: u8 = 10;
const DEMON_HUNTER: u8 = 12;
const EVOKER: u8 = 13;

const TIGER_PALM: u32 = 100_780;
const BLACKOUT_KICK: u32 = 100_784;
/// Windwalker passives: 137025 makes Blackout Kick cost 3 Chi (SpellPower
/// RequiredAuraSpellID), 1258122 adds 2 to Tiger Palm's Chi ENERGIZE.
const WINDWALKER_MONK: u32 = 137_025;
const WINDWALKER_CHI: u32 = 1_258_122;
const DEMONS_BITE_INITIAL: u32 = 344_859;
const CHAOS_STRIKE_INITIAL: u32 = 344_862;
const LIVING_FLAME: u32 = 361_469;
const AZURE_STRIKE: u32 = 362_969;
/// Makes Living Flame cost 12% of base mana (SpellPower RequiredAuraSpellID).
const DEVASTATION_EVOKER: u32 = 356_809;

fn damage_taken(h: &Harness, unit: bevy::prelude::Entity) -> f32 {
    let health = h
        .app
        .world()
        .get::<shared::components::Health>(unit)
        .unwrap();
    health.max - health.current
}

fn damaged_by(h: &mut Harness, spell_id: u32) -> Vec<u64> {
    std::mem::take(&mut h.inbox.log)
        .into_iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(spell_id))
        .map(|event| event.target.expect("damage target"))
        .collect()
}

#[test]
fn tiger_palm_spends_50_energy_and_hits() {
    let Some(mut h) = harness_with_native_enemy_dodge_reduction(Vec::new(), &[TIGER_PALM], MONK)
    else {
        return;
    };
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(TIGER_PALM, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(damage_taken(&h, h.npc) > 0.0);
    // 50 spent, then at most 0.15 s of 10/s regen.
    let energy = h.power(PowerType::Energy);
    assert!((50.0..=51.5).contains(&energy), "energy {energy}");
    assert_eq!(h.power(PowerType::Chi), 0.0, "no Chi outside Windwalker");
}

#[test]
fn windwalker_tiger_palm_generates_2_chi_and_blackout_kick_spends_3() {
    let ids = [TIGER_PALM, BLACKOUT_KICK, WINDWALKER_MONK, WINDWALKER_CHI];
    let Some(mut h) = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, MONK) else {
        return;
    };
    h.advance(1);
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(TIGER_PALM, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Chi), 2.0);

    h.advance(30);
    h.set_power(PowerType::Chi, 3.0);
    let before = damage_taken(&h, h.npc);
    h.cast_and_deliver(BLACKOUT_KICK, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Chi), 0.0);
    assert!(damage_taken(&h, h.npc) > before);
}

/// Demon's Bite 344859 triggers 162243: damage and 25 Fury ± 20% (Variance 0.4).
#[test]
fn demons_bite_generates_fury() {
    let Some(mut h) = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[DEMONS_BITE_INITIAL, 162_243],
        DEMON_HUNTER,
    ) else {
        return;
    };
    h.set_power(PowerType::Fury, 0.0);
    h.cast_and_deliver(DEMONS_BITE_INITIAL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let fury = h.power(PowerType::Fury);
    assert!((20.0..=30.0).contains(&fury), "fury {fury}");
    h.advance(1);
    assert_eq!(damaged_by(&mut h, 162_243), vec![h.npc.to_bits()]);
}

/// Chaos Strike 344862 costs 40 Fury; its DUMMY script casts 162794, whose
/// triggers 222031 and 199547 are the two weapon hits.
#[test]
fn chaos_strike_spends_40_fury_and_hits_twice() {
    let ids = [CHAOS_STRIKE_INITIAL, 162_794, 222_031, 199_547, 193_840];
    let mut refund = world_db().unwrap().store.get(197_125).unwrap().clone();
    // This test isolates gross cost and weapon hits; dummy5 covers refund endpoints.
    refund.aura_options.as_mut().unwrap().proc_chance = 0;
    let Some(mut h) = harness_with_native_enemy_dodge_reduction(vec![refund], &ids, DEMON_HUNTER)
    else {
        return;
    };
    h.set_power(PowerType::Fury, 50.0);
    h.cast_and_deliver(CHAOS_STRIKE_INITIAL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Fury), 10.0);
    h.advance(1);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(199_547)),
        "second native hit waits300ms"
    );
    h.advance(6);
    let npc = h.npc.to_bits();
    let log = std::mem::take(&mut h.inbox.log);
    for spell_id in [222_031, 199_547] {
        assert!(
            log.iter().any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell_id)
                && event.target == Some(npc)),
            "no {spell_id} hit in {log:?}"
        );
    }
}

#[test]
fn chaos_strike_without_40_fury_fails() {
    let Some(mut h) = harness(Vec::new(), &[CHAOS_STRIKE_INITIAL], DEMON_HUNTER) else {
        return;
    };
    h.set_power(PowerType::Fury, 39.0);
    h.cast_and_deliver(CHAOS_STRIKE_INITIAL, Some(h.npc));
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.power(PowerType::Fury), 39.0);
    assert_eq!(damage_taken(&h, h.npc), 0.0);
}

/// Living Flame 361469 has a 2 s cast; its DUMMY script casts the damage
/// spell 361500 at an enemy. Devastation pays 12% of base mana (10 000).
#[test]
fn living_flame_casts_for_2_seconds_spends_mana_and_damages_enemy() {
    let Some(mut h) = harness(
        Vec::new(),
        &[LIVING_FLAME, DEVASTATION_EVOKER, 361_500, 361_509],
        EVOKER,
    ) else {
        return;
    };
    h.advance(1);
    h.cast_and_deliver(LIVING_FLAME, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(damage_taken(&h, h.npc), 0.0, "still casting");
    // 2 s cast, then the 361500 missile's flight (Speed 45).
    h.advance(80);
    assert_eq!(damaged_by(&mut h, 361_500), vec![h.npc.to_bits()]);
    let mana = h
        .app
        .world()
        .get::<shared::components::Mana>(h.caster)
        .unwrap()
        .current;
    // 1 200 spent, then up to 4 s of 1%-of-base-mana regen per second.
    assert!((8_800.0..=9_200.0).contains(&mana), "mana {mana}");
}

#[test]
fn living_flame_heals_an_ally_without_damaging_the_enemy() {
    let ids = [LIVING_FLAME, DEVASTATION_EVOKER, 361_500, 361_509];
    let mut h = harness(Vec::new(), &ids, EVOKER).expect("local world.db required");
    let ally = spawn_player(&mut h.app, 1, 2.0);
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(ally)
        .unwrap()
        .current = 100.0;
    h.advance(1);
    h.cast_and_deliver(LIVING_FLAME, Some(ally));
    assert_eq!(h.health(ally), 100.0, "still casting");
    // 2 s cast, then the 361509 missile's flight (Speed 40).
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(ally) > 100.0);
    assert_eq!(damage_taken(&h, h.npc), 0.0);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(361_509)
                && event.target == Some(ally.to_bits()))
    );
}

/// Azure Strike's area damage lands on the target and one more enemy
/// (effect 0: 2 targets), not on every enemy in range.
#[test]
fn azure_strike_hits_the_target_and_one_other_enemy() {
    let Some(mut h) = harness(Vec::new(), &[AZURE_STRIKE, 355_627], EVOKER) else {
        return;
    };
    let extra: Vec<_> = (0..2)
        .map(|i| {
            h.app
                .world_mut()
                .spawn((
                    Npc {
                        template_id: 6,
                        name: "Kobold Vermin".into(),
                    },
                    UnitFactionTemplate(25),
                    Position {
                        x: 3.0,
                        y: 1.0 + i as f32,
                        z: 0.0,
                    },
                    crate::maps::InMap::continent(0),
                    shared::components::Health {
                        current: 5000.0,
                        max: 5000.0,
                    },
                ))
                .id()
        })
        .collect();
    h.advance(1);
    h.cast_and_deliver(AZURE_STRIKE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(1);
    let hit = damaged_by(&mut h, AZURE_STRIKE);
    assert_eq!(hit.len(), 2, "{hit:?}");
    assert!(hit.contains(&h.npc.to_bits()));
    assert!(extra.iter().any(|unit| hit.contains(&unit.to_bits())));
}

/// Energizing Flame 400006 refunds 80% (effect 0) of Living Flame's mana cost
/// when it hits an enemy: 12% of 10 000 base mana = 1 200 spent, 960 back.
#[test]
fn energizing_flame_refunds_most_of_living_flames_mana() {
    let ids = [LIVING_FLAME, DEVASTATION_EVOKER, 361_500, 400_006];
    let Some(mut h) = harness(Vec::new(), &ids, EVOKER) else {
        return;
    };
    h.advance(1);
    h.cast_and_deliver(LIVING_FLAME, Some(h.npc));
    // 2 s cast, then the 361500 missile's flight (Speed 45).
    h.advance(80);
    eprintln!(
        "failed {:?} log {:?}",
        h.inbox.failed,
        h.inbox
            .log
            .iter()
            .map(|e| (e.kind, e.spell_id))
            .collect::<Vec<_>>()
    );
    assert_eq!(damaged_by(&mut h, 361_500), vec![h.npc.to_bits()]);
    let mana = h
        .app
        .world()
        .get::<shared::components::Mana>(h.caster)
        .unwrap()
        .current;
    // 1 200 spent, 960 refunded, then up to 4 s of regen.
    assert!((9_760.0..=10_000.0).contains(&mana), "mana {mana}");
}

//! Batch11 local Mainline tooltip paths, exercised through native casts/impacts.
use super::test_harness::*;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::components::{Level, Position, PowerType};
use shared::protocol::{CombatChannel, CombatLogKind, SpellCastIntent};

fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32, mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
fn no_rank(h: &mut Harness, id: u32) {
    let known = h
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .iter()
        .filter(|&i| i != id)
        .collect();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(known));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == id,
    );
}
fn hits(h: &Harness, id: u32, unit: Entity) -> Vec<i32> {
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(id)
                && e.kind == CombatLogKind::Damage
                && e.target == Some(unit.to_bits())
        })
        .map(|e| e.amount)
        .collect()
}
fn ground(h: &mut Harness, point: [f32; 3]) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(5740),
            spell: String::new(),
            target_entity: None,
            destination: Some(point),
            target_item_guid: None,
            witness: None,
        });
    h.advance(3);
}

#[test]
fn dummy11_rain_of_fire_overlaps_keep_independent_clocks() {
    let mut h = harness(Vec::new(), &[5740, 42223], 9).unwrap();
    let first = spawn_npc(&mut h.app, 21.0);
    let second = spawn_npc(&mut h.app, 41.0);
    h.set_power(PowerType::SoulShards, 50.0);
    ground(&mut h, [20.0, 0.0, 0.0]);
    h.advance(35);
    h.set_power(PowerType::SoulShards, 50.0);
    ground(&mut h, [40.0, 0.0, 0.0]);
    landed(&h);
    h.advance(180);
    assert_eq!(hits(&h, 42223, first), vec![60; 8]);
    assert_eq!(hits(&h, 42223, second), vec![60; 8]);
}

#[test]
fn dummy11_cancelled_voidform_never_launches_volley() {
    let mut h = harness(Vec::new(), &[228260, 194249, 1242173, 1242189], 5).unwrap();
    h.cast_and_deliver(228260, Some(h.npc));
    super::cancel_cast(h.app.world_mut(), h.caster);
    h.advance(60);
    assert!(hits(&h, 1242189, h.npc).is_empty());
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 194249));
}

#[test]
fn dummy11_shred_doubles_modified_crit_chance() {
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[5221, 768, 5215, 343232], 11)
            .unwrap();
    cat(&mut h);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 50.0;
    let c = h.caster;
    apply(&mut h, c, c, 5215, 1);
    h.cast_and_deliver(5221, Some(h.npc));
    h.advance(3);
    landed(&h);
    let strike = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(5221) && e.kind == CombatLogKind::Damage)
        .unwrap();
    assert!(strike.crit);
    assert!(varied(260.48 * 1.6 * 2.0).contains(&(strike.amount as f32)));
}

#[test]
fn dummy11_lashing_flames_excludes_foreign_caster_and_ends_on_expiry() {
    let mut h = harness(Vec::new(), &[60103, 334046, 334168, 188389], 7).unwrap();
    h.cast_and_deliver(60103, Some(h.npc));
    h.advance(3);
    landed(&h);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 334168));
    let foreign = spawn_player(&mut h.app, 7, 0.0);
    let spell = world_db().unwrap().store.get(188389).unwrap();
    let base = super::effects::scaled_amount(h.app.world(), foreign, spell, &spell.effects[0]);
    // Foreign caster has no connection; observe target health, not owner's inbox.
    let before = h.health(h.npc);
    super::cast_triggered(h.app.world_mut(), foreign, 188389, Some(h.npc), 1);
    h.advance(3);
    assert!(varied(base).contains(&(before - h.health(h.npc))));
    h.advance(410);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 334168));
    h.inbox.log.clear();
    let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
    super::cast_triggered(h.app.world_mut(), h.caster, 188389, Some(h.npc), 1);
    h.advance(3);
    assert!(varied(base).contains(&(hits(&h, 188389, h.npc).iter().sum::<i32>() as f32)));
}

fn landed(h: &Harness) {
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}
fn cat(h: &mut Harness) {
    let c = h.caster;
    h.cast_and_deliver(768, Some(c));
    h.advance(35);
    landed(h);
    h.set_power(PowerType::Energy, 100.0);
    h.set_power(PowerType::ComboPoints, 0.0);
}

#[test]
fn dummy11_sinister_strike_uses_authored_level_curve() {
    for (level, factor) in [
        (1, 195.0 / 275.0),
        (10, 195.0 / 275.0),
        (15, 235.0 / 275.0),
        (20, 1.0),
        (80, 1.0),
    ] {
        let mut h = harness(Vec::new(), &[193315], 4).unwrap();
        h.app.world_mut().entity_mut(h.caster).insert(Level(level));
        let spell = world_db().unwrap().store.get(193315).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
        super::cast_triggered(h.app.world_mut(), h.caster, 193315, Some(h.npc), 1);
        h.advance(3);
        let damage: i32 = hits(&h, 193315, h.npc).iter().sum();
        assert!(
            varied(base * factor).contains(&(damage as f32)),
            "level {level}: base {base}, damage {damage}"
        );
    }
}
#[test]
fn dummy11_void_volley_counts_primary_secondary_bolts_and_insanity() {
    let mut h = harness(Vec::new(), &[1242173, 1242189], 5).unwrap();
    let near = spawn_npc(&mut h.app, 4.0);
    let far = spawn_npc(&mut h.app, 20.0);
    let ally = spawn_player(&mut h.app, 5, 4.0);
    h.set_power(PowerType::Insanity, 0.0);
    h.cast_and_deliver(1242173, Some(h.npc));
    h.advance(12);
    landed(&h);
    assert_eq!(hits(&h, 1242189, h.npc).len(), 10);
    assert_eq!(hits(&h, 1242189, near).len(), 1);
    assert!(hits(&h, 1242189, far).is_empty());
    assert!(hits(&h, 1242189, ally).is_empty());
    assert_eq!(h.power(PowerType::Insanity), 1000.0);
}
#[test]
fn dummy11_voidform_launches_initial_volley_after_cast_completion() {
    let mut h = harness(Vec::new(), &[228260, 194249, 1242173, 1242189], 5).unwrap();
    h.cast_and_deliver(228260, Some(h.npc));
    h.advance(10);
    assert!(hits(&h, 1242189, h.npc).is_empty());
    h.advance(40);
    landed(&h);
    assert_eq!(hits(&h, 1242189, h.npc).len(), 10);
}
#[test]
fn dummy11_rain_of_fire_has_eight_fixed_ground_pulses() {
    let mut h = harness(Vec::new(), &[5740, 42223], 9).unwrap();
    h.set_power(PowerType::SoulShards, 50.0);
    let near = spawn_npc(&mut h.app, 21.0);
    let far = spawn_npc(&mut h.app, 29.0);
    let ally = spawn_player(&mut h.app, 9, 21.0);
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(5740),
            spell: String::new(),
            target_entity: None,
            destination: Some([20.0, 0.0, 0.0]),
            target_item_guid: None,
            witness: None,
        });
    h.advance(5);
    landed(&h);
    assert!(hits(&h, 42223, near).is_empty());
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: 40.0,
        y: 0.0,
        z: 0.0,
    });
    h.advance(180);
    assert_eq!(hits(&h, 42223, near), vec![60; 8]);
    for u in [far, ally, h.npc] {
        assert!(hits(&h, 42223, u).is_empty());
    }
    h.advance(40);
    assert_eq!(hits(&h, 42223, near).len(), 8);
}
#[test]
fn dummy11_blade_of_justice_splash_requires_blade_talent() {
    for rank in [false, true] {
        let mut h = harness(Vec::new(), &[184575, 404358, 403826], 2).unwrap();
        if !rank {
            no_rank(&mut h, 403826);
        }
        let near = spawn_npc(&mut h.app, 5.0);
        // Native child radius is8yd despite its tooltip saying6yd.
        let far = spawn_npc(&mut h.app, 11.1);
        h.cast_and_deliver(184575, Some(h.npc));
        h.advance(8);
        landed(&h);
        assert_eq!(!hits(&h, 404358, near).is_empty(), rank);
        assert!(hits(&h, 404358, far).is_empty());
        assert_eq!(h.power(PowerType::HolyPower), 1.0);
    }
}
#[test]
fn dummy11_hammer_wave_requires_caster_in_owned_consecration() {
    for (field, foreign, caster_x) in [
        (false, false, 0.0),
        (true, false, 0.0),
        (true, false, 20.0),
        (true, true, 0.0),
    ] {
        let mut h = harness(Vec::new(), &[53595, 88263, 26573, 81297, 1302139], 2).unwrap();
        h.app.insert_resource(load_native_creature_world(&[6]));
        let near = spawn_npc(&mut h.app, 4.0);
        let far = spawn_npc(&mut h.app, 13.0);
        if field {
            let owner = if foreign {
                spawn_player(&mut h.app, 2, 0.0)
            } else {
                h.caster
            };
            h.advance(1);
            super::cast_granted(h.app.world_mut(), owner, 26573, None).unwrap();
            h.advance(3);
        }
        h.app.world_mut().entity_mut(h.caster).insert(Position {
            x: caster_x,
            y: 0.0,
            z: 0.0,
        });
        super::cast_triggered(h.app.world_mut(), h.caster, 53595, Some(h.npc), 0);
        h.advance(3);
        let wave = field && !foreign && caster_x == 0.0;
        assert_eq!(!hits(&h, 88263, near).is_empty(), wave);
        assert!(hits(&h, 88263, h.npc).is_empty());
        assert!(hits(&h, 88263, far).is_empty());
    }
}
#[test]
fn dummy11_shred_stealth_rank_grants_damage_and_combo_point() {
    for (rank, stealth) in [(false, true), (true, false), (true, true)] {
        let mut h = harness(Vec::new(), &[5221, 768, 5215, 343232, 405834], 11).unwrap();
        cat(&mut h);
        if !rank {
            no_rank(&mut h, 343232);
            no_rank(&mut h, 405834);
        }
        if stealth {
            let c = h.caster;
            apply(&mut h, c, c, 5215, 1);
        }
        let spell = world_db().unwrap().store.get(5221).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
        h.cast_and_deliver(5221, Some(h.npc));
        h.advance(3);
        landed(&h);
        let bonus = rank && stealth;
        let damage: i32 = hits(&h, 5221, h.npc).iter().sum();
        assert!(
            varied(base * if bonus { 1.6 } else { 1.0 }).contains(&(damage as f32)),
            "{rank}/{stealth}: {base}/{damage}"
        );
        assert_eq!(
            h.power(PowerType::ComboPoints),
            if bonus { 2.0 } else { 1.0 }
        );
    }
}
#[test]
fn dummy11_thrash_applies_named_bleed_only_to_hit_enemies_and_expires() {
    let mut h = harness(Vec::new(), &[106830, 405233, 768], 11).unwrap();
    cat(&mut h);
    let near = spawn_npc(&mut h.app, 4.0);
    let far = spawn_npc(&mut h.app, 20.0);
    let ally = spawn_player(&mut h.app, 11, 4.0);
    h.cast_and_deliver(106830, None);
    h.advance(3);
    landed(&h);
    for u in [h.npc, near] {
        assert!(crate::auras::has_aura(h.app.world(), u, 405233));
    }
    for u in [far, ally] {
        assert!(!crate::auras::has_aura(h.app.world(), u, 405233));
    }
    h.advance(65);
    assert!(!hits(&h, 405233, near).is_empty());
    h.advance(400);
    assert!(!crate::auras::has_aura(h.app.world(), near, 405233));
}
#[test]
fn dummy11_lava_lash_applies_owned_lashing_flames_only_with_rank() {
    for rank in [false, true] {
        let mut h = harness(
            Vec::new(),
            &[60103, 334046, 334168, 188389, 188196, 343725],
            7,
        )
        .unwrap();
        if !rank {
            no_rank(&mut h, 334046);
        }
        h.cast_and_deliver(60103, Some(h.npc));
        h.advance(3);
        landed(&h);
        assert_eq!(crate::auras::has_aura(h.app.world(), h.npc, 334168), rank);
        let spell = world_db().unwrap().store.get(188389).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
        super::cast_triggered(h.app.world_mut(), h.caster, 188389, Some(h.npc), 1);
        h.advance(3);
        let damage: i32 = hits(&h, 188389, h.npc).iter().sum();
        assert!(
            varied(base * if rank { 2.0 } else { 1.0 }).contains(&(damage as f32)),
            "{base}/{damage}"
        );
        let bolt = world_db().unwrap().store.get(188196).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, bolt, &bolt.effects[0]);
        super::cast_triggered(h.app.world_mut(), h.caster, 188196, Some(h.npc), 1);
        h.advance(20);
        let damage: i32 = hits(&h, 188196, h.npc).iter().sum();
        assert!(
            varied(base).contains(&(damage as f32)),
            "unrelated bolt {base}/{damage}"
        );
    }
}

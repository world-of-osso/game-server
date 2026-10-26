//! Native equipped Loa/Rite/Ritual Stone stat procs through ordinary300s combat, no forced rolls.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_info::SpellStore;
use std::collections::BTreeSet;

fn prepare_item_probe(spec: u32) -> Run {
    use crate::followers::simulator::ParityActor;
    let (profile, traits) = read_reference(spec).unwrap();
    let identity = match spec {
        70 | 266 | 269 => ParityActor::Follower,
        62 | 63 | 64 | 71 | 72 | 261 | 263 | 265 | 267 | 577 => ParityActor::Player {
            race: inputs::read_reference_race(&profile.race).unwrap(),
        },
        _ => panic!("unconfigured item-control specialization{spec}"),
    };
    let driver = measurement_driver(spec, &identity).expect("native literal item-control driver");
    let mut run = prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    run.spec_driver = Some((driver.prepare)(&mut run).unwrap());
    run
}

#[test]
#[ignore = "native Ret Band/Peridot profile and item tables"]
fn shared_stat_items_loa_native_band_dispatches_both_equipped_stat_buffs() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_item_probe(70);
    let driver = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1251904 && aura.caster == run.actor)
        .expect("actually equipped native Loa Band driver");
    let capybara = driver
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    let level = driver.cast_item_level.expect("equipped Band item level");
    assert!(capybara > 0.0, "native driver owns primary-stat amount");
    let mut akilzon = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(1252818)
        .unwrap()
        .clone();
    akilzon.cast_item_level = Some(level);
    let effect = akilzon
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let haste = crate::spell_cast::effects::modded_base_points(
        run.app.world(),
        run.actor,
        &akilzon,
        effect,
    );
    assert!(
        haste > 0.0,
        "native Akilzon effect owns its own scaled stat amount"
    );
    let mut seen = BTreeSet::new();
    engage(&mut run);
    let (seconds, _, _) = advance_combat_observed(&mut run, |run| {
        let auras = run.app.world().get::<ActiveAuras>(run.actor).unwrap();
        for (id, expected) in [(1252524, capybara), (1252818, haste)] {
            let Some(buff) = auras
                .iter()
                .find(|aura| aura.spell_id == id && aura.caster == run.actor)
            else {
                continue;
            };
            let amount = buff
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount;
            assert!(
                (amount - expected).abs() < 0.001,
                "Loa{id} amount{amount} must match native source{expected}"
            );
            assert_eq!(
                buff.cast_item_level,
                Some(level),
                "native equipped item scaling is retained"
            );
            seen.insert(id);
        }
    })
    .expect("ordinary native Ret combat");
    assert!(
        report::combat(run.app.world(), &run.damage_targets, seconds)["total_damage"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert_eq!(
        seen,
        BTreeSet::from([1252524, 1252818]),
        "real native Band/Peridot procs must dispatch both eligible stat children"
    );
}

#[test]
#[ignore = "native70/269/71/63/72 equipped Rite profiles and item tables"]
fn shared_stat_items_rite_native_rating_children_use_linked_driver_value_across_specs() {
    for spec in [70, 269, 71, 63, 72] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_item_probe(spec);
        let driver = run
            .app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1297654 && aura.caster == run.actor)
            .expect("native enchant really linked the proc driver");
        let amount = driver
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount;
        assert!(amount > 0.0, "native linked BP0 owns the rank-scaled value");
        let payloads: BTreeSet<_> = run
            .app
            .world()
            .resource::<SpellStore>()
            .get(driver.spell_id)
            .expect("native Rite payload catalog")
            .effects
            .iter()
            .filter(|effect| effect.aura == 4 && effect.trigger_spell != 0)
            .map(|effect| effect.trigger_spell)
            .collect();
        assert!(
            run.app
                .world()
                .get::<ActiveAuras>(run.actor)
                .unwrap()
                .iter()
                .all(|aura| !payloads.contains(&aura.spell_id)),
            "item callbacks cannot grant Rite stats during precombat setup"
        );
        let mut seen = BTreeSet::new();
        engage(&mut run);
        let (seconds, _, _) = advance_combat_observed(&mut run, |run| {
            let auras = run.app.world().get::<ActiveAuras>(run.actor).unwrap();
            for buff in auras
                .iter()
                .filter(|aura| aura.caster == run.actor && payloads.contains(&aura.spell_id))
            {
                let value = buff
                    .effects
                    .iter()
                    .find(|effect| effect.aura_type == 189)
                    .unwrap()
                    .amount;
                assert!(
                    (value - amount).abs() < 0.001,
                    "Rite spec{spec} child{} must use native linked amount{amount}, got{value}",
                    buff.spell_id
                );
                seen.insert(buff.spell_id);
            }
        })
        .expect("ordinary native equipped-spec combat");
        let combat = report::combat(run.app.world(), &run.damage_targets, seconds);
        assert!(combat["total_damage"].as_f64().unwrap() > 0.0);
        assert!(
            !seen.is_empty(),
            "native Rite spec{spec} never dispatched a rating child"
        );
        eprintln!(
            "NATIVE_RITE spec{spec} seen={seen:?} value={amount} single_seed_dps={}",
            combat["dps"]
        );
    }
}

fn native_ritual_root(run: &Run) -> crate::spell_info::SpellInfo {
    let world = run.app.world();
    let root = world
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| {
            aura.spell_id == 1297382
                && matches!(aura.source, crate::auras::AuraSource::Equipment(_))
        })
        .expect("actually equipped native Hunter's Ritual Stone root");
    let mut native = world
        .resource::<SpellStore>()
        .get(root.spell_id)
        .unwrap()
        .clone();
    native.cast_item_level = root.cast_item_level;
    assert!(
        native.cast_item_level.is_some(),
        "native equipped root retains item level"
    );
    native
}

#[test]
#[ignore = "native Hunter's Ritual Stone must dispatch scaled rating buffs on every equipping profile"]
fn shared_stat_items_ritual_stone_native_rating_children_follow_equipped_root_across_specs() {
    let payloads = BTreeSet::from([1295898, 1295899, 1295900, 1295901]);
    let mut all_seen = BTreeSet::new();
    for spec in [63, 62, 64, 72, 261, 263, 265, 266, 267, 577] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_item_probe(spec);
        let root = native_ritual_root(&run);
        let proc_spell = root
            .effects
            .iter()
            .find(|effect| effect.aura == 285)
            .unwrap()
            .trigger_spell;
        assert_eq!(
            proc_spell, 1295897,
            "discover the linked RPPM driver from native data"
        );
        let amount_effect = root
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap();
        let amount = crate::spell_cast::effects::modded_base_points(
            run.app.world(),
            run.actor,
            &root,
            amount_effect,
        );
        assert!(
            amount > 0.0,
            "native root effect2 supplies the scaled rating amount"
        );
        let auras = run.app.world().get::<ActiveAuras>(run.actor).unwrap();
        assert!(auras.iter().any(|aura| aura.spell_id == proc_spell));
        assert!(
            auras.iter().all(|aura| !payloads.contains(&aura.spell_id)),
            "Stone cannot grant stats before genuine combat"
        );
        let mut seen = BTreeSet::new();
        engage(&mut run);
        let (seconds, uptimes, _) = advance_combat_observed(&mut run, |run| {
            let world = run.app.world();
            for buff in world
                .get::<ActiveAuras>(run.actor)
                .unwrap()
                .iter()
                .filter(|aura| aura.caster == run.actor && payloads.contains(&aura.spell_id))
            {
                let native = world.resource::<SpellStore>().get(buff.spell_id).unwrap();
                let rating = buff
                    .effects
                    .iter()
                    .find(|effect| effect.aura_type == 189)
                    .unwrap();
                let native_rating = native
                    .effects
                    .iter()
                    .find(|effect| effect.aura == 189)
                    .unwrap();
                assert!(
                    (rating.amount - amount).abs() < 0.001,
                    "Stone spec{spec} child{} must use native root amount{amount}, got{}",
                    buff.spell_id,
                    rating.amount
                );
                assert_eq!(
                    rating.misc_values, native_rating.misc_values,
                    "native rating mask"
                );
                assert_eq!(
                    buff.duration_ms,
                    native.misc.as_ref().unwrap().duration.unwrap().duration_ms as u32,
                    "native child lifetime"
                );
                assert_eq!(
                    buff.cast_item_level, root.cast_item_level,
                    "native equipped scaling source"
                );
                seen.insert(buff.spell_id);
            }
        })
        .expect("ordinary equipped native-spec combat");
        let combat = report::combat(run.app.world(), &run.damage_targets, seconds);
        assert!(
            combat["total_damage"].as_f64().unwrap() > 0.0,
            "real combat prerequisite"
        );
        assert!(
            !seen.is_empty(),
            "native Ritual Stone spec{spec} emitted no rating child"
        );
        all_seen.extend(seen.iter().copied());
        eprintln!(
            "NATIVE_RITUAL_STONE spec{spec} value={amount} level={:?} seen={seen:?} uptimes={:?} dps={}",
            root.cast_item_level,
            uptimes
                .iter()
                .filter(|(id, _)| payloads.contains(id))
                .collect::<Vec<_>>(),
            combat["dps"]
        );
    }
    assert_eq!(
        all_seen, payloads,
        "all four source-qualified native children must execute"
    );
}

//! Diabolist ritual → Demonic Art → demon → Ruination/Infernal Bolt on the real 266 follower.
use super::*;
use shared::protocol::CombatLogKind;

const RUINATION_BUFF: u32 = 433885;
const INFERNAL_BOLT_BUFF: u32 = 433891;

/// Real follower with three Shadow Bolt shards, engaged with its dummy.
fn prepare_funded() -> Run {
    let mut run = prepare_literal(266, 1).unwrap();
    for _ in 0..3 {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            686,
            Some(run.targets[0]),
            false,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    engage(&mut run);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    run
}

fn cast(run: &mut Run, spell: u32) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        false,
    )
    .unwrap_or_else(|failure| panic!("native {spell} must cast: {failure:?}"));
    preparation::finish_pending_setup(run).unwrap();
}

fn grant_art(run: &mut Run, art: u32) {
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, art, Some(run.actor), true)
        .expect("native Demonic Art must apply");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, art));
}

fn events(run: &Run, kind: CombatLogKind, spell: u32) -> Vec<shared::protocol::CombatLogEvent> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == kind
        })
        .cloned()
        .collect()
}

fn update(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

#[test]
#[ignore = "requires private266 inputs; natural Diabolic Ritual earns its real Art"]
fn demonology_expired_overlord_ritual_earns_demonic_art() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 428514));
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), actor, target);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 431944, Some(actor), true)
        .expect("native Overlord ritual must apply");
    assert!(crate::auras::has_aura(run.app.world(), actor, 431944));
    for tick in 0..420 {
        if tick % 60 == 0 {
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 686, Some(target), false)
                .expect("real Shadow Bolt keeps dummy combat active");
        }
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<shared::components::CombatStatus>(actor)
            .is_some_and(|status| status.0)
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 431944));
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 428524),
        "natural Overlord ritual expiry must earn its native Demonic Art"
    );
}

#[test]
#[ignore = "requires private266 inputs; Demonic Art consumed by a paid Hand summons the Pit Lord"]
fn demonology_art_hand_summons_pit_lord_and_grants_ruination() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    grant_art(&mut run, 432795);
    cast(&mut run, 105174);
    update(&mut run, 5);
    let summons = events(&run, CombatLogKind::Summon, 434400);
    assert_eq!(
        summons.len(),
        1,
        "consumed Pit Lord Art must summon one real Pit Lord"
    );
    let pit_lord = Entity::from_bits(summons[0].target.unwrap());
    assert!(
        run.app
            .world()
            .get::<crate::spell_cast::summon::SummonedCreature>(pit_lord)
            .is_some_and(|summon| summon.owner == run.actor && summon.spell_id == 434400),
        "the Pit Lord must be a live native summon of the follower"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 432795),
        "Hand must consume the Art"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, RUINATION_BUFF),
        "Pit Lord summon must grant native Ruination"
    );
}

#[test]
#[ignore = "requires private266 inputs; Mother of Chaos grants Infernal Bolt"]
fn demonology_art_hand_summons_mother_and_grants_infernal_bolt() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    grant_art(&mut run, 432794);
    cast(&mut run, 105174);
    assert_eq!(
        events(&run, CombatLogKind::Summon, 428565).len(),
        1,
        "Mother Art must summon one Mother"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, INFERNAL_BOLT_BUFF),
        "Mother summon must grant native Infernal Bolt"
    );
    let before = crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        shared::components::PowerType::SoulShards,
    )
    .unwrap()
    .0;
    let bolt = crate::auras::overrides::resolve_spell(run.app.world(), run.actor, 686);
    assert_eq!(bolt, 434506, "Infernal Bolt overrides Shadow Bolt");
    cast(&mut run, bolt);
    update(&mut run, 120);
    assert_eq!(
        events(&run, CombatLogKind::Damage, 434506).len(),
        1,
        "Infernal Bolt must deal native damage"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, INFERNAL_BOLT_BUFF),
        "the cast consumes the buff"
    );
    let after = crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        shared::components::PowerType::SoulShards,
    )
    .unwrap()
    .0;
    assert!(
        after > before,
        "Infernal Bolt must generate native Soul Shards: {before} -> {after}"
    );
}

#[test]
#[ignore = "requires private266 inputs; Ruination impact and its Hand meteor"]
fn demonology_ruination_impacts_then_summons_three_hand_imps() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        RUINATION_BUFF,
        Some(run.actor),
        true,
    )
    .expect("native Ruination buff must apply");
    let ruination = crate::auras::overrides::resolve_spell(run.app.world(), run.actor, 105174);
    assert_eq!(ruination, 434635, "Ruination overrides Hand of Gul'dan");
    cast(&mut run, ruination);
    update(&mut run, 120);
    assert_eq!(
        events(&run, CombatLogKind::Damage, 434636).len(),
        1,
        "Ruination impact must deal native damage"
    );
    assert_eq!(
        events(&run, CombatLogKind::Damage, 86040).len(),
        1,
        "Ruination must drop one Hand meteor"
    );
    assert_eq!(
        events(&run, CombatLogKind::Summon, 104317).len(),
        3,
        "Ruination meteor must summon three actual Wild Imps"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, RUINATION_BUFF),
        "Ruination consumes its native buff"
    );
}

/// Damage events from the actual demon each Diabolist summon creates within ten seconds.
fn demon_damage(summon: u32, damage: u32) -> (Run, Vec<i32>) {
    demon_damage_with(summon, damage, true)
}

fn demon_damage_with(summon: u32, damage: u32, ritual: bool) -> (Run, Vec<i32>) {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    if !ritual {
        let actor = run.actor;
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 428514,
        );
    }
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        summon,
        Some(run.actor),
        true,
    )
    .expect("native Diabolist summon must be admitted");
    update(&mut run, 200);
    let summoned: Vec<_> = events(&run, CombatLogKind::Summon, summon)
        .iter()
        .map(|event| event.target.unwrap())
        .collect();
    assert_eq!(summoned.len(), 1, "one actual demon from {summon}");
    let hits: Vec<i32> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(summoned[0])
                && event.spell_id == Some(damage)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| event.amount)
        .collect();
    (run, hits)
}

#[test]
#[ignore = "requires private266 inputs; actual Overlord casts one Wicked Cleave"]
fn demonology_overlord_casts_one_wicked_cleave() {
    // SimC76f8118 pets.cpp2574–2697: one charge, then exactly one Wicked Cleave.
    let (_, hits) = demon_damage(428571, 432120);
    assert_eq!(
        hits.len(),
        1,
        "Overlord must cleave its target exactly once: {hits:?}"
    );
}

#[test]
#[ignore = "requires private266 inputs; actual Mother of Chaos channels Chaos Salvo"]
fn demonology_mother_of_chaos_channels_chaos_salvo() {
    // pets.cpp2717–2790: one channeled Chaos Salvo whose ticks launch native432596.
    let (_, hits) = demon_damage(428565, 432596);
    assert!(
        hits.len() >= 15,
        "Chaos Salvo ticks must hit the target: {}",
        hits.len()
    );
}

#[test]
#[ignore = "requires private266 inputs; actual Pit Lord channels Felseeker"]
fn demonology_pit_lord_channels_felseeker() {
    // pets.cpp2795–2896: one channeled Felseeker of four native434404 ticks.
    let (_, hits) = demon_damage(434400, 434404);
    assert_eq!(hits.len(), 4, "Felseeker must tick four times: {hits:?}");
}

#[test]
#[ignore = "requires private266 inputs; Demonology scales Diabolist demon damage"]
fn demonology_diabolic_ritual_scales_demon_damage() {
    // SimC76f8118 pets.cpp2853–2858: Demonology multiplies demon damage by
    // 1 + native428514 effect3 (-36%).
    let (_, with) = demon_damage_with(434400, 434404, true);
    let (_, without) = demon_damage_with(434400, 434404, false);
    // Crit rolls diverge between runs; compare each run's smallest (noncritical) tick.
    let ratio = *with.iter().min().unwrap() as f64 / *without.iter().min().unwrap() as f64;
    assert!(
        (0.63..=0.65).contains(&ratio),
        "Demonology demon damage ratio {ratio}"
    );
}

#[test]
#[ignore = "requires private266 inputs; Wicked Cleave applies the owner's Cloven Soul"]
fn demonology_wicked_cleave_applies_cloven_soul() {
    // pets.cpp2670–2676: each Wicked Cleave impact triggers owner-owned434424.
    let (run, hits) = demon_damage(428571, 432120);
    assert_eq!(hits.len(), 1);
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .is_some_and(|auras| auras
                .iter()
                .any(|aura| aura.spell_id == 434424 && aura.caster == run.actor)),
        "Cloven Soul must be the owner's debuff on the cleaved target"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs; real Tyrant shortens the ritual"]
fn demonology_infernal_machine_advances_ritual_with_active_tyrant() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 429917));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        actor,
        265187,
        Some(run.targets[0]),
        true,
    )
    .expect("real native Tyrant must spawn");
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 431944, Some(actor), true)
        .expect("native ritual must apply");
    let expiry = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 431944 && a.caster == actor)
            .unwrap()
            .expires_at
            .unwrap()
    };
    let before = expiry(&run);
    let base =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 428514, 0) as f64;
    let bonus = crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 429917, 0)
        as f64
        / 1000.0;
    cast(&mut run, 105174);
    let advanced = before - expiry(&run);
    assert!(
        (advanced - (3.0 * base + bonus)).abs() < 0.06,
        "three paid shards and one native Tyrant bonus: got {advanced}, base {base}, bonus {bonus}"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs; Gloom scales actual Hand impact"]
fn demonology_gloom_of_nathreza_scales_hand_meteor() {
    fn meteor(with_gloom: bool) -> (i32, bool, f32) {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = prepare_funded();
        super::channel_timing_tests::remove_equipped_proc_drivers(&mut run);
        let actor = run.actor;
        let gloom =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 429899, 0);
        assert!(crate::auras::has_aura(run.app.world(), actor, 429899));
        if !with_gloom {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |a| a.spell_id == 429899,
            );
        }
        cast(&mut run, 105174);
        update(&mut run, 30);
        let damage = events(&run, CombatLogKind::Damage, 86040)
            .into_iter()
            .next()
            .expect("actual Hand meteor damage");
        (damage.amount, damage.crit, gloom)
    }
    let (base, base_crit, gloom) = meteor(false);
    let (boosted, boosted_crit, _) = meteor(true);
    assert_eq!(
        base_crit, boosted_crit,
        "same seed must compare the same hit state"
    );
    let expected = 1.0 + 3.0 * gloom / 100.0;
    assert!(
        ((boosted as f32 / base as f32) - expected).abs() < 0.01,
        "native Gloom multiplier {expected}, actual {boosted}/{base}"
    );
}

fn own_stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
                .map(|aura| aura.stacks.max(1))
        })
        .unwrap_or(0)
}

#[test]
#[ignore = "requires private266 inputs; Hand earns Demonic Oculi whose ticks gaze"]
fn demonology_hand_earns_oculi_that_gazes() {
    // SimC76f8118 actions.cpp2840–2841, init.cpp940–958: each Hand adds an
    // Oculi stack; each unhasted tick casts the stack-count Diabolic Gaze.
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    cast(&mut run, 105174);
    assert_eq!(
        own_stacks(&run, 1269643),
        1,
        "one paid Hand earns one Demonic Oculi"
    );
    update(&mut run, 30);
    assert!(
        !events(&run, CombatLogKind::Damage, 1269892).is_empty(),
        "a one-stack Oculi tick must deal native Diabolic Gaze (1)"
    );
}

#[test]
#[ignore = "requires private266 inputs; Art-consuming Hand detonates Oculi"]
fn demonology_art_hand_detonates_oculi_into_minds_eyes() {
    // actions.cpp370–403/5356–5376: an Art-consuming cast explodes every Oculi
    // stack, grants that many Mind's Eyes and expires the Oculi.
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    for _ in 0..2 {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            1269643,
            Some(run.actor),
            true,
        )
        .expect("native Demonic Oculi must apply");
    }
    assert_eq!(own_stacks(&run, 1269643), 2);
    grant_art(&mut run, 428524);
    cast(&mut run, 105174);
    update(&mut run, 2);
    let explosions = events(&run, CombatLogKind::Damage, 1269800);
    assert_eq!(explosions.len(), 1, "one Eye Explosion");
    assert!(
        explosions[0].amount > 0,
        "Eye Explosion must retain its Oculi damage multiplier"
    );
    assert_eq!(
        own_stacks(&run, 1269879),
        3,
        "Hand's third Oculi becomes three Mind's Eyes"
    );
    assert_eq!(
        own_stacks(&run, 1269643),
        0,
        "the explosion expires the Oculi"
    );
}

#[test]
#[ignore = "native Mother buff survives the Hand projectile that granted it"]
fn demonology_infernal_bolt_survives_hand_impact() {
    let mut run = prepare_funded();
    grant_art(&mut run, 432794);
    cast(&mut run, 105174);
    assert!(crate::auras::has_aura(
        run.app.world(),
        run.actor,
        INFERNAL_BOLT_BUFF
    ));
    for _ in 0..120 {
        if !events(&run, CombatLogKind::Damage, 86040).is_empty() {
            break;
        }
        update(&mut run, 1);
    }
    assert!(
        !events(&run, CombatLogKind::Damage, 86040).is_empty(),
        "actual Hand impact"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, INFERNAL_BOLT_BUFF),
        "Hand impact must not consume Infernal Bolt"
    );
    cast(&mut run, 434506);
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, INFERNAL_BOLT_BUFF),
        "Infernal Bolt cast consumes its buff"
    );
}

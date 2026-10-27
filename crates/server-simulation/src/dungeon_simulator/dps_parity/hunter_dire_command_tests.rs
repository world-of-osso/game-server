use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_dire_command_actual_kills_spawn_authentic_beasts() {
    const SUMMON_CONTROL_PET: u32 = 2;
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 378743));
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 385810));
    let dire_frenzy =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 385810, 1)
            as f32
            / 100.0;
    let pet_damage =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 137015, 5)
            as f32
            / 100.0;
    let probability = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        378743,
        0,
    )) / 100.0;
    assert!(probability > 0.0);
    let maximum_attempts =
        (1.0 / crate::random::accumulated_proc_constant(probability)).ceil() as usize;
    let mut query = run.app.world_mut().query::<(
        Entity,
        &crate::spell_cast::summon::SummonedCreature,
        &shared::components::Npc,
    )>();
    let mut beasts = std::collections::BTreeSet::new();
    let mut origins = std::collections::HashMap::new();
    let mut moved_toward_target = false;
    let mut checked_events = 0;
    let mut inherited_ap_hits = Vec::new();
    let mut previous_ap = 0.0;
    let mut previous_mods = std::collections::HashMap::<Entity, crate::auras::AuraStatMods>::new();
    engage(&mut run);
    advance_combat_observed(&mut run, |run| {
        for (unit, summon, npc) in query.iter(run.app.world()) {
            if summon.owner == run.actor && summon.spell_id == 219199 {
                assert_eq!(
                    npc.template_id, 110063,
                    "Dire Command must retain native summon identity"
                );
                assert!(
                    summon.expires_at.is_some(),
                    "native Dire Beast must be finite"
                );
                let position = *run
                    .app
                    .world()
                    .get::<shared::components::Position>(unit)
                    .unwrap();
                let target = *run
                    .app
                    .world()
                    .get::<shared::components::Position>(run.targets[0])
                    .unwrap();
                let distance =
                    ((position.x - target.x).powi(2) + (position.z - target.z).powi(2)).sqrt();
                let original = *origins.entry(unit.to_bits()).or_insert(distance);
                moved_toward_target |= distance + 0.1 < original;
                beasts.insert(unit.to_bits());
            }
        }
        let world = run.app.world();
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[checked_events..] {
            let Some(source) = event.source else { continue };
            if !beasts.contains(&source)
                || event.target != Some(run.targets[0].to_bits())
                || event.spell_id.is_some()
                || event.kind != shared::protocol::CombatLogKind::Damage
                || event.crit
                || event.glancing
                || event.blocked != 0
                || event.absorbed != 0
            {
                continue;
            }
            let Some((pet, summon, _)) = query
                .iter(world)
                .find(|(pet, _, _)| pet.to_bits() == source)
            else {
                continue;
            };
            let stats = world
                .get::<crate::stats::CharacterStats>(run.actor)
                .unwrap();
            let ap = stats.total_attack_power(
                crate::stats::WeaponAttackType::Ranged,
                world.get::<crate::stats::StatModifiers>(run.actor),
            );
            // Combat events can precede an aura recalculation in the same update.
            // Compare damage only when the observed calculation inputs are stable.
            if ap != previous_ap
                || [pet, run.actor, run.targets[0]].into_iter().any(|unit| {
                    previous_mods.get(&unit) != world.get::<crate::auras::AuraStatMods>(unit)
                })
            {
                continue;
            }
            let mods = world.get::<crate::auras::AuraStatMods>(pet);
            let target_mods = world.get::<crate::auras::AuraStatMods>(run.targets[0]);
            let base = ap / 3.5
                * 2.0
                * mods.map_or(1.0, |m| m.autoattack_damage_mult * m.damage_done(1))
                * target_mods.map_or(1.0, |m| m.damage_taken(1))
                * if world.get::<crate::hunter_pet::HunterPet>(pet).is_some()
                    || summon.properties.control == SUMMON_CONTROL_PET
                {
                    world
                        .get::<crate::auras::AuraStatMods>(run.actor)
                        .unwrap()
                        .pet_damage_mult
                } else {
                    world
                        .get::<crate::auras::AuraStatMods>(run.actor)
                        .unwrap()
                        .guardian_damage_mult
                };
            let pet_scaling = crate::level_scaling::UnitScaling::of(world, pet);
            let target_scaling = crate::level_scaling::UnitScaling::of(world, run.targets[0]);
            let armor = target_scaling.armor_against(
                world
                    .get::<crate::level_scaling::CreatureArmor>(run.targets[0])
                    .unwrap()
                    .0,
                &pet_scaling,
            );
            let floor = crate::combat::armor_mitigated(
                base.round(),
                armor,
                &pet_scaling,
                &target_scaling,
                Some(world.resource::<crate::level_scaling::ArmorConstants>()),
                mods,
            )
            .damage
            .floor();
            inherited_ap_hits.push((event.amount, floor, ap));
        }
        checked_events = events.len();
        let stats = world
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap();
        previous_ap = stats.total_attack_power(
            crate::stats::WeaponAttackType::Ranged,
            world.get::<crate::stats::StatModifiers>(run.actor),
        );
        previous_mods.clear();
        for unit in query
            .iter(world)
            .map(|(pet, _, _)| pet)
            .chain([run.actor, run.targets[0]])
        {
            if let Some(mods) = world.get::<crate::auras::AuraStatMods>(unit) {
                previous_mods.insert(unit, mods.clone());
            }
        }
    })
    .expect("ordinary native300s literal combat");
    // SimC76f8118 sc_hunter.cpp1869–1873,1930–1938: genuine Dire
    // Beast auto attacks inherit the owner's full attack power.
    assert!(
        !inherited_ap_hits.is_empty(),
        "fixture needs landed noncritical Dire Beast swings"
    );
    assert!(
        inherited_ap_hits
            .iter()
            .all(|(hit, floor, _)| *hit as f32 >= *floor),
        "Dire Beast swings omit inherited owner AP: {inherited_ap_hits:?}"
    );
    // SimC76f8118 sc_hunter.cpp1877–1882: selected Dire Frenzy's
    // authored second effect multiplies actual DireCritter damage.
    assert!(dire_frenzy > 0.0);
    assert!(
        inherited_ap_hits
            .iter()
            .all(|(hit, floor, _)| *hit as f32 >= (*floor * (1.0 + dire_frenzy)).floor()),
        "Dire Beast swings omit selected Dire Frenzy: {inherited_ap_hits:?}, bonus={dire_frenzy}"
    );
    // SimC76f8118 hunter.cpp1854–1856,1869,1880–1882: every real
    // DireCritter also receives Pet Damage1284992, modified by selected BM.
    assert!(pet_damage > 0.0);
    assert!(
        inherited_ap_hits.iter().all(|(hit, floor, _)| {
            *hit as f32 >= (*floor * (1.0 + dire_frenzy) * (1.0 + pet_damage)).floor()
        }),
        "actual Dire Beast swings must include BM Pet Damage: bonus={pet_damage}, hits={inherited_ap_hits:?}"
    );
    let commands = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(34026)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        })
        .count();
    assert!(
        commands >= maximum_attempts,
        "fixture must execute enough actual KCs to exceed the native PRD guarantee: commands={commands}, guarantee={maximum_attempts}"
    );
    // SimC76f8118 hunter6610–6613/8396: actual successful KC drives the
    // selected native chance's PRD, not an unearned manual summon cast.
    assert!(
        !beasts.is_empty(),
        "actual KC Dire Command must spawn native219199 actors: commands={commands}, beasts={}",
        beasts.len()
    );
    assert!(
        moved_toward_target,
        "a real summoned Beast outside melee range must move toward its target"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(
            |event| event.source.is_some_and(|source| beasts.contains(&source))
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id.is_none()
                && matches!(
                    event.kind,
                    shared::protocol::CombatLogKind::Damage
                        | shared::protocol::CombatLogKind::Miss(_)
                )
        ),
        "actual Dire Beasts must make native white attacks at the real primary"
    );
    assert!(
        events.iter().any(
            |event| event.source.is_some_and(|source| beasts.contains(&source))
                && event.spell_id == Some(1232922)
                && matches!(
                    event.kind,
                    shared::protocol::CombatLogKind::Damage
                        | shared::protocol::CombatLogKind::Miss(_)
                )
        ),
        "existing actual Dire Beasts must receive selected Wildspeaker commands"
    );
}

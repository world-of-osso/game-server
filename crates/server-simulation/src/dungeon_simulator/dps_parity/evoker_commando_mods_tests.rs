//! An actual Commando's Pyre reads its owner's selected passive spell modifiers.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

const PYRE: u32 = 1236970;

fn owner_effect_amount(run: &Run, aura: u32, index: u8) -> f32 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|instance| instance.spell_id == aura)
        .unwrap_or_else(|| panic!("selected owner passive {aura} is active"))
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .amount
}

fn launch_squadron(run: &mut Run) -> Entity {
    let primary = run.targets[0];
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        433874,
        crate::spell_cast::CastTargets {
            unit: run.actor,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    let owner = run.actor;
    for _ in 0..10 {
        run.app.update();
        let pet = run
            .app
            .world_mut()
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(run.app.world())
            .find(|(_, npc, summon)| npc.template_id == 219827 && summon.owner == owner)
            .map(|(entity, _, _)| entity);
        if let Some(pet) = pet {
            return pet;
        }
    }
    panic!("actual Command Squadron must summon a real Commando");
}

#[test]
#[ignore = "requires native Devastation profile and canonical-source Commando templates"]
fn evoker_commando_pyre_reads_owner_passive_spell_modifiers() {
    let _seed = crate::random::seed_scope(1236970);
    let mut run = prepare_literal(1467, 1).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    let refined_essence = owner_effect_amount(&run, 1261452, 0);
    let spec_damage = owner_effect_amount(&run, 356809, 0);
    let spellweaver = owner_effect_amount(&run, 370845, 0);
    let pet = launch_squadron(&mut run);
    run.app.world_mut().insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            pet,
            primary,
            PYRE,
            "native_commando_pyre",
            4,
        ),
    );
    for _ in 0..240 {
        run.app.update();
    }
    let capture = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let hits: Vec<_> = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "result" && event["spell_id"] == PYRE)
        .map(|event| &event["factors"])
        .collect();
    assert!(!hits.is_empty(), "actual Commando flight must cast Pyre");
    // SimC76f8118 player.cpp15268–15272: pets and guardians read their owner's
    // passive spell values; native family masks select Pyre for both talents.
    let damage = (1.0 + refined_essence / 100.0) * (1.0 + spec_damage / 100.0);
    let critical_bonus = 1.0 + spellweaver / 100.0;
    for hit in &hits {
        let modded = hit["native_spell_mod_product"].as_f64().unwrap() as f32;
        assert!(
            (modded - damage).abs() < 1e-3,
            "Pyre must apply the owner's passive damage modifiers: {hit}"
        );
    }
    // Inherited-chance crits also read Spellweaver's Dominance through the same owner.
    for hit in hits.iter().filter(|hit| hit["result"] == "crit") {
        let bonus = hit["crit_bonus"].as_f64().unwrap() as f32;
        assert!(
            (bonus - critical_bonus).abs() < 1e-3,
            "Pyre crits must apply the owner's passive critical bonus: {hit}"
        );
    }
}

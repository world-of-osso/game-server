use super::*;

fn stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 1281559 && aura.caster == run.actor)
        })
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires canonical 266 inputs; real Hellbent Imp heartbeat"]
fn demonology_hellbent_commander_counts_actual_wild_imps() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 1250897));
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 267216,
    );
    for _ in 0..3 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 104317, Some(actor), true)
            .unwrap();
    }
    assert_eq!(stacks(&run), 0, "Wild Imps do not count on arise");
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), actor, target);
    for tick in 0..120 {
        if tick % 60 == 0 {
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 686, Some(target), false)
                .expect("real Shadow Bolt keeps combat active");
        }
        run.app.update();
    }
    let living = run
        .app
        .world_mut()
        .query::<(
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(run.app.world())
        .filter(|(summon, npc)| summon.owner == actor && npc.template_id == 55659)
        .count();
    assert_eq!(living, 3);
    assert_eq!(
        usize::from(stacks(&run)),
        living,
        "heartbeat counts the actual living Imps"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs; real Hellbent Imp demise"]
fn demonology_hellbent_hand_imp_demise_removes_two_stacks() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let actor = run.actor;
    for _ in 0..3 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1281559, Some(actor), true)
            .unwrap();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 104317, Some(actor), true)
        .unwrap();
    let imp = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == actor && summon.spell_id == 104317)
        .map(|(entity, _)| entity)
        .expect("actual Hand Imp");
    crate::spell_cast::summon::despawn_summon(run.app.world_mut(), imp);
    assert_eq!(
        stacks(&run),
        1,
        "normal Hand Imp demise removes two real stacks"
    );
}

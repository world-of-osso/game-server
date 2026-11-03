//! A keystone follower party casts the raid buffs its classes provide, as players do.
use super::*;
use server_follower_apl::{FollowerApl, FollowerAplFailed, party_buffs::PARTY_BUFFS};
use shared::protocol::CombatLogKind;
use std::collections::BTreeSet;

const MARK_OF_THE_WILD: u32 = 1126;
const BRONZE: u32 = 364342;
const REGROWTH: u32 = 8936;
const POLYMORPH: u32 = 118;
/// Blessing of the Bronze grants one class-specific aura per recipient.
const BRONZE_RESULTS: [u32; 13] = [
    381732, 381741, 381746, 381748, 381749, 381750, 381751, 381752, 381753, 381754, 381756, 381757,
    381758,
];

fn spawn_vale(specs: [u32; 5]) -> Run {
    let _rng = crate::random::seed_scope(1);
    spawn_run_mode(
        1,
        &mut None,
        Some(super::super::keystone::Options {
            map_id: 2859,
            level: 2,
            specs,
        }),
    )
    .unwrap()
}

fn party(run: &Run) -> Vec<Entity> {
    std::iter::once(run.party.owner)
        .chain(run.party.members.iter().copied())
        .collect()
}

/// Frames without route control, so no pull starts; every APL must stay healthy.
fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
        for unit in party(run) {
            assert!(
                run.app.world().get::<FollowerAplFailed>(unit).is_none(),
                "{unit:?} APL failed"
            );
        }
    }
}

fn holds(world: &World, unit: Entity, buff: u32) -> bool {
    if buff == BRONZE {
        BRONZE_RESULTS
            .iter()
            .any(|&aura| crate::auras::has_aura(world, unit, aura))
    } else {
        crate::auras::has_aura(world, unit, buff)
    }
}

fn remove_buff(run: &mut Run, unit: Entity, buff: u32) {
    let removed = crate::auras::remove_auras(
        run.app.world_mut(),
        unit,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == buff || (buff == BRONZE && BRONZE_RESULTS.contains(&aura.spell_id)),
    );
    assert!(!removed.is_empty(), "{unit:?} held buff{buff} to lose");
}

/// Party buffs some party unit knows: the composition's provided set.
fn provided(run: &Run) -> BTreeSet<u32> {
    let world = run.app.world();
    PARTY_BUFFS
        .into_iter()
        .filter(|&spell| {
            party(run).into_iter().any(|unit| {
                world
                    .get::<crate::class_progression::KnownSpells>(unit)
                    .is_some_and(|known| known.contains(spell))
            })
        })
        .collect()
}

/// Successful party-buff casts after `cursor`, as (spell, caster).
fn buff_casts(run: &Run, cursor: usize) -> Vec<(u32, u64)> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..]
        .iter()
        .filter(|event| event.kind == CombatLogKind::CastSuccess)
        .filter_map(|event| {
            let spell = event.spell_id.filter(|spell| PARTY_BUFFS.contains(spell))?;
            Some((spell, event.source?))
        })
        .collect()
}

fn cursor(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len()
}

fn in_combat_units(run: &Run) -> Vec<Entity> {
    party(run)
        .into_iter()
        .filter(|&unit| {
            run.app
                .world()
                .get::<shared::components::CombatStatus>(unit)
                .is_some_and(|status| status.0)
        })
        .collect()
}

fn count(casts: &[(u32, u64)], spell: u32) -> usize {
    casts.iter().filter(|(cast, _)| *cast == spell).count()
}

/// Precombat and the out-of-combat step finish; every unit holds exactly the provided set.
fn assert_settled_with(run: &mut Run, expected: &[u32]) {
    advance(run, 800);
    let provided = provided(run);
    assert_eq!(provided, expected.iter().copied().collect());
    let world = run.app.world();
    for unit in party(run) {
        assert!(
            world
                .get::<FollowerApl>(unit)
                .is_some_and(|apl| !apl.in_combat()),
            "{unit:?} finished precombat out of combat"
        );
        for spell in PARTY_BUFFS {
            assert_eq!(
                holds(world, unit, spell),
                provided.contains(&spell),
                "{unit:?} buff{spell}"
            );
        }
    }
    let casts = buff_casts(run, 0);
    for spell in PARTY_BUFFS {
        assert_eq!(
            count(&casts, spell),
            usize::from(provided.contains(&spell)),
            "buff{spell} casts {casts:?}"
        );
    }
}

#[test]
#[ignore = "requires Blinding Vale graph and navmesh"]
fn vale_party_gets_every_buff_its_classes_provide() {
    let mut run = spawn_vale([104, 1468, 265, 70, 254]);
    assert_settled_with(&mut run, &[MARK_OF_THE_WILD, BRONZE]);
}

#[test]
#[ignore = "requires Blinding Vale graph and navmesh"]
fn five_class_party_casts_each_buff_once() {
    let mut run = spawn_vale([73, 256, 63, 262, 1467]);
    // Arcane Intellect comes from the Mage's own precombat row, never twice.
    assert_settled_with(&mut run, &[1459, 6673, 21562, 462854, BRONZE]);
}

#[test]
#[ignore = "requires Blinding Vale graph and navmesh"]
fn lowest_entity_druid_alone_restores_mark_of_the_wild() {
    // Three druids: Guardian, Restoration and the Balance owner.
    let mut run = spawn_vale([104, 105, 265, 70, 102]);
    advance(&mut run, 800);
    let druids: Vec<_> = party(&run)
        .into_iter()
        .filter(|&unit| {
            run.app
                .world()
                .get::<crate::class_progression::KnownSpells>(unit)
                .is_some_and(|known| known.contains(MARK_OF_THE_WILD))
        })
        .collect();
    assert_eq!(druids.len(), 3);
    let chosen = *druids.iter().min_by_key(|unit| unit.to_bits()).unwrap();
    let marks = |run: &Run, cursor: usize| -> Vec<_> {
        buff_casts(run, cursor)
            .into_iter()
            .filter(|(spell, _)| *spell == MARK_OF_THE_WILD)
            .collect()
    };
    assert_eq!(marks(&run, 0), [(MARK_OF_THE_WILD, chosen.to_bits())]);
    // Once the chosen druid starts Regrowth, the other two must not take over.
    let member = run.party.members[2];
    for _ in 0..200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            chosen,
            REGROWTH,
            Some(member),
            false,
        ) {
            Ok(()) => break,
            Err(
                crate::spell_cast::CastFailure::InProgress
                | crate::spell_cast::CastFailure::OnGlobalCooldown,
            ) => advance(&mut run, 1),
            Err(error) => panic!("Regrowth failed: {error:?}"),
        }
    }
    assert_eq!(
        run.app
            .world()
            .get::<shared::casting::CastState>(chosen)
            .map(|cast| cast.spell_id),
        Some(REGROWTH)
    );
    remove_buff(&mut run, member, MARK_OF_THE_WILD);
    let start = cursor(&run);
    for _ in 0..200 {
        if holds(run.app.world(), member, MARK_OF_THE_WILD) {
            break;
        }
        advance(&mut run, 1);
    }
    advance(&mut run, 40);
    assert!(holds(run.app.world(), member, MARK_OF_THE_WILD));
    assert_eq!(marks(&run, start), [(MARK_OF_THE_WILD, chosen.to_bits())]);
}

#[test]
#[ignore = "requires Blinding Vale graph and navmesh"]
fn buffs_are_recast_once_after_death_and_resurrection() {
    let mut run = spawn_vale([104, 1468, 265, 70, 254]);
    assert_settled_with(&mut run, &[MARK_OF_THE_WILD, BRONZE]);
    let victim = run.party.members[2];
    crate::combat::kill_unit(run.app.world_mut(), victim, victim);
    advance(&mut run, 2);
    for spell in [MARK_OF_THE_WILD, BRONZE] {
        assert!(
            !holds(run.app.world(), victim, spell),
            "death strips buff{spell}"
        );
    }
    let start = cursor(&run);
    // The healer's native out-of-combat resurrection (followers/release.rs).
    let mut resurrected = false;
    for _ in 0..2400 {
        advance(&mut run, 1);
        if crate::spell_cast::targets::is_alive(run.app.world(), victim) {
            resurrected = true;
            break;
        }
        assert!(
            buff_casts(&run, start).is_empty(),
            "nothing to restore while the member is dead"
        );
    }
    assert!(
        resurrected,
        "the dead follower is resurrected out of combat"
    );
    // The healer tops the resurrected member up before it buffs again.
    for _ in 0..1200 {
        if [MARK_OF_THE_WILD, BRONZE]
            .into_iter()
            .all(|spell| holds(run.app.world(), victim, spell))
        {
            break;
        }
        advance(&mut run, 1);
    }
    advance(&mut run, 200);
    let casts = buff_casts(&run, start);
    for spell in [MARK_OF_THE_WILD, BRONZE] {
        assert!(
            holds(run.app.world(), victim, spell),
            "buff{spell} restored"
        );
        assert_eq!(
            count(&casts, spell),
            1,
            "buff{spell} recast once: {casts:?}"
        );
    }
}

#[test]
#[ignore = "requires Blinding Vale graph and navmesh"]
fn lost_buffs_wait_for_combat_to_end() {
    let mut run = spawn_vale([104, 1468, 265, 70, 254]);
    assert_settled_with(&mut run, &[MARK_OF_THE_WILD, BRONZE]);
    let dummy = crate::dungeon_simulator::dps_parity::spawn_level_dummy(
        &mut run.app,
        run.party.owner,
        90,
        0,
    )
    .unwrap();
    for unit in party(&run) {
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), unit, dummy);
    }
    // A polymorphed enemy keeps the party in combat without an engaged target,
    // so each APL leaves its combat phase and reaches out-of-combat upkeep.
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.party.owner,
            target: dummy,
            spell_id: POLYMORPH,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(run.app.world(), dummy, POLYMORPH));
    let member = run.party.members[2];
    remove_buff(&mut run, member, MARK_OF_THE_WILD);
    remove_buff(&mut run, member, BRONZE);
    let start = cursor(&run);
    // Frames where the whole party is in combat and every APL runs upkeep.
    let mut upkeep_in_combat = 0;
    for frame in 0..1200 {
        if frame == 200 {
            crate::combat::kill_unit(run.app.world_mut(), dummy, dummy);
        }
        let in_combat = in_combat_units(&run);
        if frame > 200 && in_combat.is_empty() {
            break;
        }
        let before = cursor(&run);
        advance(&mut run, 1);
        for (spell, caster) in buff_casts(&run, before) {
            assert!(
                !in_combat.contains(&Entity::from_bits(caster)),
                "buff{spell} cast by an in-combat unit"
            );
        }
        let world = run.app.world();
        if in_combat.len() == party(&run).len()
            && party(&run).into_iter().all(|unit| {
                world
                    .get::<FollowerApl>(unit)
                    .is_some_and(|apl| !apl.in_combat())
            })
        {
            upkeep_in_combat += 1;
        }
    }
    assert!(in_combat_units(&run).is_empty(), "the party leaves combat");
    assert!(
        upkeep_in_combat >= 100,
        "in combat without an engaged target for {upkeep_in_combat} frames"
    );
    advance(&mut run, 200);
    let casts = buff_casts(&run, start);
    for spell in [MARK_OF_THE_WILD, BRONZE] {
        assert!(holds(run.app.world(), member, spell));
        assert_eq!(
            count(&casts, spell),
            1,
            "buff{spell} recast once: {casts:?}"
        );
    }
}

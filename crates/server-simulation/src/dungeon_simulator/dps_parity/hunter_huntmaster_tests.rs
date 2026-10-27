use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_huntmaster_earns_authentic_companion_from_real_beast_summons() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 459730),
        "selected Huntmaster Call"
    );
    let cap = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(459731)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura as usize;
    let mut summons = run.app.world_mut().query::<(
        Entity,
        &crate::spell_cast::summon::SummonedCreature,
        &shared::components::Npc,
    )>();
    let mut contributors = std::collections::BTreeSet::new();
    let mut companions = std::collections::BTreeSet::new();
    let mut fenryrs = std::collections::BTreeSet::new();
    engage(&mut run);
    advance_combat_observed(&mut run, |run| {
        for (actor, summon, npc) in summons.iter(run.app.world()) {
            if summon.owner != run.actor {
                continue;
            }
            if matches!(summon.spell_id, 219199 | 471990) {
                contributors.insert(actor.to_bits());
            }
            if let Some((entry, buff)) = match summon.spell_id {
                459733 => Some((228224, 459735)),
                459739 => Some((228226, 459738)),
                _ => None,
            } {
                assert_eq!(npc.template_id, entry, "native summon identity");
                let owner_aura = run
                    .app
                    .world()
                    .get::<crate::auras::ActiveAuras>(run.actor)
                    .unwrap()
                    .iter()
                    .find(|aura| aura.spell_id == buff && aura.caster == run.actor)
                    .expect("earned owner companion buff");
                let expiry = summon
                    .expires_at
                    .expect("source companion duration is finite");
                assert!(
                    (expiry - owner_aura.expires_at.unwrap()).abs() < 0.002,
                    "real companion lifetime follows native owner buff: actor={actor:?}, spell={}, now={}, companion_expiry={expiry}, buff_expiry={:?}, buff_duration_ms={}",
                    summon.spell_id, crate::spell_cast::now(run.app.world()), owner_aura.expires_at, owner_aura.duration_ms
                );
                companions.insert(actor.to_bits());
                if summon.spell_id == 459733 {
                    fenryrs.insert(actor.to_bits());
                }
            }
        }
    })
    .expect("ordinary native300s combat");
    assert!(
        cap > 0 && contributors.len() >= cap,
        "fixture must earn at least the native counter cap through real summons: cap={cap}, contributors={}",
        contributors.len()
    );
    // SimC76f8118 hunter3715–3734: each actual beast fills the native counter,
    // then one of two authentic actors and its owner buff are earned.
    assert!(
        !companions.is_empty(),
        "real beast summons must earn Fenryr or Hati: contributors={}",
        contributors.len()
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event
            .source
            .is_some_and(|source| companions.contains(&source))
            && event.target == Some(run.targets[0].to_bits())
            && event.spell_id.is_none()
            && matches!(
                event.kind,
                shared::protocol::CombatLogKind::Damage | shared::protocol::CombatLogKind::Miss(_)
            )),
        "actual Huntmaster companion must make white attacks against the actual primary"
    );
    if !fenryrs.is_empty() {
        assert!(
            events.iter().any(
                |event| event.source.is_some_and(|source| fenryrs.contains(&source))
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(459753)
            ),
            "actual Fenryr must execute its native Ravenous Leap"
        );
    }
}

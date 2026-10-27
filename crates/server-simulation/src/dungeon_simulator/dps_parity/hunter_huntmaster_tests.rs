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
                    "real companion lifetime follows native owner buff"
                );
                companions.insert(actor.to_bits());
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
}

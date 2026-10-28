use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;
use std::sync::{Arc, Mutex};

struct TargetFactsProbe(Arc<Mutex<Vec<(f64, f64)>>>);

impl SpecDriver for TargetFactsProbe {
    fn label(&self) -> &'static str {
        "tank_fixture_target_facts_probe"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let bindings = Bindings::default();
        let facts = NativeFacts::new(
            world,
            actor,
            &bindings,
            Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: FightStyle::Patchwerk,
            },
            targets.len(),
        )
        .with_targets(targets.first().copied(), targets);
        let remaining = facts.value("target.time_to_die")?;
        let boss = facts.value("target.is_boss")?;
        self.0.lock().unwrap().push((remaining, boss));
        Ok(())
    }
}

#[test]
#[ignore = "uses the real MID2 player and tank replay target"]
fn tank_replay_drives_target_lifetime_and_boss_facts() {
    let mut run = prepare_registered_player(66, 1).unwrap();
    let observations = Arc::new(Mutex::new(Vec::new()));
    run.spec_driver = Some(Box::new(TargetFactsProbe(observations.clone())));
    super::super::tank_replay::replay_registered_run(run, &[], 2, &[]).unwrap();
    let observations = observations.lock().unwrap();
    let first = observations.first().unwrap();
    let last = observations.last().unwrap();
    assert!((first.0 - 300.0).abs() < 0.1, "{first:?}");
    assert!((last.0 - 298.05).abs() < 0.1, "{last:?}");
    assert!(observations.iter().all(|(_, boss)| *boss == 1.0));
}

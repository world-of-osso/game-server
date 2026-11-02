use super::*;
use std::sync::{Arc, Mutex};

struct Probe(Arc<Mutex<Vec<&'static str>>>);

impl SpecRotation for Probe {
    fn action_spells(&self) -> Vec<u32> {
        vec![]
    }
    fn poll_precombat(
        &mut self,
        _: &mut World,
        _: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        assert!(targets.is_empty(), "precombat must not run during combat");
        self.0.lock().unwrap().push("precombat");
        Ok(PrecombatPoll::Finished)
    }
    fn execute(&mut self, _: &mut World, _: Entity, _: Action, _: Execution) -> Result<(), String> {
        unreachable!()
    }
    fn start_combat_at(&mut self, _: f64) {
        self.0.lock().unwrap().push("pull");
    }
    fn drive(&mut self, _: &mut World, _: Entity, _: &[Entity]) -> Result<(), String> {
        self.0.lock().unwrap().push("combat");
        Ok(())
    }
}

#[test]
fn same_rotation_runs_three_pulls_without_precombat_during_combat() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    let actor = world.spawn_empty().id();
    let target = world.spawn_empty().id();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut apl = FollowerApl::new(Box::new(Probe(log.clone())));
    apl.tick(&mut world, actor, &[]).unwrap();
    for _ in 0..3 {
        apl.tick(&mut world, actor, &[target]).unwrap();
        apl.tick(&mut world, actor, &[]).unwrap();
    }
    assert_eq!(
        *log.lock().unwrap(),
        [
            "precombat",
            "pull",
            "combat",
            "pull",
            "combat",
            "pull",
            "combat"
        ]
    );
}

#[test]
fn early_pull_skips_unfinished_precombat() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    let actor = world.spawn_empty().id();
    let target = world.spawn_empty().id();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut apl = FollowerApl::new(Box::new(Probe(log.clone())));
    apl.tick(&mut world, actor, &[target]).unwrap();
    assert_eq!(*log.lock().unwrap(), ["pull", "combat"]);
}

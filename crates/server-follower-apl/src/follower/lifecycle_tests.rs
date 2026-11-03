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

struct EncounterProbe {
    encounter: crate::simc_apl_native::Encounter,
    values: Arc<Mutex<Vec<f64>>>,
}

impl SpecRotation for EncounterProbe {
    fn action_spells(&self) -> Vec<u32> {
        vec![]
    }
    fn poll_precombat(
        &mut self,
        _: &mut World,
        _: Entity,
        _: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        Ok(PrecombatPoll::Finished)
    }
    fn execute(&mut self, _: &mut World, _: Entity, _: Action, _: Execution) -> Result<(), String> {
        unreachable!()
    }
    fn start_combat_at(&mut self, at: f64) {
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, at);
    }
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        use server::simc_apl::Facts;
        let bindings = crate::simc_apl_native::Bindings::default();
        let facts = crate::simc_apl_native::NativeFacts::new(
            world,
            actor,
            &bindings,
            self.encounter,
            targets.len(),
        )
        .with_targets(targets.first().copied(), targets);
        self.values
            .lock()
            .unwrap()
            .push(facts.value("target.time_to_die")?);
        Ok(())
    }
}

fn second_pull_ttd(first_pull_seconds: u64) -> f64 {
    use shared::components::Health;
    use std::time::Duration;
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    let actor = world.spawn_empty().id();
    let target = world
        .spawn(Health {
            current: 100.0,
            max: 100.0,
        })
        .id();
    let values = Arc::new(Mutex::new(Vec::new()));
    let mut apl = FollowerApl::new(Box::new(EncounterProbe {
        encounter: crate::simc_apl_native::Encounter::dungeon(0.0),
        values: values.clone(),
    }));
    apl.tick(&mut world, actor, &[target]).unwrap();
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(first_pull_seconds));
    apl.tick(&mut world, actor, &[]).unwrap();
    apl.tick(&mut world, actor, &[target]).unwrap();
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(5));
    world.get_mut::<Health>(target).unwrap().current = 50.0;
    apl.tick(&mut world, actor, &[target]).unwrap();
    let result = *values.lock().unwrap().last().unwrap();
    result
}

#[test]
fn second_pull_ttd_is_independent_of_first_pull_duration() {
    assert_eq!(second_pull_ttd(10), 5.0);
    assert_eq!(second_pull_ttd(100), 5.0);
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

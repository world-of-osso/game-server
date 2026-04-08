use super::*;
use std::collections::HashSet;

fn sample_dungeon() -> InstanceTemplate {
    dungeon_template(33, "Shadowfang Keep", 0)
}

fn sample_raid() -> InstanceTemplate {
    raid_template(409, "Molten Core", 230)
}

fn sample_registry() -> InstanceTemplateRegistry {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(sample_dungeon()).unwrap();
    reg.add(sample_raid()).unwrap();
    reg
}

fn base_creature() -> CreatureBaseStats {
    CreatureBaseStats {
        health: 10000.0,
        damage_min: 100.0,
        damage_max: 200.0,
    }
}

#[path = "instance_tests/config.rs"]
mod config;
#[path = "instance_tests/isolation.rs"]
mod isolation;
#[path = "instance_tests/manager.rs"]
mod manager;
#[path = "instance_tests/registry.rs"]
mod registry;
#[path = "instance_tests/runtime.rs"]
mod runtime;
#[path = "instance_tests/scaling.rs"]
mod scaling;

include!("instance_reset_tests.rs");

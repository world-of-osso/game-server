use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::profession::{
    CraftMaterial, CraftResult, GatherNode, GatherResult, PlayerProfessions, Profession, Recipe,
    attempt_craft, attempt_gather,
};
use shared::protocol::{
    CraftProfessionRecipe, GatherProfessionNode, ProfessionChannel, ProfessionRecipeSnapshot,
    ProfessionSkillSnapshot, ProfessionSnapshot, ProfessionStateUpdate, QueryProfessions,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase, InventoryData, InventoryItemData};

pub(crate) const NODE_COPPER_VEIN: u32 = 1;
pub(crate) const RECIPE_COPPER_BRACERS: u32 = 5001;
const GATHER_CAST_SECS: f32 = 1.5;
const COPPER_ORE_ID: u32 = 2770;
const COPPER_BRACERS_ID: u32 = 2851;

struct GatherNodeDef {
    id: u32,
    name: &'static str,
    cast_secs: f32,
    profession: Profession,
    required_skill: u16,
    trivial_skill: u16,
    yield_item_id: u32,
    yield_name: &'static str,
    min_count: u16,
    max_count: u16,
    vendor_sell_price: u32,
}

struct ProfessionRecipeDef {
    recipe_id: u32,
    name: &'static str,
    profession: Profession,
    required_skill: u16,
    trivial_skill: u16,
    material_item_id: u32,
    material_name: &'static str,
    material_count: u16,
    output_item_id: u32,
    output_name: &'static str,
    output_count: u16,
    output_vendor_sell_price: u32,
}

#[derive(Component, Debug)]
struct PendingGatherCast {
    conn_entity: Entity,
    character_id: u64,
    node_id: u32,
    remaining_secs: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProfessionActionSuccess {
    message: String,
    skill_up: Option<ProfessionSkillSnapshot>,
}

pub struct ProfessionPlugin;

impl Plugin for ProfessionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, process_profession_queries);
        app.add_systems(Update, process_craft_requests);
        app.add_systems(Update, process_gather_requests);
        app.add_systems(Update, tick_pending_gather_casts);
    }
}

fn process_profession_queries(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<QueryProfessions>,
        ),
        With<Authenticated>,
    >,
    professions: Query<&PlayerProfessions>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<ProfessionStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok(professions) = professions.get(link.0) else {
                send_profession_error(
                    conn_entity,
                    in_world.character_id,
                    db.as_ref(),
                    None,
                    &mut senders,
                    "professions unavailable for player",
                );
                continue;
            };
            let inventory = persistence::load_inventory(&db.db, in_world.character_id);
            send_profession_update(
                conn_entity,
                &build_profession_snapshot(professions, &inventory),
                None,
                None,
                None,
                &mut senders,
            );
        }
    }
}

fn process_craft_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<CraftProfessionRecipe>,
        ),
        With<Authenticated>,
    >,
    mut professions: Query<&mut PlayerProfessions>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<ProfessionStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let Ok(mut profs) = professions.get_mut(link.0) else {
                send_profession_error(
                    conn_entity,
                    in_world.character_id,
                    db.as_ref(),
                    None,
                    &mut senders,
                    "professions unavailable for player",
                );
                continue;
            };
            let mut inventory = persistence::load_inventory(&db.db, in_world.character_id);
            match craft_recipe(
                request.recipe_id,
                &mut profs,
                &mut inventory,
                &db.db,
                rand::random::<f32>(),
            ) {
                Ok(result) => {
                    persistence::save_inventory(&db.db, in_world.character_id, &inventory);
                    persistence::save_professions(&db.db, in_world.character_id, &profs);
                    send_profession_update(
                        conn_entity,
                        &build_profession_snapshot(&profs, &inventory),
                        Some(result.message),
                        result.skill_up,
                        None,
                        &mut senders,
                    );
                }
                Err(error) => {
                    send_profession_error(
                        conn_entity,
                        in_world.character_id,
                        db.as_ref(),
                        Some(&profs),
                        &mut senders,
                        &error,
                    );
                }
            }
        }
    }
}

fn process_gather_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<GatherProfessionNode>,
        ),
        With<Authenticated>,
    >,
    pending: Query<(), With<PendingGatherCast>>,
    professions: Query<&PlayerProfessions>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<ProfessionStateUpdate>>,
    mut commands: Commands,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let Ok(profs) = professions.get(link.0) else {
                send_profession_error(
                    conn_entity,
                    in_world.character_id,
                    db.as_ref(),
                    None,
                    &mut senders,
                    "professions unavailable for player",
                );
                continue;
            };
            let Some(node) = gather_node_def(request.node_id) else {
                send_profession_error(
                    conn_entity,
                    in_world.character_id,
                    db.as_ref(),
                    Some(profs),
                    &mut senders,
                    "unknown gather node",
                );
                continue;
            };
            if pending.contains(link.0) {
                send_profession_error(
                    conn_entity,
                    in_world.character_id,
                    db.as_ref(),
                    Some(profs),
                    &mut senders,
                    "already gathering",
                );
                continue;
            }
            commands.entity(link.0).insert(PendingGatherCast {
                conn_entity,
                character_id: in_world.character_id,
                node_id: node.id,
                remaining_secs: node.cast_secs,
            });
        }
    }
}

fn tick_pending_gather_casts(
    time: Res<Time>,
    mut pending_casts: Query<(Entity, &mut PendingGatherCast, &mut PlayerProfessions)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<ProfessionStateUpdate>>,
    mut commands: Commands,
) {
    let Some(db) = db else { return };
    for (player_entity, mut pending, mut professions) in &mut pending_casts {
        pending.remaining_secs -= time.delta_secs();
        if pending.remaining_secs > 0.0 {
            continue;
        }
        resolve_completed_gather(&pending, &mut professions, db.as_ref(), &mut senders);
        commands.entity(player_entity).remove::<PendingGatherCast>();
    }
}

fn resolve_completed_gather(
    pending: &PendingGatherCast,
    professions: &mut PlayerProfessions,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<ProfessionStateUpdate>>,
) {
    let mut inventory = persistence::load_inventory(&db.db, pending.character_id);
    let outcome = complete_gather(
        pending.node_id,
        professions,
        &mut inventory,
        &db.db,
        rand::random::<f32>(),
        &[rand::random::<f32>()],
    );
    match outcome {
        Ok(result) => {
            persistence::save_inventory(&db.db, pending.character_id, &inventory);
            persistence::save_professions(&db.db, pending.character_id, professions);
            send_profession_update(
                pending.conn_entity,
                &build_profession_snapshot(professions, &inventory),
                Some(result.message),
                result.skill_up,
                None,
                senders,
            );
        }
        Err(error) => {
            send_profession_error(
                pending.conn_entity,
                pending.character_id,
                db,
                Some(professions),
                senders,
                &error,
            );
        }
    }
}

fn send_profession_update(
    conn_entity: Entity,
    snapshot: &ProfessionSnapshot,
    message: Option<String>,
    skill_up: Option<ProfessionSkillSnapshot>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<ProfessionStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<ProfessionChannel>(ProfessionStateUpdate {
        snapshot: Some(snapshot.clone()),
        message,
        skill_up,
        error,
    });
}

fn send_profession_error(
    conn_entity: Entity,
    character_id: u64,
    db: &GameDatabase,
    professions: Option<&PlayerProfessions>,
    senders: &mut Query<&mut MessageSender<ProfessionStateUpdate>>,
    error: &str,
) {
    let snapshot = professions.map(|profs| {
        let inventory = persistence::load_inventory(&db.db, character_id);
        build_profession_snapshot(profs, &inventory)
    });
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<ProfessionChannel>(ProfessionStateUpdate {
        snapshot,
        message: None,
        skill_up: None,
        error: Some(error.into()),
    });
}

fn build_profession_snapshot(
    professions: &PlayerProfessions,
    inventory: &InventoryData,
) -> ProfessionSnapshot {
    ProfessionSnapshot {
        skills: professions
            .skills
            .iter()
            .map(|skill| ProfessionSkillSnapshot {
                profession: profession_name(skill.profession).into(),
                current: skill.current,
                max: skill.max,
            })
            .collect(),
        recipes: recipe_defs()
            .iter()
            .filter(|def| professions.knows(def.profession))
            .map(|def| ProfessionRecipeSnapshot {
                spell_id: def.recipe_id,
                profession: profession_name(def.profession).into(),
                name: def.name.into(),
                craftable: recipe_is_craftable(def, professions, inventory),
                cooldown: None,
            })
            .collect(),
    }
}

fn recipe_is_craftable(
    def: &ProfessionRecipeDef,
    professions: &PlayerProfessions,
    inventory: &InventoryData,
) -> bool {
    matches!(
        attempt_craft(
            &recipe_from_def(def),
            &mut professions.clone(),
            &|item_id| { count_inventory_item(inventory, item_id) },
            1.0
        ),
        CraftResult::Success { .. }
    )
}

fn craft_recipe(
    recipe_id: u32,
    professions: &mut PlayerProfessions,
    inventory: &mut InventoryData,
    db: &Database,
    roll: f32,
) -> Result<ProfessionActionSuccess, String> {
    let Some(def) = recipe_def(recipe_id) else {
        return Err("unknown profession recipe".into());
    };
    match attempt_craft(
        &recipe_from_def(def),
        professions,
        &|item_id| count_inventory_item(inventory, item_id),
        roll,
    ) {
        CraftResult::Success {
            output_item_id,
            output_count,
            skilled_up,
        } => {
            consume_recipe_materials(inventory, def)?;
            add_inventory_item(
                inventory,
                db,
                output_item_id,
                def.output_name,
                u32::from(output_count),
                def.output_vendor_sell_price,
            );
            Ok(ProfessionActionSuccess {
                message: format!("crafted {}", def.output_name),
                skill_up: skill_up_snapshot(professions, def.profession, skilled_up),
            })
        }
        CraftResult::WrongProfession => Err("profession not learned".into()),
        CraftResult::SkillTooLow => Err("profession skill is too low".into()),
        CraftResult::MissingMaterials { missing } => Err(format_missing_materials(&missing)),
    }
}

fn complete_gather(
    node_id: u32,
    professions: &mut PlayerProfessions,
    inventory: &mut InventoryData,
    db: &Database,
    roll: f32,
    count_rolls: &[f32],
) -> Result<ProfessionActionSuccess, String> {
    let Some(def) = gather_node_def(node_id) else {
        return Err("unknown gather node".into());
    };
    match attempt_gather(&gather_node_from_def(def), professions, roll, count_rolls) {
        GatherResult::Success { items, skilled_up } => {
            for (item_id, count) in items {
                add_inventory_item(
                    inventory,
                    db,
                    item_id,
                    def.yield_name,
                    u32::from(count),
                    def.vendor_sell_price,
                );
            }
            Ok(ProfessionActionSuccess {
                message: format!("gathered {}", def.name),
                skill_up: skill_up_snapshot(professions, def.profession, skilled_up),
            })
        }
        GatherResult::SkillTooLow => Err("gathering skill is too low".into()),
        GatherResult::WrongProfession => Err("profession not learned".into()),
    }
}

fn consume_recipe_materials(
    inventory: &mut InventoryData,
    def: &ProfessionRecipeDef,
) -> Result<(), String> {
    let Some(index) = inventory.items.iter().position(|item| {
        item.item_id == def.material_item_id && item.stack_count >= u32::from(def.material_count)
    }) else {
        return Err(format!(
            "missing materials: {} x{}",
            def.material_name, def.material_count
        ));
    };
    inventory.items[index].stack_count -= u32::from(def.material_count);
    if inventory.items[index].stack_count == 0 {
        inventory.items.remove(index);
    }
    Ok(())
}

fn add_inventory_item(
    inventory: &mut InventoryData,
    db: &Database,
    item_id: u32,
    name: &str,
    count: u32,
    vendor_sell_price: u32,
) {
    if let Some(existing) = inventory
        .items
        .iter_mut()
        .find(|item| item.item_id == item_id)
    {
        existing.stack_count = existing.stack_count.saturating_add(count);
        return;
    }
    inventory.items.push(InventoryItemData {
        item_guid: persistence::next_id(db, "item_instance"),
        item_id,
        name: name.into(),
        quality: 1,
        required_level: 1,
        stack_count: count,
        vendor_sell_price,
        unique_auction: false,
    });
}

fn count_inventory_item(inventory: &InventoryData, item_id: u32) -> u16 {
    inventory
        .items
        .iter()
        .filter(|item| item.item_id == item_id)
        .map(|item| item.stack_count as u16)
        .sum()
}

fn format_missing_materials(missing: &[CraftMaterial]) -> String {
    let list = missing
        .iter()
        .map(|material| format!("{} x{}", material.item_id, material.count))
        .collect::<Vec<_>>()
        .join(", ");
    format!("missing materials: {list}")
}

fn skill_up_snapshot(
    professions: &PlayerProfessions,
    profession: Profession,
    skilled_up: bool,
) -> Option<ProfessionSkillSnapshot> {
    if !skilled_up {
        return None;
    }
    professions
        .get(profession)
        .map(|skill| ProfessionSkillSnapshot {
            profession: profession_name(profession).into(),
            current: skill.current,
            max: skill.max,
        })
}

fn profession_name(profession: Profession) -> &'static str {
    match profession {
        Profession::Mining => "Mining",
        Profession::Herbalism => "Herbalism",
        Profession::Skinning => "Skinning",
        Profession::Blacksmithing => "Blacksmithing",
        Profession::Leatherworking => "Leatherworking",
        Profession::Tailoring => "Tailoring",
        Profession::Engineering => "Engineering",
        Profession::Alchemy => "Alchemy",
        Profession::Enchanting => "Enchanting",
        Profession::Jewelcrafting => "Jewelcrafting",
        Profession::Inscription => "Inscription",
        Profession::Cooking => "Cooking",
        Profession::Fishing => "Fishing",
    }
}

fn gather_node_def(node_id: u32) -> Option<&'static GatherNodeDef> {
    gather_node_defs().iter().find(|node| node.id == node_id)
}

fn recipe_def(recipe_id: u32) -> Option<&'static ProfessionRecipeDef> {
    recipe_defs()
        .iter()
        .find(|recipe| recipe.recipe_id == recipe_id)
}

fn gather_node_from_def(def: &GatherNodeDef) -> GatherNode {
    GatherNode {
        profession: def.profession,
        required_skill: def.required_skill,
        trivial_skill: def.trivial_skill,
        yields: vec![(def.yield_item_id, def.min_count, def.max_count)],
        respawn_secs: 30,
    }
}

fn recipe_from_def(def: &ProfessionRecipeDef) -> Recipe {
    Recipe {
        id: def.recipe_id,
        profession: def.profession,
        required_skill: def.required_skill,
        trivial_skill: def.trivial_skill,
        materials: vec![CraftMaterial {
            item_id: def.material_item_id,
            count: def.material_count,
        }],
        output_item_id: def.output_item_id,
        output_count: def.output_count,
        source: None,
    }
}

fn gather_node_defs() -> &'static [GatherNodeDef] {
    &[GatherNodeDef {
        id: NODE_COPPER_VEIN,
        name: "Copper Vein",
        cast_secs: GATHER_CAST_SECS,
        profession: Profession::Mining,
        required_skill: 1,
        trivial_skill: 50,
        yield_item_id: COPPER_ORE_ID,
        yield_name: "Copper Ore",
        min_count: 1,
        max_count: 2,
        vendor_sell_price: 25,
    }]
}

fn recipe_defs() -> &'static [ProfessionRecipeDef] {
    &[ProfessionRecipeDef {
        recipe_id: RECIPE_COPPER_BRACERS,
        name: "Copper Bracers",
        profession: Profession::Blacksmithing,
        required_skill: 1,
        trivial_skill: 50,
        material_item_id: COPPER_ORE_ID,
        material_name: "Copper Ore",
        material_count: 2,
        output_item_id: COPPER_BRACERS_ID,
        output_name: "Copper Bracers",
        output_count: 1,
        output_vendor_sell_price: 75,
    }]
}

#[cfg(test)]
fn test_db() -> GameDatabase {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("professions.redb");
    let db = redb::Database::create(&path).expect("db");
    crate::persistence::create_tables(&db);
    std::mem::forget(dir);
    GameDatabase { db }
}

#[cfg(test)]
#[path = "profession_tests.rs"]
mod tests;

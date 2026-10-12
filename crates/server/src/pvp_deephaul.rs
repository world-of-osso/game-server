//! Deephaul's two source NPC carts and source base/drop crystal templates.
use crate::game_objects::{GameObjectSpawn, GameObjectSpawns};
use crate::maps::InMap;
use crate::world_data::WorldData;
use bevy::prelude::*;
use shared::battleground::{
    BgInstance, BgTeam, BgTemplate, CartEvent, CartPaths, DEEPHAUL_CART_SPEED, DEEPHAUL_CARTS,
    DEEPHAUL_DROP_ENTRY, DEEPHAUL_FLAG_ENTRY, DeephaulBattle, FlagSite,
};
use shared::protocol::GameObjectInfo;

#[derive(Debug)]
pub(super) struct DeephaulRuntime {
    pub battle: DeephaulBattle,
    paths: CartPaths,
    entities: [Option<Entity>; 2],
    flag_home: GameObjectSpawn,
}

impl DeephaulRuntime {
    pub fn from_source(world: &World) -> Result<Self, String> {
        super::pvp_objects::require_scripted_objects(world, &[DEEPHAUL_DROP_ENTRY])?;
        let data = world
            .get_resource::<WorldData>()
            .ok_or("Deephaul creature catalog unavailable")?;
        let mut paths = CartPaths::new();
        for definition in &DEEPHAUL_CARTS {
            if !data.templates.contains_key(&definition.entry) {
                return Err(format!(
                    "Deephaul cart template {} missing; run add_battleground_summon_templates.py",
                    definition.entry
                ));
            }
            let path = data
                .waypoints
                .get(&definition.path)
                .filter(|path| !path.points.is_empty())
                .ok_or_else(|| {
                    format!(
                        "Deephaul source path {} missing; run add_battleground_summon_templates.py",
                        definition.path
                    )
                })?;
            paths.insert(
                definition.path,
                path.points
                    .iter()
                    .map(|point| [point.position_x, point.position_y, point.position_z])
                    .collect(),
            );
        }
        let flag_home = world
            .resource::<GameObjectSpawns>()
            .0
            .iter()
            .find(|spawn| spawn.map_id == 2656 && spawn.info.entry == DEEPHAUL_FLAG_ENTRY)
            .cloned()
            .ok_or("authored Deephaul Crystal absent")?;
        Ok(Self {
            battle: DeephaulBattle::new(),
            paths,
            entities: [None; 2],
            flag_home,
        })
    }

    pub fn advance(
        &mut self,
        world: &mut World,
        map: InMap,
        delta_ms: u64,
        players: &[(u64, BgTeam, [f32; 3])],
        instance: &mut BgInstance,
        template: &BgTemplate,
    ) -> Vec<CartEvent> {
        let events = self
            .battle
            .advance(delta_ms, players, &self.paths, instance, template);
        self.synchronize_carts(world, map);
        events
    }

    pub fn synchronize_carts(&mut self, world: &mut World, map: InMap) {
        for (index, site) in self.battle.sites().into_iter().enumerate() {
            match (self.entities[index], site) {
                (Some(entity), None) => {
                    world.despawn(entity);
                    self.entities[index] = None;
                }
                (None, Some(at)) => {
                    let definition = &DEEPHAUL_CARTS[index];
                    self.entities[index] = Some(super::pvp_silvershard::spawn_source_cart(
                        world,
                        map,
                        definition.entry,
                        at,
                        definition.orientation,
                    ));
                }
                (Some(entity), Some(at)) => {
                    super::pvp_silvershard::move_source_cart(world, entity, at, DEEPHAUL_CART_SPEED)
                }
                (None, None) => {}
            }
        }
    }

    pub fn synchronize_crystal(&self, world: &mut World, map: InMap) {
        let mut query = world.query::<(Entity, &GameObjectInfo, &InMap)>();
        let objects: Vec<_> = query
            .iter(world)
            .filter(|(_, info, at)| {
                **at == map && [DEEPHAUL_FLAG_ENTRY, DEEPHAUL_DROP_ENTRY].contains(&info.entry)
            })
            .map(|(entity, _, _)| entity)
            .collect();
        for object in objects {
            world.despawn(object);
        }
        match self.battle.flag_site() {
            Some(FlagSite::Home) => super::pvp_objective::spawn_flag(
                world,
                map,
                &self.flag_home,
                self.flag_home.position,
            ),
            Some(FlagSite::Dropped(at)) => {
                super::pvp_objects::spawn_scripted_object(
                    world,
                    map,
                    DEEPHAUL_DROP_ENTRY,
                    Vec3::from_array(at),
                    self.flag_home.orientation,
                );
            }
            None => {}
        }
    }
}

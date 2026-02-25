use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy)]
pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy)]
pub struct Mana {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy)]
pub struct MovementSpeed(pub f32);

#[derive(Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone)]
pub struct Player {
    pub name: String,
}

#[derive(Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy)]
pub struct Npc {
    pub template_id: u32,
}

#[derive(Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy)]
pub struct Zone {
    pub id: u32,
}

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($name:ident, $inner:ty) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            Serialize,
            Deserialize,
            Component,
            Reflect,
            bitcode::Encode,
            bitcode::Decode,
        )]
        pub struct $name(pub $inner);
    };
}

id_type!(PlayerId, u64);
id_type!(CreatureId, u64);
id_type!(ItemId, u32);
id_type!(SpellId, u32);
id_type!(MapId, u32);
id_type!(ZoneId, u32);

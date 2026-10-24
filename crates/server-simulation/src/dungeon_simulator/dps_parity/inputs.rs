//! Read exact external reference inputs; unsupported options remain reportable data.
use bevy::prelude::*;
use serde::Deserialize;
use server::item_instance::{ItemInstanceModifiers, load};
use shared::protocol::EquipmentSlot;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub(super) struct Profile {
    pub spec_id: u32,
    pub level: String,
    pub race: String,
    pub equipment: Vec<Equipment>,
    pub consumables: BTreeMap<String, String>,
    pub reference_gap: Option<String>,
    pub omnium_talents: Option<String>,
    #[serde(flatten)]
    pub options: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
pub(super) struct Equipment {
    pub slot: String,
    pub item_id: u32,
    pub bonus_list: Vec<u32>,
    pub item_level_override: Option<u16>,
    pub enchant: u32,
    pub gems: Vec<u32>,
    pub crafted_stats: Vec<i8>,
    pub options: BTreeMap<String, String>,
}

impl Equipment {
    fn stat_redirect(&self, world: &World) -> Result<Option<(EquipmentSlot, u32)>, String> {
        let Some(value) = self.options.get("redirected_base_stats") else {
            return Ok(None);
        };
        let id: u32 = value.parse().map_err(|error| {
            format!(
                "reference {} item{} redirected_base_stats {value:?}: {error}",
                self.slot, self.item_id
            )
        })?;
        // Pinned item.cpp95/1314: zero is the explicit no-redirect value.
        if id == 0 {
            return Ok(None);
        }
        let templates = world
            .get_resource::<crate::item_templates::ItemTemplates>()
            .ok_or("redirected_base_stats requires native item catalogs")?;
        if templates
            .get(shared::item_data::ItemDefinitionSource::Retail, id)
            .is_none()
        {
            return Err(format!(
                "reference {} item{}: native redirected_base_stats item{id} absent",
                self.slot, self.item_id
            ));
        }
        Ok(Some((slot(&self.slot)?, id)))
    }

    pub fn modifiers(&self) -> ItemInstanceModifiers {
        ItemInstanceModifiers {
            bonus_list: self.bonus_list.clone(),
            item_level_override: self.item_level_override,
            enchant: self.enchant,
            gems: self.gems.clone(),
            crafted_stats: self.crafted_stats.clone(),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct AllocatedTrait {
    pub node_id: u32,
    pub entry_id: u32,
    pub ranks: u32,
    pub active_ranks: u32,
}

pub(super) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

pub(super) fn directory() -> Result<PathBuf, String> {
    std::env::var_os("DPS_PARITY_OUTPUT")
        .map(PathBuf::from)
        .ok_or_else(|| "DPS_PARITY_OUTPUT must name the prepared profile/evidence directory".into())
}

pub(super) fn read_actor(
    profile: &Profile,
) -> Result<crate::followers::simulator::ParityActor, String> {
    use crate::followers::simulator::ParityActor;
    match std::env::var("DPS_PARITY_ACTOR") {
        Err(std::env::VarError::NotPresent) => Ok(ParityActor::Follower),
        Ok(mode) if mode == "follower" => Ok(ParityActor::Follower),
        Ok(mode) if mode == "player" => Ok(ParityActor::Player {
            race: read_reference_race(&profile.race)?,
        }),
        other => Err(format!("invalid DPS_PARITY_ACTOR: {other:?}")),
    }
}

fn read_reference_race(name: &str) -> Result<u8, String> {
    let path = std::env::var_os("DPS_PARITY_WORLD_DB").ok_or("DPS_PARITY_WORLD_DB absent")?;
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("read native race catalog: {error}"))?;
    let mut statement = db
        .prepare("SELECT ID, Name_lang FROM chr_races")
        .map_err(|error| format!("query native race catalog: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, u8>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("read native race rows: {error}"))?;
    let normalize = |text: &str| {
        text.chars()
            .filter(char::is_ascii_alphanumeric)
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let expected = normalize(name);
    let mut matches = Vec::new();
    for row in rows {
        let (id, native_name) = row.map_err(|error| format!("decode native race: {error}"))?;
        if normalize(&native_name) == expected {
            matches.push(id);
        }
    }
    if expected == "human" {
        // Pinned SimC76f8118 util.cpp1989-1994 maps RACE_HUMAN to1.
        // Native70448 also calls ThinHuman33 "Human"; display names are not IDs.
        const SIMC_HUMAN_RACE_ID: u8 = 1;
        return matches
            .contains(&SIMC_HUMAN_RACE_ID)
            .then_some(SIMC_HUMAN_RACE_ID)
            .ok_or_else(|| {
                format!("reference human source race1 absent from native matches {matches:?}")
            });
    }
    match matches.as_slice() {
        [id] => Ok(*id),
        _ => Err(format!(
            "reference race {name:?} matched native races {matches:?}"
        )),
    }
}

pub(super) fn slot(name: &str) -> Result<EquipmentSlot, String> {
    use EquipmentSlot::*;
    match name {
        "head" => Ok(Head),
        "neck" => Ok(Neck),
        "shoulders" => Ok(Shoulder),
        "back" => Ok(Back),
        "chest" => Ok(Chest),
        "wrists" => Ok(Wrist),
        "hands" => Ok(Hands),
        "waist" => Ok(Waist),
        "legs" => Ok(Legs),
        "feet" => Ok(Feet),
        "finger1" => Ok(Finger1),
        "finger2" => Ok(Finger2),
        "trinket1" => Ok(Trinket1),
        "trinket2" => Ok(Trinket2),
        "main_hand" => Ok(MainHand),
        "off_hand" => Ok(OffHand),
        _ => Err(format!("unsupported reference equipment slot {name}")),
    }
}

pub(super) fn install_reference_catalogs(app: &mut App) -> Result<(), String> {
    let path = std::env::var_os("DPS_PARITY_WORLD_DB")
        .map(PathBuf::from)
        .ok_or("DPS_PARITY_WORLD_DB must name the private native database")?;
    let db =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("open private database {}: {error}", path.display()))?;
    let data = crate::world_data::load_world_data(&path);
    app.insert_resource(data.armor_constants.clone());
    app.insert_resource(data);
    app.insert_resource(
        crate::hunter_pet::read_pet_catalog(&db).map_err(|error| error.to_string())?,
    );
    app.insert_resource(
        crate::pet_control::load_pet_levelup_spells(&db).map_err(|error| error.to_string())?,
    );
    let followers = crate::followers::simulator::load_parity_catalog(&db)
        .map_err(|error| format!("load private follower catalog: {error}"))?;
    app.insert_resource(followers);
    let native = load_native_catalogs(&db)?;
    app.insert_resource(native);
    app.insert_resource(
        crate::item_proc_data::load(&db)
            .map_err(|error| format!("load private native equipped-producer inputs: {error}"))?,
    );
    app.insert_resource(
        crate::item_templates::load_item_templates(&path).map_err(|error| error.to_string())?,
    );
    app.insert_resource(
        crate::item_stats::load_item_scaling_tables(&path).map_err(|error| error.to_string())?,
    );
    Ok(())
}

fn load_native_catalogs(
    db: &rusqlite::Connection,
) -> Result<crate::stats::NativeItemInstances, String> {
    Ok(crate::stats::NativeItemInstances {
        bonuses: load::load_bonus_tables(db).map_err(|error| error.to_string())?,
        enchants: load::load_enchantment_tables(db).map_err(|error| error.to_string())?,
        effects: load::load_item_effect_tables(db).map_err(|error| error.to_string())?,
    })
}

pub(super) fn equip(world: &mut World, unit: Entity, profile: &Profile) -> Result<(), String> {
    let items = profile
        .equipment
        .iter()
        .map(|item| {
            Ok((
                slot(&item.slot)?,
                shared::item_data::ItemDefinitionSource::Retail,
                item.item_id,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let modifiers = profile
        .equipment
        .iter()
        .map(|item| Ok((slot(&item.slot)?, item.modifiers())))
        .collect::<Result<Vec<_>, String>>()?;
    let base_stat_redirects = profile
        .equipment
        .iter()
        .filter_map(|item| item.stat_redirect(world).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    world.entity_mut(unit).insert((
        crate::stats::EquippedItems { items },
        crate::stats::EquippedItemModifiers {
            items: modifiers,
            base_stat_redirects,
        },
    ));
    Ok(())
}

pub(super) fn install_traits(
    world: &mut World,
    unit: Entity,
    allocated: &[AllocatedTrait],
) -> Result<(), String> {
    let level = world.get::<shared::components::Level>(unit).unwrap().0;
    if allocated
        .iter()
        .any(|entry| entry.active_ranks > entry.ranks)
    {
        return Err("active trait ranks exceed exported allocations".into());
    }
    let purchases: Vec<_> = allocated
        .iter()
        .filter(|entry| entry.active_ranks > 0)
        .map(|entry| crate::trait_config::Purchase {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            ranks: entry.active_ranks,
        })
        .collect();
    let classes = world.resource::<crate::spell_info::ClassSpells>();
    let (spec, evaluated) = if let Some(player) = world.get::<shared::components::Player>(unit) {
        let spec = world
            .get::<crate::class_progression::ActiveSpec>(unit)
            .ok_or("reference player lacks native ActiveSpec")?
            .0;
        (
            spec,
            crate::class_progression::evaluate_player_reference_traits(
                classes, player, spec, level, &purchases,
            ),
        )
    } else {
        let follower = world
            .get::<crate::followers::Follower>(unit)
            .ok_or("reference actor is neither player nor follower")?;
        (
            follower.profile.spec_id,
            crate::class_progression::evaluate_reference_traits(
                classes,
                &follower.profile,
                level,
                &purchases,
            ),
        )
    };
    let (known, traits) =
        evaluated.map_err(|reason| format!("NATIVE_REFERENCE_TRAIT_CONFIG_REJECTED: {reason}"))?;
    let known = apply_ret_hammer_of_wrath_known_from_selected_talent_simc(spec, known, &traits);
    world.entity_mut(unit).insert((known, traits));
    Ok(())
}

/// RET_HAMMER_OF_WRATH_KNOWN_FROM_SELECTED_TALENT_SIMC: reference actors only.
/// SimC76f8118 sc_paladin.cpp3407–3412/4192 links the selected class talent
///1241288 to action24275. Native70448 has no learn edge; NeverLearned stays native.
fn apply_ret_hammer_of_wrath_known_from_selected_talent_simc(
    spec: u32,
    known: crate::class_progression::KnownSpells,
    traits: &crate::class_progression::TraitLoadout,
) -> crate::class_progression::KnownSpells {
    const RETRIBUTION: u32 = 70;
    const HAMMER_OF_WRATH_TALENT: u32 = 1241288;
    const HAMMER_OF_WRATH_ACTION: u32 = 24275;
    let selected = traits
        .active_talent_spells()
        .any(|spell| spell.spell_id == HAMMER_OF_WRATH_TALENT);
    if spec != RETRIBUTION || !selected || known.contains(HAMMER_OF_WRATH_ACTION) {
        return known;
    }
    crate::class_progression::KnownSpells::from_baseline(
        known
            .iter()
            .chain(std::iter::once(HAMMER_OF_WRATH_ACTION))
            .collect(),
    )
}

#[test]
#[ignore = "requires prepared native Ret profile and selected76f8118 allocations"]
fn dps_parity_ret_hammer_knowledge_is_scoped_to_selected_reference_actors() {
    use crate::class_progression::KnownSpells;
    use crate::followers::simulator::ParityActor;
    let selected: Vec<AllocatedTrait> =
        read_json(&directory().unwrap().join("70.selected-traits-active.json")).unwrap();
    for identity in [ParityActor::Follower, ParityActor::Player { race: 3 }] {
        let profile: Profile = read_json(&directory().unwrap().join("70.profile.json")).unwrap();
        let mut run = super::prepare_reference(profile, &selected, 1, identity).unwrap();
        let known = run.app.world().get::<KnownSpells>(run.actor).unwrap();
        assert!(
            known.contains(1241288),
            "native selected class talent is known"
        );
        assert!(
            known.contains(24275),
            "selected parity actor must know Hammer of Wrath24275"
        );
        install_traits(run.app.world_mut(), run.actor, &[]).unwrap();
        let known = run.app.world().get::<KnownSpells>(run.actor).unwrap();
        // Native Ret grants its root class talent even with no purchases.
        assert!(known.contains(1241288));
        assert!(known.contains(24275));
    }
}

#[test]
#[ignore = "requires native70448 class catalogs and a prepared Ret gear profile"]
fn dps_parity_ret_hammer_policy_does_not_grant_to_other_specializations() {
    let (mut profile, _) = super::read_reference(70).unwrap();
    profile.spec_id = 66;
    let run = super::prepare_reference(
        profile,
        &[],
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    assert!(!known.contains(24275));
}

#[test]
#[ignore = "requires native70448 Ret traits; ordinary reference evaluation remains native-only"]
fn dps_parity_ret_hammer_native_book_does_not_learn_the_reference_exception() {
    let (profile, selected) = super::read_reference(70).unwrap();
    let run = super::prepare_reference(
        profile,
        &[],
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    let purchases = selected
        .iter()
        .filter(|entry| entry.active_ranks > 0)
        .map(|entry| crate::trait_config::Purchase {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            ranks: entry.active_ranks,
        })
        .collect::<Vec<_>>();
    let world = run.app.world();
    let classes = world.resource::<crate::spell_info::ClassSpells>();
    let player = world.get::<shared::components::Player>(run.actor).unwrap();
    let (known, _) = crate::class_progression::evaluate_player_reference_traits(
        classes,
        player,
        70,
        run.profile.level.parse().unwrap(),
        &purchases,
    )
    .unwrap();
    assert!(known.contains(1241288));
    assert!(
        !known.contains(24275),
        "ordinary native books must remain unchanged"
    );
}

#[test]
#[ignore = "requires the read-only native parity database with Human1 and ThinHuman33"]
fn dps_parity_input_human_race_uses_the_pinned_source_id() {
    assert_eq!(read_reference_race("human").unwrap(), 1);
    assert!(read_reference_race("not_an_installed_race").is_err());
}

#[cfg(unix)]
#[test]
#[ignore = "requires scripts/dps_parity_native.py frozen source view and private parity inputs"]
fn dps_parity_input_startup_loaders_share_the_prepared_database() {
    use std::os::unix::fs::MetadataExt;
    let prepared = std::env::var_os("DPS_PARITY_WORLD_DB").expect("prepared native database");
    let expected = std::fs::metadata(PathBuf::from(prepared)).unwrap();
    for startup in [
        crate::test_data::world_db_path(),
        PathBuf::from(crate::spell_info::WORLD_DB_PATH),
    ] {
        let actual = std::fs::metadata(&startup).unwrap();
        assert_eq!(
            (actual.dev(), actual.ino()),
            (expected.dev(), expected.ino()),
            "startup loader {} must read the prepared snapshot, not a live canonical database",
            startup.display()
        );
    }
}

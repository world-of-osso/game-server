mod achievement;
mod admin_handlers;
mod admin_ipc;
mod admin_types;
mod adventure_guide;
mod appearance_collection;
mod area_table;
mod auction_house;
mod auras;
mod auth;
mod auth_character;
mod bank;
mod bank_storage;
#[cfg(test)]
mod bank_test_harness;
mod barber_shop;
mod battle_pets;
mod calendar;
mod chair;
mod character_stats_data;
mod class_progression;
mod collection;
mod combat;
mod combat_refs;
mod conditions;
mod crafting_orders;
mod creature_aggro;
mod creature_ai;
mod creature_scripts;
mod creature_sight;
mod creature_tap;
mod creature_tooltip;
mod currency;
mod damage_meter;
mod db2_source_build;
mod death;
mod duel;
mod durability;
mod economy;
mod equipment_admin_types;
mod experience;
mod faction;
mod faction_reaction;
mod followers;
mod frame_stats;
mod friends;
mod game_objects;
mod gathering;
mod gold;
mod grid_activity;
mod ground;
mod group;
mod guild;
mod guild_bank;
mod guild_event_log;
mod guild_news;
mod guild_news_network;
mod guild_perks;
mod guild_tabards;
mod housing;
mod housing_acquisition;
mod housing_catalog;
#[cfg(test)]
mod housing_quest_tests;
mod housing_quests;
mod hunter_pet;
mod ignore_list;
mod inspect;
mod interest;
mod inventory;
mod item_stats;
mod item_templates;
mod knockback;
mod level_scaling;
mod lfg;
mod line_of_sight;
mod liquid;
mod loot;
mod mail;
mod maps;
mod meeting_stone;
mod melee;
mod merchant;
mod messaging;
mod mirror_timers;
mod mount_journal;
mod mythic_plus;
mod navigation;
mod networking;
mod npc_interaction;
#[cfg(test)]
mod owned_commerce_source_tests;
mod persistence;
mod persistence_accounts;
mod persistence_character;
mod persistence_progression;
mod persistence_types;
mod persistence_writer;
mod pet_control;
mod pet_journal_state;
#[cfg(test)]
mod pet_journal_state_tests;
mod pet_stable;
mod player_create_info;
mod player_start;
mod profession;
mod pvp;
mod quest;
mod quest_bonus_money;
mod quest_data;
mod quest_rules;
mod quest_time;
mod quest_view;
mod race_class;
mod raid_history;
mod renown;
mod reputation;
mod rest;
mod scripted_ai;
mod seething_parachute;
mod sheet_visibility;
mod skyriding;
mod spawn_groups;
mod spawn_pools;
mod spell_area;
mod spell_cast;
mod spell_info;
mod spell_scaling_data;
mod stand_state;
mod stats;
#[cfg(feature = "system-timings")]
mod system_timings;
mod tame;
mod taxi;
mod teleport;
mod terrain_height;
#[cfg(test)]
mod test_data;
mod threat_updates;
mod toy_catalog;
mod toys;
mod trade;
mod trading_post;
mod trading_post_store;
mod trait_config;
mod transmog;
mod transmog_store;
mod transport;
mod transport_catalog;
mod transport_movement;
mod unit_creature_type;
mod unit_target;
mod vehicle;
mod vignettes;
mod who;
mod wild_pet_battles;
mod world_data;
mod world_map;
mod world_quest_availability;
mod world_quest_objectives;
mod world_quest_rewards;
mod world_quests;
mod world_time;
mod xp_data;
mod zones;

use bevy::prelude::*;
use lightyear::prelude::{ReplicationMetadata, server::ServerPlugins};
use std::time::Duration;

/// Server tick rate: 20Hz (50ms per tick).
const TICK_RATE_HZ: f64 = 20.0;

fn main() {
    let mut app = App::new();
    add_runtime_plugins(&mut app);
    add_game_plugins(&mut app);
    register_server_resources(&mut app);
    register_server_systems(&mut app);
    app.run();
}

fn add_runtime_plugins(app: &mut App) {
    app.add_plugins(
        MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
        )),
    )
    .add_plugins(bevy::state::app::StatesPlugin)
    .add_plugins(log_plugin())
    .add_plugins(ServerPlugins {
        tick_duration: Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
    })
    .insert_resource(ReplicationMetadata::new(Duration::from_secs_f64(
        1.0 / TICK_RATE_HZ,
    )))
    .add_plugins(frame_stats::FrameStatsPlugin {
        budget: Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
    });
}

fn log_plugin() -> bevy::log::LogPlugin {
    bevy::log::LogPlugin {
        #[cfg(feature = "system-timings")]
        custom_layer: system_timings::system_timing_layer,
        ..default()
    }
}

fn add_game_plugins(app: &mut App) {
    app.add_plugins(shared::ProtocolPlugin)
        .add_plugins(persistence::PersistencePlugin)
        .add_plugins(achievement::AchievementPlugin)
        .add_plugins(adventure_guide::AdventureGuidePlugin)
        .add_plugins(trading_post::TradingPostPlugin)
        .add_plugins(renown::RenownPlugin);
    add_social_plugins(app);
    add_progression_plugins(app);
    add_world_plugins(app);
}

fn add_social_plugins(app: &mut App) {
    app.add_plugins(barber_shop::BarberShopPlugin)
        .add_plugins(calendar::CalendarPlugin)
        .add_plugins(friends::FriendsPlugin)
        .add_plugins(group::GroupPlugin)
        .add_plugins(guild::GuildPlugin)
        .add_plugins(
            guild_perks::GuildPerksPlugin::load(std::path::Path::new("data/db2/active"))
                .expect("installed Mainline guild perk catalog"),
        )
        .add_plugins(
            guild_tabards::GuildTabardPlugin::load(std::path::Path::new("data/db2/active"))
                .expect("load installed guild tabard catalog"),
        )
        .add_plugins(who::WhoPlugin)
        .add_plugins(ignore_list::IgnoreListPlugin)
        .add_plugins(lfg::LfgPlugin)
        .add_plugins(followers::FollowerPlugin)
        .add_plugins(duel::DuelPlugin)
        .add_plugins(inspect::InspectPlugin)
        .add_plugins(trade::TradePlugin)
        .add_plugins(item_templates::ItemTemplatesPlugin)
        .add_plugins(player_create_info::PlayerCreateInfoPlugin)
        .add_plugins(conditions::ConditionsPlugin)
        .add_plugins(item_stats::ItemScalingPlugin)
        .add_plugins(inventory::InventoryPlugin)
        .add_plugins(appearance_collection::AppearanceCollectionPlugin)
        .add_plugins(transmog::TransmogPlugin)
        .add_plugins(quest_data::QuestDataPlugin)
        .add_plugins(quest::QuestPlugin)
        .add_plugins(spell_area::SpellAreaPlugin)
        .add_plugins(experience::ExperiencePlugin);
}

fn add_progression_plugins(app: &mut App) {
    app.add_plugins(gold::GoldPlugin)
        .add_plugins(auction_house::AuctionHousePlugin)
        .add_plugins(economy::EconomyPlugin)
        .add_plugins(collection::CollectionPlugin)
        .add_plugins(toys::ToysPlugin)
        .add_plugins(currency::CurrencyPlugin)
        .add_plugins(durability::DurabilityPlugin)
        .add_plugins(death::DeathPlugin)
        .add_plugins(pvp::PvpPlugin)
        .add_plugins(profession::ProfessionPlugin)
        .add_plugins(crafting_orders::CraftingOrdersPlugin)
        .add_plugins(reputation::ReputationPlugin)
        .add_plugins(rest::RestPlugin)
        .add_plugins(stand_state::StandStatePlugin)
        .add_plugins(hunter_pet::HunterPetPlugin)
        .add_plugins(pet_control::PetControlPlugin)
        .add_plugins(class_progression::ClassProgressionPlugin);
}

fn add_world_plugins(app: &mut App) {
    app.add_plugins(world_map::WorldMapPlugin)
        .add_plugins(networking::ServerNetworkPlugin)
        .add_plugins(world_data::WorldDataPlugin)
        .add_plugins(ground::GroundPlugin)
        .add_plugins(line_of_sight::LineOfSightPlugin)
        .add_plugins(navigation::NavigationPlugin)
        .add_plugins(mirror_timers::MirrorTimerPlugin)
        .add_plugins(world_time::WorldTimePlugin)
        .add_plugins(maps::MapPlugin)
        .add_plugins(housing::HousingPlugin)
        .add_plugins(mythic_plus::MythicPlusPlugin)
        .add_plugins(zones::ZonePlugin)
        .add_plugins(creature_ai::CreatureAiPlugin)
        .add_plugins(creature_aggro::CreatureAggroPlugin)
        .add_plugins(creature_scripts::CreatureScriptsPlugin)
        .add_plugins(scripted_ai::ScriptedAiPlugin)
        .add_plugins(grid_activity::GridActivityPlugin)
        .add_plugins(stats::StatsPlugin)
        .add_plugins(combat::CombatPlugin)
        .add_plugins(damage_meter::DamageMeterPlugin)
        .add_plugins(threat_updates::ThreatUpdatesPlugin)
        .add_plugins(unit_target::UnitTargetPlugin)
        .add_plugins(npc_interaction::NpcInteractionPlugin)
        .add_plugins(merchant::MerchantPlugin)
        .add_plugins(loot::LootPlugin)
        .add_plugins(creature_tooltip::CreatureTooltipPlugin)
        .add_plugins(taxi::TaxiPlugin)
        .add_plugins(vehicle::VehiclePlugin)
        .add_plugins(game_objects::GameObjectPlugin)
        .add_plugins(bank::BankPlugin)
        .add_plugins(guild_bank::GuildBankPlugin)
        .add_plugins(mail::MailPlugin)
        .add_plugins(meeting_stone::MeetingStonePlugin)
        .add_plugins(spell_info::SpellInfoPlugin)
        .add_plugins(spell_cast::SpellCastPlugin)
        .add_plugins(auras::AuraPlugin)
        .add_plugins(interest::InterestPlugin)
        .add_plugins(sheet_visibility::SheetVisibilityPlugin)
        .add_plugins(admin_ipc::AdminIpcPlugin);
}

fn register_server_resources(app: &mut App) {
    app.init_resource::<shared::ticket::TicketManager>()
        .init_resource::<shared::motd::Autobroadcast>();
}

fn register_server_systems(app: &mut App) {
    app.add_systems(Startup, on_startup)
        // Motd loaded from redb in PersistencePlugin startup
        .add_systems(FixedUpdate, tick_log);
}

fn on_startup() {
    info!("Game server starting at {TICK_RATE_HZ}Hz tick rate");
}

fn tick_log(time: Res<Time>, mut tick_count: Local<u64>) {
    *tick_count += 1;
    if (*tick_count).is_multiple_of(TICK_RATE_HZ as u64 * 10) {
        info!("Tick {} (uptime: {:.0}s)", *tick_count, time.elapsed_secs());
    }
}

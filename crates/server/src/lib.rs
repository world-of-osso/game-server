//! Native executable assembly over the `server-combat` gameplay library.

extern crate self as server;

pub use server_combat::*;

/// Public combat API with the complete native script fixture assembly.
pub mod spell_cast {
    pub use server_combat::spell_cast::*;
    #[cfg(any(test, feature = "test-support"))]
    pub use server_spell_scripts::test_support as test_harness;
}

mod calendar;
mod frame_stats;
mod inspect;
#[cfg(feature = "system-timings")]
mod system_timings;
mod who;
mod world_time;

use bevy::prelude::*;
use lightyear::prelude::{ReplicationMetadata, server::ServerPlugins};
use std::time::Duration;

/// Server tick rate: 20Hz (50ms per tick).
const TICK_RATE_HZ: f64 = 20.0;

/// Install the same runtime, gameplay, resources and schedules used by the server bin.
pub fn configure_app(app: &mut App) {
    add_runtime_plugins(app);
    add_game_plugins(app);
    register_server_resources(app);
    register_server_systems(app);
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
    app.add_plugins(server_spell_scripts::SpellScriptsPlugin);
    app.add_plugins(shared::ProtocolPlugin)
        .add_plugins(PersistencePlugin)
        .add_plugins(AchievementPlugin)
        .add_plugins(AdventureGuidePlugin)
        .add_plugins(TradingPostPlugin)
        .add_plugins(RenownPlugin);
    add_social_plugins(app);
    add_progression_plugins(app);
    add_world_plugins(app);
}

fn add_social_plugins(app: &mut App) {
    app.add_plugins(BarberShopPlugin)
        .add_plugins(calendar::CalendarPlugin)
        .add_plugins(FriendsPlugin)
        .add_plugins(GroupPlugin)
        .add_plugins(GuildPlugin)
        .add_plugins(
            GuildPerksPlugin::load(std::path::Path::new("data/db2/active"))
                .expect("installed Mainline guild perk catalog"),
        )
        .add_plugins(
            GuildTabardPlugin::load(std::path::Path::new("data/db2/active"))
                .expect("load installed guild tabard catalog"),
        )
        .add_plugins(who::WhoPlugin)
        .add_plugins(IgnoreListPlugin)
        .add_plugins(LfgPlugin)
        .add_plugins(FollowerPlugin)
        .add_plugins(DuelPlugin)
        .add_plugins(inspect::InspectPlugin)
        .add_plugins(TradePlugin)
        .add_plugins(ItemTemplatesPlugin)
        .add_plugins(PlayerCreateInfoPlugin)
        .add_plugins(ConditionsPlugin)
        .add_plugins(ItemScalingPlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(AppearanceCollectionPlugin)
        .add_plugins(TransmogPlugin)
        .add_plugins(QuestDataPlugin)
        .add_plugins(QuestPlugin)
        .add_plugins(SpellAreaPlugin)
        .add_plugins(ExperiencePlugin);
}

fn add_progression_plugins(app: &mut App) {
    app.add_plugins(GoldPlugin)
        .add_plugins(AuctionHousePlugin)
        .add_plugins(EconomyPlugin)
        .add_plugins(CollectionPlugin)
        .add_plugins(ToysPlugin)
        .add_plugins(CurrencyPlugin)
        .add_plugins(DurabilityPlugin)
        .add_plugins(DeathPlugin)
        .add_plugins(PvpPlugin)
        .add_plugins(ProfessionPlugin)
        .add_plugins(CraftingOrdersPlugin)
        .add_plugins(ReputationPlugin)
        .add_plugins(RestPlugin)
        .add_plugins(StandStatePlugin)
        .add_plugins(HunterPetPlugin)
        .add_plugins(PetControlPlugin)
        .add_plugins(ClassProgressionPlugin);
}

fn add_world_plugins(app: &mut App) {
    app.add_plugins(WorldMapPlugin)
        .add_plugins(ServerNetworkPlugin)
        .add_plugins(WorldDataPlugin)
        .add_plugins(GroundPlugin)
        .add_plugins(LineOfSightPlugin)
        .add_plugins(NavigationPlugin)
        .add_plugins(MirrorTimerPlugin)
        .add_plugins(world_time::WorldTimePlugin)
        .add_plugins(MapPlugin)
        .add_plugins(HousingPlugin)
        .add_plugins(MythicPlusPlugin)
        .add_plugins(ZonePlugin)
        .add_plugins(CreatureAiPlugin)
        .add_plugins(CreatureAggroPlugin)
        .add_plugins(CreatureScriptsPlugin)
        .add_plugins(ScriptedAiPlugin)
        .add_plugins(GridActivityPlugin)
        .add_plugins(StatsPlugin)
        .add_plugins(CombatPlugin)
        .add_plugins(DamageMeterPlugin)
        .add_plugins(ThreatUpdatesPlugin)
        .add_plugins(UnitTargetPlugin)
        .add_plugins(NpcInteractionPlugin)
        .add_plugins(MerchantPlugin)
        .add_plugins(LootPlugin)
        .add_plugins(CreatureTooltipPlugin)
        .add_plugins(TaxiPlugin)
        .add_plugins(VehiclePlugin)
        .add_plugins(GameObjectPlugin)
        .add_plugins(BankPlugin)
        .add_plugins(GuildBankPlugin)
        .add_plugins(MailPlugin)
        .add_plugins(MeetingStonePlugin)
        .add_plugins(SpellInfoPlugin)
        .add_plugins(SpellCastPlugin)
        .add_plugins(AuraPlugin)
        .add_plugins(InterestPlugin)
        .add_plugins(SheetVisibilityPlugin)
        .add_plugins(AdminIpcPlugin);
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

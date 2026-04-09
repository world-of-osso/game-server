use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::{AppChannelExt, ChannelMode, ChannelSettings, NetworkDirection};

use super::*;

pub(super) fn register_messages(app: &mut App) {
    register_core_messages(app);
    register_account_messages(app);
    register_character_messages(app);
    register_guild_and_chat_messages(app);
    register_auction_messages(app);
    register_trade_messages(app);
    register_talent_messages(app);
    register_inspect_messages(app);
    register_duel_messages(app);
    register_profession_messages(app);
    register_reputation_messages(app);
    register_achievement_messages(app);
    register_world_map_messages(app);
    register_rest_messages(app);
    register_friends_messages(app);
    register_who_messages(app);
    register_calendar_messages(app);
    register_ignore_messages(app);
    register_lfg_messages(app);
    register_pvp_messages(app);
    register_barber_shop_messages(app);
    register_death_messages(app);
    register_durability_messages(app);
    register_collection_messages(app);
    register_currency_messages(app);
    crate::protocol_snapshots::register_snapshot_messages(app);
}

fn register_core_messages(app: &mut App) {
    app.register_message::<PlayerInput>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ChatMessage>()
        .add_direction(NetworkDirection::Bidirectional);
    app.register_message::<SetTarget>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CombatEvent>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<SpellCastIntent>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<StopSpellCast>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<EmoteIntent>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<EmoteEvent>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<GroupInviteIntent>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GroupUninviteIntent>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<LoadTerrain>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_account_messages(app: &mut App) {
    app.register_message::<LoginRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<LoginResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<ForcedDisconnect>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<RegisterRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<RegisterResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_character_messages(app: &mut App) {
    app.register_message::<CreateCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CreateCharacterResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<DeleteCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DeleteCharacterResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<CharacterListUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<SelectCharacter>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<EnterWorldResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_guild_and_chat_messages(app: &mut App) {
    app.register_message::<GuildInviteRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GuildInviteResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<GuildAcceptInviteRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GuildAcceptInviteResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryGuild>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SetGuildMotd>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SetGuildInfo>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SetGuildOfficerNote>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GuildStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<ChatHistoryRequest>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ChatHistoryResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_auction_messages(app: &mut App) {
    register_auction_query_messages(app);
    register_auction_action_messages(app);
    register_auction_mail_messages(app);
}

fn register_auction_query_messages(app: &mut App) {
    app.register_message::<OpenAuctionHouse>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionHouseOpened>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryAuctions>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionSearchResults>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryOwnedAuctions>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<OwnedAuctionListResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryBidAuctions>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<BidAuctionListResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<QueryAuctionInventory>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionInventorySnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_auction_action_messages(app: &mut App) {
    app.register_message::<PlaceBid>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<BuyoutAuction>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CreateAuction>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CancelAuction>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionOperationResponse>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_auction_mail_messages(app: &mut App) {
    app.register_message::<QueryAuctionMailbox>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AuctionMailboxSnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<ClaimAuctionMail>()
        .add_direction(NetworkDirection::ClientToServer);
}

fn register_trade_messages(app: &mut App) {
    app.register_message::<InitiateTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AcceptTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DeclineTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CancelTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SetTradeItem>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ClearTradeItem>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SetTradeMoney>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ConfirmTrade>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<TradeStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_talent_messages(app: &mut App) {
    app.register_message::<QueryTalents>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ApplyTalentChoice>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ResetTalents>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<TalentStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_inspect_messages(app: &mut App) {
    app.register_message::<QueryInspectTarget>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<InspectStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_duel_messages(app: &mut App) {
    app.register_message::<InitiateDuel>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AcceptDuel>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DeclineDuel>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DuelStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_profession_messages(app: &mut App) {
    app.register_message::<QueryProfessions>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CraftProfessionRecipe>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<GatherProfessionNode>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ProfessionStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_reputation_messages(app: &mut App) {
    app.register_message::<ReputationStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_achievement_messages(app: &mut App) {
    app.register_message::<AchievementStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_world_map_messages(app: &mut App) {
    app.register_message::<WorldMapStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_rest_messages(app: &mut App) {
    app.register_message::<RestStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_friends_messages(app: &mut App) {
    app.register_message::<QueryFriends>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AddFriend>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<RemoveFriend>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SetPresenceStatus>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<FriendsStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_who_messages(app: &mut App) {
    app.register_message::<QueryWho>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<WhoStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_calendar_messages(app: &mut App) {
    app.register_message::<QueryCalendar>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ScheduleCalendarEvent>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<RespondCalendarSignup>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CalendarStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_ignore_messages(app: &mut App) {
    app.register_message::<QueryIgnoreList>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AddIgnore>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<RemoveIgnore>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<IgnoreListStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_lfg_messages(app: &mut App) {
    app.register_message::<QueryLfgStatus>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<QueueForLfg>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DequeueFromLfg>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<RespondToLfgRoleCheck>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<LfgStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_pvp_messages(app: &mut App) {
    app.register_message::<QueryPvpStatus>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<QueueForBattleground>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<QueueForRatedPvp>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DequeueFromPvp>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<PvpStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_barber_shop_messages(app: &mut App) {
    app.register_message::<QueryBarberShopStatus>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ApplyBarberShopChanges>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<BarberShopStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_death_messages(app: &mut App) {
    app.register_message::<QueryDeathStatus>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ReleaseSpirit>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<ResurrectAtCorpse>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<AcceptSpiritHealerResurrection>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<UseStuckEscape>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DeathStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_durability_messages(app: &mut App) {
    app.register_message::<QueryDurabilityStatus>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DurabilityStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_collection_messages(app: &mut App) {
    app.register_message::<SummonMount>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DismissMount>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SummonPet>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<DismissPet>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CollectionStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

fn register_currency_messages(app: &mut App) {
    app.register_message::<EarnCurrency>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<SpendCurrency>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<CurrencyStateUpdate>()
        .add_direction(NetworkDirection::ServerToClient);
}

pub(super) fn register_channels(app: &mut App) {
    register_unreliable_channels(app);
    register_server_channels(app);
    register_bidirectional_channels(app);
}

fn register_unreliable_channels(app: &mut App) {
    add_channel::<MovementChannel>(
        app,
        ChannelMode::UnorderedUnreliable,
        NetworkDirection::ServerToClient,
    );
    add_channel::<InputChannel>(
        app,
        ChannelMode::UnorderedUnreliable,
        NetworkDirection::ClientToServer,
    );
}

fn register_server_channels(app: &mut App) {
    add_reliable_channel::<TerrainChannel>(app, NetworkDirection::ServerToClient);
}

fn register_bidirectional_channels(app: &mut App) {
    add_reliable_channel::<CombatChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<ChatChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<AuthChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<AuctionChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<TradeChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<TalentChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<InspectChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<DuelChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<ProfessionChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<ReputationChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<AchievementChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<WorldMapChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<RestChannel>(app, NetworkDirection::ServerToClient);
    add_reliable_channel::<FriendsChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<GuildChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<WhoChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<CalendarChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<IgnoreChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<LfgChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<PvpChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<BarberShopChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<DeathChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<DurabilityChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<CollectionChannel>(app, NetworkDirection::Bidirectional);
    add_reliable_channel::<CurrencyChannel>(app, NetworkDirection::Bidirectional);
}

fn add_reliable_channel<C: Channel>(app: &mut App, direction: NetworkDirection) {
    add_channel::<C>(app, ChannelMode::OrderedReliable(default()), direction);
}

fn add_channel<C: Channel>(app: &mut App, mode: ChannelMode, direction: NetworkDirection) {
    app.add_channel::<C>(ChannelSettings { mode, ..default() })
        .add_direction(direction);
}

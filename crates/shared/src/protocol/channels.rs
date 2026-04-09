/// Unreliable movement position updates, server-to-client only.
pub struct MovementChannel;

/// Reliable ordered channel for combat events, bidirectional.
pub struct CombatChannel;

/// Reliable ordered channel for chat messages, bidirectional.
pub struct ChatChannel;

/// Unreliable client-to-server channel for movement inputs.
pub struct InputChannel;

/// Reliable ordered channel for server-to-client terrain loading commands.
pub struct TerrainChannel;

/// Reliable ordered channel for authentication and character management, bidirectional.
pub struct AuthChannel;

/// Reliable ordered channel for auction house operations, bidirectional.
pub struct AuctionChannel;

/// Reliable ordered channel for trade operations, bidirectional.
pub struct TradeChannel;

/// Reliable ordered channel for talent operations, bidirectional.
pub struct TalentChannel;

/// Reliable ordered channel for profession operations, bidirectional.
pub struct ProfessionChannel;

/// Reliable ordered channel for reputation updates, bidirectional.
pub struct ReputationChannel;

/// Reliable ordered channel for achievement updates, bidirectional.
pub struct AchievementChannel;

/// Reliable ordered channel for world map updates, bidirectional.
pub struct WorldMapChannel;

/// Reliable ordered channel for resting state updates, bidirectional.
pub struct RestChannel;

/// Reliable ordered channel for friends updates, bidirectional.
pub struct FriendsChannel;

/// Reliable ordered channel for guild updates, bidirectional.
pub struct GuildChannel;

/// Reliable ordered channel for who-list updates, bidirectional.
pub struct WhoChannel;

/// Reliable ordered channel for calendar updates, bidirectional.
pub struct CalendarChannel;

/// Reliable ordered channel for ignore list updates, bidirectional.
pub struct IgnoreChannel;

/// Reliable ordered channel for LFG updates, bidirectional.
pub struct LfgChannel;

/// Reliable ordered channel for PVP updates, bidirectional.
pub struct PvpChannel;

/// Reliable ordered channel for barber shop updates, bidirectional.
pub struct BarberShopChannel;

/// Reliable ordered channel for collection updates, bidirectional.
pub struct CollectionChannel;

/// Reliable ordered channel for currency updates, bidirectional.
pub struct CurrencyChannel;

/// Reliable ordered channel for inspect operations, bidirectional.
pub struct InspectChannel;

/// Reliable ordered channel for duel operations, bidirectional.
pub struct DuelChannel;

/// Reliable ordered channel for death and respawn operations, bidirectional.
pub struct DeathChannel;

/// Reliable ordered channel for equipment durability updates, bidirectional.
pub struct DurabilityChannel;

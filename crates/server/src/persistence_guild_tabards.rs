//! Additive guild-emblem migration: legacy guilds have no design; bank/rank rows stay intact.
//! TC Guild::HandleSetEmblem charges10gold per save, only to the Guild Master.
use redb::{Database, ReadableDatabase, TableDefinition};
use shared::guild_creation::{GuildTabardDesign, TABARD_COST};

pub(crate) const DESIGNS: TableDefinition<u32, &[u8]> =
    TableDefinition::new("guild_tabard_designs");

pub fn load_design(db: &Database, guild_id: u32) -> Option<GuildTabardDesign> {
    let read = db.begin_read().expect("read guild tabard design");
    let table = read.open_table(DESIGNS).expect("open guild tabard designs");
    table
        .get(guild_id)
        .expect("read guild tabard design row")
        .map(|row| serde_json::from_slice(row.value()).expect("decode guild tabard design"))
}

/// Vendor and catalog validation precede this durable operation on the exclusive server World.
/// Gold and design share one commit: failed authority/funds never changes either record.
pub fn register(db: &Database, character: u64, design: GuildTabardDesign) -> Result<u32, String> {
    let membership = super::load_character_guild(db, character).ok_or("You are not in a guild.")?;
    let guild = super::banks::load_guild(db, membership.guild_id);
    if super::banks::load_guild_member_rank(db, &guild, character) != 0 {
        return Err("Only guild leaders can create emblems.".into());
    }
    let mut record = super::inventory_store::load_inventory_record(db, character)
        .ok_or("Character inventory is unavailable.")?;
    if record.inventory.gold < u64::from(TABARD_COST) {
        return Err("You can't afford to do that.".into());
    }
    record.inventory.gold -= u64::from(TABARD_COST);
    let encoded = serde_json::to_vec(&design).expect("encode guild tabard design");
    let write = db.begin_write().expect("begin guild tabard registration");
    write
        .open_table(DESIGNS)
        .expect("open guild tabard designs")
        .insert(membership.guild_id, encoded.as_slice())
        .expect("save guild tabard design");
    super::inventory_store::write_inventory_record(&write, character, &record);
    write
        .commit()
        .expect("commit guild tabard design and payment");
    Ok(membership.guild_id)
}

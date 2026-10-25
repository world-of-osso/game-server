//! TC a352b1fa `SpellHistory::SaveToDB`/`LoadFromDB`: a player's spell and category
//! cooldowns persist as wallclock deadlines (`character_spell_cooldown`), and
//! `SendSpellHistory` replays the active ones at login. Global cooldowns, school
//! locks and charges are not persisted.

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use redb::{Database, ReadableDatabase, TableDefinition, WriteTransaction};
use shared::protocol::SpellCooldownUpdate;

use super::{SpellHistory, history_now, send_to_owner};
use crate::networking::PlayerLink;
use crate::persistence::{ClientId, GameDatabase};

/// Character id -> its cooldown deadlines.
pub(crate) const SPELL_COOLDOWNS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_spell_cooldowns");

#[derive(bitcode::Encode, bitcode::Decode, Debug, Default, PartialEq)]
struct StoredCooldowns {
    /// (spell, ready at unix ms)
    spells: Vec<(u32, u64)>,
    /// (category, spell that started it, ready at unix ms)
    categories: Vec<(u32, u32, u64)>,
}

impl StoredCooldowns {
    fn retain_unexpired(&mut self, now_ms: u64) {
        self.spells.retain(|&(_, at)| at > now_ms);
        self.categories.retain(|&(_, _, at)| at > now_ms);
    }

    fn is_empty(&self) -> bool {
        self.spells.is_empty() && self.categories.is_empty()
    }
}

pub(crate) fn create_table(txn: &WriteTransaction) {
    txn.open_table(SPELL_COOLDOWNS)
        .expect("create character spell cooldowns");
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after the Unix epoch")
        .as_millis() as u64
}

/// History seconds -> unix ms, or None when already ready.
fn to_unix_ms(ready_at: f64, now: f64, now_ms: u64) -> Option<u64> {
    (ready_at > now).then(|| now_ms + ((ready_at - now) * 1000.0).round() as u64)
}

fn to_history(at_ms: u64, now: f64, now_ms: u64) -> f64 {
    now + at_ms.saturating_sub(now_ms) as f64 / 1000.0
}

fn stored(history: &SpellHistory, now: f64, now_ms: u64) -> StoredCooldowns {
    let mut cooldowns = StoredCooldowns {
        spells: history
            .spell_ready_at
            .iter()
            .filter_map(|(&spell, &at)| Some((spell, to_unix_ms(at, now, now_ms)?)))
            .collect(),
        categories: history
            .category_ready_at
            .iter()
            .filter_map(|(&category, &at)| {
                let spell = history.category_spell_ids.get(&category).copied()?;
                Some((category, spell, to_unix_ms(at, now, now_ms)?))
            })
            .collect(),
    };
    cooldowns.spells.sort_unstable();
    cooldowns.categories.sort_unstable();
    cooldowns
}

fn write(db: &Database, character_id: u64, cooldowns: &StoredCooldowns) {
    let txn = db.begin_write().expect("begin spell cooldown save");
    {
        let mut table = txn
            .open_table(SPELL_COOLDOWNS)
            .expect("open spell cooldowns");
        if cooldowns.is_empty() {
            table.remove(character_id).expect("remove spell cooldowns");
        } else {
            table
                .insert(character_id, bitcode::encode(cooldowns).as_slice())
                .expect("write spell cooldowns");
        }
    }
    txn.commit().expect("commit spell cooldowns");
}

/// The character's unexpired cooldowns; expired rows are pruned from the store.
fn load(db: &Database, character_id: u64, now_ms: u64) -> StoredCooldowns {
    let mut cooldowns: StoredCooldowns = {
        let txn = db.begin_read().expect("begin spell cooldown load");
        let table = txn
            .open_table(SPELL_COOLDOWNS)
            .expect("open spell cooldowns");
        let Some(row) = table.get(character_id).expect("read spell cooldowns") else {
            return StoredCooldowns::default();
        };
        bitcode::decode(row.value()).expect("decode spell cooldowns")
    };
    let before = (cooldowns.spells.len(), cooldowns.categories.len());
    cooldowns.retain_unexpired(now_ms);
    if (cooldowns.spells.len(), cooldowns.categories.len()) != before {
        write(db, character_id, &cooldowns);
    }
    cooldowns
}

/// Save the player's active cooldowns when it leaves the world.
pub(crate) fn save(world: &World, player: Entity) {
    let (Some(db), Some(client), Some(history)) = (
        world.get_resource::<GameDatabase>(),
        world.get::<ClientId>(player),
        world.get::<SpellHistory>(player),
    ) else {
        return;
    };
    write(
        &db.db,
        client.0,
        &stored(history, history_now(world), unix_ms()),
    );
}

/// A player linked to a connection gets its saved cooldowns and the client is told.
pub(crate) fn queue_restore(
    entered: Query<&PlayerLink, Added<PlayerLink>>,
    mut commands: Commands,
) {
    for link in &entered {
        let player = link.0;
        commands.queue(move |world: &mut World| restore(world, player));
    }
}

fn restore(world: &mut World, player: Entity) {
    let (Some(db), Some(client)) = (
        world.get_resource::<GameDatabase>(),
        world.get::<ClientId>(player),
    ) else {
        return;
    };
    let now_ms = unix_ms();
    let saved = load(&db.db, client.0, now_ms);
    let now = history_now(world);
    let Ok(mut entity) = world.get_entity_mut(player) else {
        return;
    };
    let mut history = entity.entry::<SpellHistory>().or_default();
    let mut history = history.get_mut();
    for (spell, at) in saved.spells {
        let ready = history.spell_ready_at.entry(spell).or_default();
        *ready = ready.max(to_history(at, now, now_ms));
    }
    for (category, spell, at) in saved.categories {
        let ready = history.category_ready_at.entry(category).or_default();
        *ready = ready.max(to_history(at, now, now_ms));
        history.category_spell_ids.entry(category).or_insert(spell);
    }
    let updates = active_updates(&history, now);
    for update in updates {
        send_to_owner(world, player, update);
    }
}

/// One update per spell still cooling down, with the category it started.
fn active_updates(history: &SpellHistory, now: f64) -> Vec<SpellCooldownUpdate> {
    let remaining_ms = |at: f64| ((at - now).max(0.0) * 1000.0).round() as u32;
    let mut updates: Vec<_> = history
        .spell_ready_at
        .iter()
        .filter(|&(_, &at)| at > now)
        .map(|(&spell_id, &at)| {
            let category = history
                .category_spell_ids
                .iter()
                .find(|&(category, &started)| {
                    started == spell_id
                        && history
                            .category_ready_at
                            .get(category)
                            .is_some_and(|&ready| ready > now)
                })
                .map(|(&category, _)| category);
            let category_ms = category.map_or(0, |category| {
                remaining_ms(history.category_ready_at[&category])
            });
            let remaining = remaining_ms(at).max(category_ms);
            SpellCooldownUpdate {
                spell_id,
                category: category.unwrap_or(0),
                duration_ms: remaining,
                remaining_ms: remaining,
                is_gcd: false,
            }
        })
        .collect();
    updates.sort_unstable_by_key(|update| update.spell_id);
    updates
}

/// Unexpired (spell, ready unix ms) and (category, spell, ready unix ms) for fixtures.
#[cfg(test)]
pub(crate) fn stored_deadlines(
    db: &Database,
    character_id: u64,
) -> (Vec<(u32, u64)>, Vec<(u32, u32, u64)>) {
    let cooldowns = load(db, character_id, unix_ms());
    (cooldowns.spells, cooldowns.categories)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored_row(db: &Database, character_id: u64) -> Option<StoredCooldowns> {
        let txn = db.begin_read().unwrap();
        let table = txn.open_table(SPELL_COOLDOWNS).unwrap();
        let row = table.get(character_id).unwrap()?;
        Some(bitcode::decode(row.value()).unwrap())
    }

    #[test]
    fn spell_cooldown_load_prunes_expired_rows_from_the_store() {
        let db = crate::persistence::temp_database();
        let now_ms = 1_800_000_000_000;
        write(
            &db,
            7,
            &StoredCooldowns {
                spells: vec![(18499, now_ms - 1), (28027, now_ms + 5_000)],
                categories: vec![(1174, 28027, now_ms + 5_000), (99, 18499, now_ms)],
            },
        );
        let unexpired = StoredCooldowns {
            spells: vec![(28027, now_ms + 5_000)],
            categories: vec![(1174, 28027, now_ms + 5_000)],
        };
        assert_eq!(load(&db, 7, now_ms), unexpired);
        assert_eq!(stored_row(&db, 7), Some(unexpired));

        assert_eq!(load(&db, 7, now_ms + 5_000), StoredCooldowns::default());
        assert_eq!(stored_row(&db, 7), None, "a fully expired row is removed");
    }
}

use super::{Config, model::EconomyState};
use crate::persistence::NEXT_IDS;
use redb::{Database, ReadableTable};
use std::collections::BTreeMap;

#[derive(Clone)]
struct IdRange {
    used: u64,
    high: u64,
}

/// IDs become usable only after the FIFO reservation transaction commits.
#[derive(Clone)]
pub(super) struct ReservedIds(BTreeMap<&'static str, IdRange>);

impl ReservedIds {
    pub(super) fn take(&mut self, key: &'static str) -> Result<u64, String> {
        let range = self
            .0
            .get_mut(key)
            .ok_or_else(|| format!("unreserved ID kind {key}"))?;
        if range.used == range.high {
            return Err(format!("economy {key} reservation exhausted"));
        }
        range.used += 1;
        Ok(range.used)
    }
}

pub(super) fn reservation_counts(
    config: &Config,
    state: &EconomyState,
) -> Result<[(&'static str, u64); 4], String> {
    let gatherables = u64::try_from(config.gatherables.len()).map_err(|error| error.to_string())?;
    let listings = gatherables
        .checked_mul(u64::from(config.gatherer_listing_limit))
        .ok_or("economy listing reservation overflows")?;
    let characters = config
        .gatherables
        .iter()
        .filter(|item| !state.gatherers.contains_key(item))
        .count() as u64;
    let mails = config.merchants.iter().try_fold(0u64, |sum, merchant| {
        let ingredients = merchant
            .recipes
            .iter()
            .map(|recipe| recipe.ingredients.len())
            .max()
            .unwrap_or(0) as u64;
        sum.checked_add(ingredients)
            .ok_or("economy mail reservation overflows")
    })?;
    Ok([
        ("character", characters),
        ("item_instance", listings),
        ("auction", listings),
        ("mail", mails),
    ])
}

pub(super) fn reserve(
    db: &Database,
    counts: [(&'static str, u64); 4],
) -> Result<ReservedIds, String> {
    let transaction = db.begin_write().map_err(|error| error.to_string())?;
    let mut ranges = BTreeMap::new();
    {
        let mut table = transaction
            .open_table(NEXT_IDS)
            .map_err(|error| error.to_string())?;
        for (key, count) in counts {
            let used = table
                .get(key)
                .map_err(|error| error.to_string())?
                .map_or(0, |row| row.value());
            let high = used
                .checked_add(count)
                .ok_or_else(|| format!("{key} ID high-water mark exhausted"))?;
            table.insert(key, high).map_err(|error| error.to_string())?;
            ranges.insert(key, IdRange { used, high });
        }
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(ReservedIds(ranges))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence;

    #[test]
    fn economy_restart_never_reuses_unspent_reserved_ids() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::create(file.path()).unwrap();
        persistence::create_tables(&db);
        let counts = [
            ("character", 2),
            ("item_instance", 4),
            ("auction", 3),
            ("mail", 2),
        ];
        let mut ids = reserve(&db, counts).unwrap();
        for (key, _) in counts {
            assert_eq!(ids.take(key).unwrap(), 1);
        }
        drop(ids);
        drop(db);
        let db = Database::open(file.path()).unwrap();
        for (key, count) in counts {
            assert_eq!(persistence::next_id(&db, key), count + 1);
        }
    }

    #[test]
    fn economy_reserved_ids_do_not_collide_with_other_allocators() {
        let db = persistence::temp_database();
        assert_eq!(persistence::next_id(&db, "item_instance"), 1);
        let mut ids = reserve(
            &db,
            [
                ("character", 0),
                ("item_instance", 2),
                ("auction", 0),
                ("mail", 0),
            ],
        )
        .unwrap();
        assert_eq!(ids.take("item_instance").unwrap(), 2);
        assert_eq!(persistence::next_id(&db, "item_instance"), 4);
        assert_eq!(ids.take("item_instance").unwrap(), 3);
        assert!(ids.take("item_instance").is_err());
    }
}

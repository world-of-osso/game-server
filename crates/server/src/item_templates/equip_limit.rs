//! Authored Retail equip quantities. ItemSparse.LimitCategory supplies the key.
//! The pinned DB2 export is shared by every definition, never an inferred cap.
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipLimit {
    pub category: u32,
    pub quantity: u8,
}

static QUANTITIES: LazyLock<HashMap<u32, u8>> = LazyLock::new(|| {
    include_str!("../item_limit_categories.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let (category, quantity) = line.split_once('\t').expect("DB2 category/quantity row");
            (
                category.parse().expect("DB2 category id"),
                quantity.parse().expect("DB2 category quantity"),
            )
        })
        .collect()
});

pub(super) fn limit_from_category(category: u32) -> rusqlite::Result<Option<EquipLimit>> {
    if category == 0 {
        return Ok(None);
    }
    let quantity = QUANTITIES.get(&category).copied().ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(std::io::Error::other(format!(
                "ItemSparse.LimitCategory {category} has no authored ItemLimitCategory quantity"
            ))),
        )
    })?;
    Ok(Some(EquipLimit { category, quantity }))
}

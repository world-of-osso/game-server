//! Explicit best-guess news membership; never substitute the full DB2 catalog.
use chrono::Datelike;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const POLICY: &str = "TP_MONTHLY_CATALOG_WOWHEAD_NEWS_BEST_GUESS";
const EPOCH_POLICY: &str = "TP_PERKS_MONTH_EPOCH_2023_01_WOWHEAD_ANCHORED";

#[derive(Deserialize)]
struct Document {
    schema_version: u32,
    policy: String,
    month_epoch_policy: String,
    db2_build: String,
    months: Vec<Month>,
    permanent_vendor_item_ids: BTreeSet<u32>,
}
#[derive(Deserialize)]
struct Month {
    perks_month: i32,
    calendar: String,
    vendor_item_ids: BTreeSet<u32>,
    source: Source,
    supplemental_sources: Vec<Source>,
}
#[derive(Deserialize)]
struct Source {
    url: String,
    retrieved_date: String,
    sha256: String,
}

#[derive(Clone)]
pub(crate) struct Catalog {
    months: BTreeMap<i32, BTreeSet<u32>>,
    permanent: BTreeSet<u32>,
}

pub(super) struct Membership<'a> {
    pub(super) vendors: Option<&'a BTreeSet<u32>>,
    permanent: &'a BTreeSet<u32>,
    pub(super) perks_month: i32,
}
impl Membership<'_> {
    pub(super) fn contains(&self, vendor: u32, frozen: Option<u32>) -> bool {
        if frozen == Some(vendor) {
            return true;
        }
        self.vendors
            .is_some_and(|vendors| vendors.contains(&vendor) || self.permanent.contains(&vendor))
    }
    pub(super) fn gap(&self) -> Option<String> {
        self.vendors.is_none().then(|| {
            format!(
                "TP_MONTHLY_CATALOG_MONTH_UNCOVERED: PerksMonth {}",
                self.perks_month
            )
        })
    }
}
impl Catalog {
    pub(super) fn at(&self, now: i64) -> Membership<'_> {
        // ServerClock and Adventure Guide both use UTC month boundaries.
        let perks_month = chrono::DateTime::from_timestamp(now, 0)
            .map(|date| (date.year() - 2023) * 12 + date.month() as i32)
            .unwrap_or(i32::MIN);
        Membership {
            vendors: self.months.get(&perks_month),
            permanent: &self.permanent,
            perks_month,
        }
    }
}

pub(super) fn load(
    path: &Path,
    db2_build: &str,
    vendor_ids: &BTreeSet<u32>,
) -> Result<Catalog, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("TP_MONTHLY_CATALOG_LOAD {}: {error}", path.display()))?;
    let document: Document = serde_json::from_str(&text)
        .map_err(|error| format!("TP_MONTHLY_CATALOG_PARSE {}: {error}", path.display()))?;
    validate_document(&document, db2_build, vendor_ids)
        .map_err(|error| format!("TP_MONTHLY_CATALOG_INVALID {}: {error}", path.display()))?;
    Ok(Catalog {
        months: document
            .months
            .into_iter()
            .map(|month| (month.perks_month, month.vendor_item_ids))
            .collect(),
        permanent: document.permanent_vendor_item_ids,
    })
}

fn validate_document(
    document: &Document,
    db2_build: &str,
    vendor_ids: &BTreeSet<u32>,
) -> Result<(), String> {
    if document.schema_version != 1
        || document.policy != POLICY
        || document.month_epoch_policy != EPOCH_POLICY
    {
        return Err("unsupported schema, source policy or calendar epoch".into());
    }
    if document.db2_build != db2_build {
        return Err(format!(
            "DB2 build {} differs from world.db {db2_build}",
            document.db2_build
        ));
    }
    let mut months = BTreeSet::new();
    for month in &document.months {
        validate_month(month, vendor_ids)?;
        if !months.insert(month.perks_month) {
            return Err(format!("duplicate PerksMonth {}", month.perks_month));
        }
    }
    if months.is_empty() {
        return Err("no monthly records".into());
    }
    validate_permanent(document, &months, vendor_ids)
}

fn validate_month(month: &Month, vendor_ids: &BTreeSet<u32>) -> Result<(), String> {
    if month.perks_month < 1 {
        return Err(format!("invalid PerksMonth {}", month.perks_month));
    }
    let year = 2023 + (month.perks_month - 1).div_euclid(12);
    let calendar_month = (month.perks_month - 1).rem_euclid(12) + 1;
    if month.calendar != format!("{year:04}-{calendar_month:02}") {
        return Err(format!(
            "invalid calendar for PerksMonth {}",
            month.perks_month
        ));
    }
    if let Some(id) = month.vendor_item_ids.difference(vendor_ids).next() {
        return Err(format!(
            "PerksMonth {} references unknown vendor {id}",
            month.perks_month
        ));
    }
    for source in std::iter::once(&month.source).chain(&month.supplemental_sources) {
        validate_source(source, month.perks_month)?;
    }
    Ok(())
}

fn validate_source(source: &Source, perks_month: i32) -> Result<(), String> {
    let hash_valid =
        source.sha256.len() == 64 && source.sha256.bytes().all(|byte| byte.is_ascii_hexdigit());
    let date_valid = chrono::NaiveDate::parse_from_str(&source.retrieved_date, "%Y-%m-%d").is_ok();
    let url_valid = source.url.starts_with("https://www.wowhead.com/news/");
    if hash_valid && date_valid && url_valid {
        return Ok(());
    }
    Err(format!(
        "PerksMonth {perks_month} has invalid news provenance"
    ))
}

fn validate_permanent(
    document: &Document,
    months: &BTreeSet<i32>,
    vendor_ids: &BTreeSet<u32>,
) -> Result<(), String> {
    for id in &document.permanent_vendor_item_ids {
        if !vendor_ids.contains(id) {
            return Err(format!("unknown permanent vendor {id}"));
        }
        let appearances: BTreeSet<_> = document
            .months
            .iter()
            .filter(|month| month.vendor_item_ids.contains(id))
            .map(|month| month.perks_month)
            .collect();
        let continuous = appearances.first().is_some_and(|first| {
            months
                .range(*first..)
                .all(|month| appearances.contains(month))
        });
        if appearances.len() < 2 || !continuous {
            return Err(format!(
                "permanent vendor {id} lacks continuous multi-month evidence"
            ));
        }
    }
    Ok(())
}

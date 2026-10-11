//! Fixed-output orders: authored catalog, durable escrow and the normal craft pipeline.
mod cast;
mod catalog;
#[cfg(test)]
mod mail_compat_tests;
mod rules;
mod runtime;
mod store;

pub(crate) use cast::{
    begin_fulfill, cast_requirements, complete_effect, interrupt, is_order_cast,
};
pub(crate) use catalog::load_catalog;
#[cfg(test)]
pub(crate) use rules::{PlaceOrder, browse, cancel, claim, expire, place, release};
pub(crate) use runtime::CraftingOrdersPlugin;
pub(crate) use store::all;

#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub(crate) struct Order {
    pub id: u64,
    pub spell_id: u32,
    pub skill_line: u32,
    pub item_id: u32,
    pub count: u32,
    /// Wire kind: public0, guild1, personal2. Stored separately from serde wire ABI.
    pub kind: u8,
    pub customer: u64,
    pub recipient: Option<u64>,
    pub guild: Option<u32>,
    pub crafter: Option<u64>,
    pub commission: u64,
    /// Item, authored requirement, customer contribution.
    pub reagents: Vec<(u32, u32, u32)>,
    pub held: Vec<crate::persistence::InventoryItemData>,
    pub expires_at: u64,
    pub note: String,
    /// Open0, claimed1, completed2, cancelled/expired3.
    pub status: u8,
}

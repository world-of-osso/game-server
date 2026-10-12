//! Authoritative Retail profession specialization progression.
use super::PlayerProfessions;

mod data;
mod descriptions;
mod runtime;
mod state;
mod store;
mod view;

pub(crate) use data::{SpecializationData, load_knowledge_quest_rewards, load_specialization_data};
pub(crate) use runtime::{SpecializationsPlugin, reload_knowledge_wallet};
use state::{
    CommitSelection, SpecializationActor, SpecializationCommit, SpecializationState,
    calculate_commit,
};
pub(crate) use store::{initialize_store, write_currency_wallet};
use view::build_snapshot;

pub(crate) fn award_knowledge(
    catalog: &SpecializationData,
    professions: &PlayerProfessions,
    wallet: &shared::currency::CurrencyWallet,
    quest: u32,
) -> Result<shared::currency::CurrencyWallet, String> {
    state::award_knowledge(catalog, professions, wallet, quest)
}

#[cfg(test)]
#[path = "specializations/live_fixture.rs"]
mod live_fixture;

#[cfg(test)]
#[path = "specializations/tests.rs"]
mod tests;

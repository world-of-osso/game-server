//! Shared Hunter fixture facts, moved unchanged from the BM adapter.
use super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::NativeFacts;
use server::simc_apl::{Action, ActionFacts, Facts};

pub(super) struct HunterFacts<'a> {
    pub native: NativeFacts<'a>,
    pub equipment: &'a Equipment,
    pub potion: &'a str,
}

fn equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("equipped.") || name.starts_with("set_bonus.")
}

impl Facts for HunterFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if equipment_operand(name) {
            return self.equipment.fact(name, &self.native);
        }
        // SimC76f8118 sc_hunter.cpp7340–7342,1055: configured default is true.
        if name == "max_prio_damage" {
            return Ok(1.0);
        }
        // SimC76f8118 player.cpp12317–12335: the third BM tiered entry.
        if name == "apex.3" {
            return self.native.value("talent.natures_ally_3");
        }
        // SimC76f8118 sc_hunter.cpp7368–7374: any actual ready-beast buff.
        if name == "howl_summon.ready" {
            let ready =
                ["wyvern", "boar", "bear"]
                    .into_iter()
                    .try_fold(false, |ready, beast| {
                        self.native
                            .value(&format!("buff.howl_{beast}.up"))
                            .map(|value| ready || value != 0.0)
                    })?;
            return Ok(if ready { 1.0 } else { 0.0 });
        }
        if let Some(alias) = name.strip_prefix("potion.") {
            return Ok(configured_potion(self.potion, alias));
        }
        self.native.value(name)
    }
}

fn configured_potion(potion: &str, alias: &str) -> f64 {
    // SimC76f8118 player.cpp12376–12409 uses the configured potion,
    // irrespective of whether the current inventory admits a use.
    let configured = potion
        .rsplit_once('_')
        .filter(|(_, suffix)| suffix.parse::<u8>().is_ok())
        .map_or(potion, |(base, _)| base);
    if configured.eq_ignore_ascii_case(alias) || potion.eq_ignore_ascii_case(alias) {
        1.0
    } else {
        0.0
    }
}

impl ActionFacts for HunterFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if equipment_operand(name)
            || name.starts_with("potion.")
            || matches!(name, "max_prio_damage" | "apex.3" | "howl_summon.ready")
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}


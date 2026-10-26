//! Periodic facts read the real snapshot field and the same fresh native producers.
use super::*;

impl NativeFacts<'_> {
    pub(super) fn live_persistent_multiplier(&self, alias: &str) -> Result<f64, String> {
        let id = bound_spell(&self.bindings.buffs, alias, "dot")?;
        let spell = self.spell(id)?;
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .ok_or_else(|| format!("native periodic spell{id} lacks effect0"))?;
        crate::auras::periodic_snapshot::multiplier(self.world, self.actor, spell, effect)
            .map(f64::from)
            .ok_or_else(|| {
                format!(
                    "NATIVE_DOT_PERSISTENT_UNAVAILABLE: {alias} has no qualified snapshot producer"
                )
            })
    }

    pub(super) fn stored_persistent_multiplier(
        &self,
        alias: &str,
        action: Option<&Action>,
    ) -> Result<f64, String> {
        let id = bound_spell(&self.bindings.buffs, alias, "dot")?;
        self.spell(id)?;
        let active = self.read_owned_auras(self.selected_target(action)?, id);
        // SimC76f8118 dot.cpp568–573: an absent dot state has multiplier zero.
        if active.is_empty() {
            return Ok(0.0);
        }
        active
            .iter()
            .find_map(|aura| {
                aura.effects
                    .iter()
                    .find_map(|effect| effect.persistent_multiplier)
            })
            .map(f64::from)
            .ok_or_else(|| {
                format!("NATIVE_DOT_PERSISTENT_UNAVAILABLE: {alias} has no stored snapshot")
            })
    }
}

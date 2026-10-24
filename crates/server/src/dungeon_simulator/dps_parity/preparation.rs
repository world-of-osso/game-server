//! Native setup advances real cast/GCD state before fixture engagement.
use super::Run;

// Bound setup failures; advancing frames must not become an endless cast retry.
const MAX_SETUP_TICKS: u32 = 600;

pub(super) fn warm_up_reference(run: &mut Run) -> Result<(), String> {
    for _ in 0..4 {
        run.app.update();
    }
    for _ in 0..MAX_SETUP_TICKS {
        if !setup_cast_or_gcd_active(run)? {
            return Ok(());
        }
        run.app.update();
    }
    Err(format!(
        "native fixture setup exceeded30s before engagement; actor state {}",
        super::report::engagement(run.app.world(), run.actor, &run.targets)
    ))
}

fn setup_cast_or_gcd_active(run: &Run) -> Result<bool, String> {
    let world = run.app.world();
    if world.get::<shared::casting::CastState>(run.actor).is_some() {
        return Ok(true);
    }
    let Some(history) = world.get::<crate::spell_cast::SpellHistory>(run.actor) else {
        return Ok(false);
    };
    let Some(id) = history.last_successful_cast() else {
        return Ok(false);
    };
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .ok_or_else(|| format!("native setup last cast{id} missing from SpellStore"))?;
    Ok(history.on_gcd(spell, crate::spell_cast::history_now(world)))
}

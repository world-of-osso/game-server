//! Retail owned dyed stacks; total stored counts include these quantities exactly once.
use super::*;
use shared::protocol::{DecorDye, StoredDecorVariant};

pub(super) fn take_variant(
    state: &mut State,
    account: u64,
    decor_id: u32,
    variant_id: Option<u64>,
) -> Result<Vec<DecorDye>, String> {
    let Some(id) = variant_id else {
        let total = state
            .storage
            .get(&account)
            .and_then(|stored| stored.get(&decor_id))
            .copied()
            .unwrap_or(0);
        let dyed = state
            .storage_variants
            .get(&account)
            .into_iter()
            .flatten()
            .filter(|variant| variant.decor_id == decor_id)
            .try_fold(0u32, |total, variant| {
                total
                    .checked_add(variant.quantity)
                    .ok_or("Stored variant quantity overflow.")
            })?;
        if total <= dyed {
            return Err("No stored undyed copies of this decor remain.".into());
        }
        return Ok(Vec::new());
    };
    let variants = state
        .storage_variants
        .get_mut(&account)
        .ok_or("You do not own this decor variant.")?;
    let index = variants
        .iter()
        .position(|variant| variant.variant_id == id && variant.decor_id == decor_id)
        .ok_or("You do not own this decor variant.")?;
    let variant = &mut variants[index];
    variant.quantity = variant
        .quantity
        .checked_sub(1)
        .ok_or("No stored copies of this variant remain.")?;
    let dyes = variant.dyes.clone();
    if variant.quantity == 0 {
        variants.remove(index);
    }
    Ok(dyes)
}

pub(super) fn store_variant(
    state: &mut State,
    account: u64,
    decor: &PlacedDecor,
) -> Result<(), String> {
    if decor.dyes.is_empty() {
        return Ok(());
    }
    let variants = state.storage_variants.entry(account).or_default();
    if let Some(variant) = variants
        .iter_mut()
        .find(|variant| variant.decor_id == decor.decor_id && variant.dyes == decor.dyes)
    {
        variant.quantity = variant
            .quantity
            .checked_add(1)
            .ok_or("Stored variant quantity overflow.")?;
        return Ok(());
    }
    let variant_id = state.next_instance_id;
    state.next_instance_id = variant_id
        .checked_add(1)
        .ok_or("Decor variant identity range exhausted.")?;
    variants.push(StoredDecorVariant {
        variant_id,
        decor_id: decor.decor_id,
        quantity: 1,
        dyes: decor.dyes.clone(),
    });
    Ok(())
}

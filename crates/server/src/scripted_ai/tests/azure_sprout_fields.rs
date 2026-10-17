//! Native Sprout self-aura395 must create an actual same-copy owned field.
use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;

#[test]
#[ignore = "native aura395 field boundary awaiting behavioral RED"]
fn azure_leymor_native_sprout_self_auras_create_fourteen_same_copy_fields() {
    const SPROUT_AURA: u32 = 374161;
    let (mut h, leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let children = h.creatures_of(190509);
    assert_eq!(children.len(), 14);
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    for &child in &children {
        assert!(h.has_aura(child, SPROUT_AURA));
        assert_eq!(h.app.world().get::<InMap>(child), Some(&map));
    }
    let fields: Vec<_> = h
        .app
        .world_mut()
        .query::<(&SpellAreaTrigger, &Position, &InMap)>()
        .iter(h.app.world())
        .filter(|(field, _, _)| field.spell_id == SPROUT_AURA)
        .map(|(field, position, copy)| (field.caster, *position, *copy))
        .collect();
    assert_eq!(
        fields.len(),
        14,
        "one actual aura395-owned field per native self-aura"
    );
    for child in children {
        let owned: Vec<_> = fields
            .iter()
            .filter(|(caster, _, _)| *caster == child)
            .collect();
        assert_eq!(owned.len(), 1);
        let (_, position, copy) = owned[0];
        assert_eq!(*copy, map);
        let owner_position = h.app.world().get::<Position>(child).unwrap();
        assert_eq!(
            (position.x, position.y, position.z),
            (owner_position.x, owner_position.y, owner_position.z)
        );
    }
    // Entry/exit, scale curve,374523 action and aura/death cleanup need separate proof.
}

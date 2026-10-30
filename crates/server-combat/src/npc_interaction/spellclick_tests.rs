use super::spellclick::{cast_spellclick, load_spellclick_catalog};
use crate::spell_cast::start_spellclick_cast;
use crate::spell_cast::test_harness::harness;
use shared::casting::CastState;

#[test]
fn nalorakk_berry_spellclick_runs_native_cast_time() {
    let mut h = harness(Vec::new(), &[1239001, 1239428], 2).unwrap();
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_spellclick_catalog(&conn).unwrap());
    let player = h.caster;
    let berry = h.npc;
    cast_spellclick(h.app.world_mut(), player, berry, 244941);
    let cast = h
        .app
        .world()
        .get::<CastState>(player)
        .expect("berry cast starts");
    assert_eq!(cast.spell_id, 1239001);
    h.advance(20);
    assert!(h.app.world().get::<CastState>(player).is_some());
    h.advance(80);
    assert!(h.app.world().get::<CastState>(player).is_none());
    assert!(!crate::auras::has_aura(h.app.world(), player, 1239428));
}

#[test]
fn nalorakk_fishing_spells_complete_without_carry_aura() {
    for spell in [1239040, 1239227] {
        let mut h = harness(Vec::new(), &[spell, 1239428], 2).unwrap();
        let player = h.caster;
        let target = h.npc;
        h.app
            .world_mut()
            .entity_mut(player)
            .insert(shared::components::CombatStatus(false));
        let result = start_spellclick_cast(h.app.world_mut(), player, spell, target);
        assert!(result.is_ok(), "offering {spell}: {result:?}");
        h.advance(240);
        assert!(
            h.inbox.log.iter().any(|event| {
                event.spell_id == Some(spell)
                    && event.kind == shared::protocol::CombatLogKind::CastSuccess
            }),
            "offering {spell} did not complete"
        );
        assert!(
            !crate::auras::has_aura(h.app.world(), player, 1239428),
            "offering {spell}"
        );
    }
}

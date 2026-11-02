use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;

#[test]
fn beacon_transfers_fully_overhealed_direct_healing() {
    let mut h = harness(Vec::new(), &[53563, 53651, 53652, 19750], 2).unwrap();
    let caster = h.caster;
    let beacon = spawn_player(&mut h.app, 2, 4.0);
    let ally = spawn_player(&mut h.app, 2, 3.0);
    let mut online = crate::group::OnlineView::new();
    for (index, member) in [caster, beacon, ally].into_iter().enumerate() {
        let id = index as u64 + 1;
        h.app
            .world_mut()
            .entity_mut(member)
            .insert(crate::persistence::ClientId(id));
        online.insert(
            id,
            crate::group::OnlineMember {
                entity: member.to_bits(),
                name: format!("Member{id}"),
                class: 2,
                level: 10,
                portrait: default(),
                dungeon_difficulty: 1,
                raid_difficulty: crate::maps::RaidDifficulty::default(),
            },
        );
    }
    let mut registry = crate::group::GroupRegistry::default();
    for id in 2..=3 {
        registry.invite(1, &format!("Member{id}"), &online);
        registry.respond_invite(id, true);
    }
    h.app.insert_resource(registry);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: beacon,
            spell_id: 53563,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.app.world_mut().get_mut::<Health>(beacon).unwrap().current = 100.0;
    let full = h.health(ally);
    let spell = world_db().unwrap().store.get(19750).unwrap();
    let ctx = super::effects::EffectCtx {
        caster,
        spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::apply_fixed_heal(h.app.world_mut(), &ctx, ally, 100.0);
    assert_eq!(h.health(ally), full);
    assert_eq!(h.health(beacon), 110.0);
}

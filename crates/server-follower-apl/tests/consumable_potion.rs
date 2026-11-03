use bevy::prelude::*;
use server_combat::spell_cast::test_harness::harness_with_plugins;
use server_follower_apl::literal_rotation::{PrecombatPoll, execution::Execution};
use server_follower_apl::{FollowerApl, SpecRotation};

struct Idle;
impl SpecRotation for Idle {
    fn action_spells(&self) -> Vec<u32> {
        Vec::new()
    }
    fn poll_precombat(
        &mut self,
        _: &mut World,
        _: Entity,
        _: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        Ok(PrecombatPoll::Finished)
    }
    fn execute(
        &mut self,
        _: &mut World,
        _: Entity,
        _: server_combat::simc_apl::Action,
        _: Execution,
    ) -> Result<(), String> {
        Ok(())
    }
    fn start_combat_at(&mut self, _: f64) {}
    fn drive(&mut self, _: &mut World, _: Entity, _: &[Entity]) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn follower_uses_potion_on_first_pull_and_waits_for_cooldown() {
    let mut h = harness_with_plugins(vec![], &[], 8, false, |_| {}).unwrap();
    h.app.insert_resource(
        server_combat::item_templates::load_item_templates(
            &server_combat::test_data::world_db_path(),
        )
        .unwrap(),
    );
    let native = server_combat::spell_cast::test_harness::world_db().unwrap();
    h.app.insert_resource(native.spell_store().clone());
    let actor = h.caster;
    let target = h.npc;
    let item = server_combat::consumables::ItemUse::consumable(h.app.world(), 241308).unwrap();
    h.app.world_mut().entity_mut(actor).insert((
        server_combat::consumables::Potion(item),
        server_combat::consumables::WclPotion,
    ));
    let mut apl = FollowerApl::new(Box::new(Idle));
    apl.tick(h.app.world_mut(), actor, &[]).unwrap();
    assert!(!server_combat::auras::has_aura(
        h.app.world(),
        actor,
        1236616
    ));
    server_combat::combat_refs::set_in_combat_with(h.app.world_mut(), actor, target);
    apl.tick(h.app.world_mut(), actor, &[target]).unwrap();
    assert!(server_combat::auras::has_aura(
        h.app.world(),
        actor,
        1236616
    ));
    h.app
        .world_mut()
        .resource_mut::<bevy::time::Time<bevy::time::Real>>()
        .advance_by(std::time::Duration::from_secs(299));
    apl.tick(h.app.world_mut(), actor, &[target]).unwrap();
    assert!(!server_combat::consumables::use_wcl_potion(h.app.world_mut(), actor).unwrap());
    h.app
        .world_mut()
        .resource_mut::<bevy::time::Time<bevy::time::Real>>()
        .advance_by(std::time::Duration::from_secs(1));
    apl.tick(h.app.world_mut(), actor, &[target]).unwrap();
    assert!(!server_combat::consumables::use_wcl_potion(h.app.world_mut(), actor).unwrap());
}

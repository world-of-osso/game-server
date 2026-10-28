use super::dps_parity;

#[test]
#[ignore = "requires native Preservation profile and canonical DB"]
fn merithra_procs_from_essence_casts_not_mana_heals() {
    let _rng = crate::random::seed_scope(70448);
    let (profile, traits) = dps_parity::read_reference(1468).unwrap();
    let mut run = dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 70 },
    )
    .unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 1256577));
    for _ in 0..20 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 361469, Some(actor), false)
            .unwrap();
        for _ in 0..80 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 1256579),
            "Living Flame does not spend Essence"
        );
    }
    let mut procced = false;
    for _ in 0..25 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 364343, Some(actor), false)
            .unwrap();
        for _ in 0..240 {
            run.app.update();
        }
        if crate::auras::has_aura(run.app.world(), actor, 1256579) {
            procced = true;
            break;
        }
    }
    assert!(
        procced,
        "native 20% proc must arise from actual Essence-cost Echo casts"
    );
}

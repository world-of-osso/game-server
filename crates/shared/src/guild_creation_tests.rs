use super::*;

#[test]
fn purchase_charter_success() {
    let mut mgr = PetitionManager::new();
    let id = mgr
        .purchase_charter(100, "Test Guild", 5000, false, 1000)
        .unwrap();

    assert_eq!(id, 1);
    let petition = mgr.get(id).unwrap();
    assert_eq!(petition.owner, 100);
    assert_eq!(petition.guild_name, "Test Guild");
    assert!(petition.signatures.is_empty());
    assert_eq!(petition.created_at, 1000);
}

#[test]
fn purchase_costs_10_silver() {
    let mut mgr = PetitionManager::new();
    // Exactly 10 silver (1000 copper) — should succeed
    assert!(
        mgr.purchase_charter(100, "Guild", CHARTER_COST, false, 1000)
            .is_ok()
    );
}

#[test]
fn purchase_insufficient_funds() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        mgr.purchase_charter(100, "Guild", CHARTER_COST - 1, false, 1000),
        Err(PetitionError::InsufficientFunds)
    );
}

#[test]
fn purchase_already_guilded() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        mgr.purchase_charter(100, "Guild", 5000, true, 1000),
        Err(PetitionError::AlreadyInGuild)
    );
}

#[test]
fn purchase_already_has_petition() {
    let mut mgr = PetitionManager::new();
    mgr.purchase_charter(100, "First Guild", 5000, false, 1000)
        .unwrap();
    assert_eq!(
        mgr.purchase_charter(100, "Second Guild", 5000, false, 1001),
        Err(PetitionError::AlreadyHasPetition)
    );
}

#[test]
fn purchase_empty_name() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        mgr.purchase_charter(100, "", 5000, false, 1000),
        Err(PetitionError::InvalidName)
    );
}

#[test]
fn purchase_name_too_long() {
    let mut mgr = PetitionManager::new();
    let long = "x".repeat(MAX_GUILD_NAME_LEN + 1);
    assert_eq!(
        mgr.purchase_charter(100, &long, 5000, false, 1000),
        Err(PetitionError::InvalidName)
    );
}

#[test]
fn purchase_name_max_length_ok() {
    let mut mgr = PetitionManager::new();
    let name = "x".repeat(MAX_GUILD_NAME_LEN);
    assert!(mgr.purchase_charter(100, &name, 5000, false, 1000).is_ok());
}

#[test]
fn purchase_name_taken_by_existing_guild() {
    let mut mgr = PetitionManager::new();
    mgr.reserve_name("Stormwind Guard");
    assert_eq!(
        mgr.purchase_charter(100, "stormwind guard", 5000, false, 1000),
        Err(PetitionError::NameTaken)
    );
}

#[test]
fn purchase_name_taken_by_pending_petition() {
    let mut mgr = PetitionManager::new();
    mgr.purchase_charter(100, "Cool Guild", 5000, false, 1000)
        .unwrap();
    assert_eq!(
        mgr.purchase_charter(200, "cool guild", 5000, false, 1001),
        Err(PetitionError::NameTaken)
    );
}

#[test]
fn find_by_owner() {
    let mut mgr = PetitionManager::new();
    mgr.purchase_charter(100, "Guild", 5000, false, 1000)
        .unwrap();
    assert!(mgr.find_by_owner(100).is_some());
    assert!(mgr.find_by_owner(200).is_none());
}

#[test]
fn cancel_petition() {
    let mut mgr = PetitionManager::new();
    let id = mgr
        .purchase_charter(100, "Guild", 5000, false, 1000)
        .unwrap();
    mgr.cancel(id, 100).unwrap();
    assert!(mgr.is_empty());
}

#[test]
fn cancel_not_owner_fails() {
    let mut mgr = PetitionManager::new();
    let id = mgr
        .purchase_charter(100, "Guild", 5000, false, 1000)
        .unwrap();
    assert_eq!(mgr.cancel(id, 200), Err(PetitionError::NotOwner));
}

#[test]
fn cancel_not_found() {
    let mut mgr = PetitionManager::new();
    assert_eq!(mgr.cancel(999, 100), Err(PetitionError::NotFound));
}

#[test]
fn sequential_ids() {
    let mut mgr = PetitionManager::new();
    let id1 = mgr
        .purchase_charter(100, "Guild A", 5000, false, 1000)
        .unwrap();
    mgr.cancel(id1, 100).unwrap();
    let id2 = mgr
        .purchase_charter(200, "Guild B", 5000, false, 1001)
        .unwrap();
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

// --- Petition signing ---

fn petition_with_owner(mgr: &mut PetitionManager) -> u64 {
    mgr.purchase_charter(100, "Test Guild", 5000, false, 1000)
        .unwrap()
}

#[test]
fn sign_petition_success() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);

    let count = mgr.sign_petition(id, 200, false).unwrap();
    assert_eq!(count, 1);

    let petition = mgr.get(id).unwrap();
    assert!(petition.has_signed(200));
}

#[test]
fn sign_multiple_signatures() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);

    for i in 0..REQUIRED_SIGNATURES {
        let signer = 200 + i as u64;
        mgr.sign_petition(id, signer, false).unwrap();
    }

    let petition = mgr.get(id).unwrap();
    assert_eq!(petition.signatures.len(), REQUIRED_SIGNATURES);
    assert!(petition.has_enough_signatures());
}

#[test]
fn sign_guilded_player_fails() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);
    assert_eq!(
        mgr.sign_petition(id, 200, true),
        Err(PetitionError::AlreadyInGuild)
    );
}

#[test]
fn sign_owner_fails() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);
    assert_eq!(
        mgr.sign_petition(id, 100, false),
        Err(PetitionError::NotOwner)
    );
}

#[test]
fn sign_already_signed_fails() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);
    mgr.sign_petition(id, 200, false).unwrap();
    assert_eq!(
        mgr.sign_petition(id, 200, false),
        Err(PetitionError::AlreadySigned)
    );
}

#[test]
fn sign_nonexistent_petition_fails() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        mgr.sign_petition(999, 200, false),
        Err(PetitionError::NotFound)
    );
}

#[test]
fn unsign_petition() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);
    mgr.sign_petition(id, 200, false).unwrap();

    mgr.unsign_petition(id, 200).unwrap();
    assert!(!mgr.get(id).unwrap().has_signed(200));
}

#[test]
fn unsign_not_signed_fails() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);
    assert_eq!(mgr.unsign_petition(id, 200), Err(PetitionError::NotFound));
}

#[test]
fn not_enough_signatures_initially() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);
    assert!(!mgr.get(id).unwrap().has_enough_signatures());
}

// --- Guild registration ---

fn fully_signed_petition(mgr: &mut PetitionManager) -> u64 {
    let id = petition_with_owner(mgr);
    for i in 0..REQUIRED_SIGNATURES {
        mgr.sign_petition(id, 200 + i as u64, false).unwrap();
    }
    id
}

#[test]
fn submit_petition_creates_guild() {
    let mut mgr = PetitionManager::new();
    let id = fully_signed_petition(&mut mgr);

    let guild = mgr.submit_petition(id, 100, 42).unwrap();
    assert_eq!(guild.guild_id, 42);
    assert_eq!(guild.guild_name, "Test Guild");
    assert_eq!(guild.guild_master, 100);
    assert_eq!(guild.founding_members.len(), REQUIRED_SIGNATURES);
    assert!(guild.founding_members.contains(&200));

    // Petition consumed
    assert!(mgr.is_empty());
}

#[test]
fn submit_reserves_guild_name() {
    let mut mgr = PetitionManager::new();
    let id = fully_signed_petition(&mut mgr);
    mgr.submit_petition(id, 100, 42).unwrap();

    // Name now taken
    assert_eq!(
        mgr.purchase_charter(300, "test guild", 5000, false, 2000),
        Err(PetitionError::NameTaken)
    );
}

#[test]
fn submit_not_enough_signatures() {
    let mut mgr = PetitionManager::new();
    let id = petition_with_owner(&mut mgr);
    mgr.sign_petition(id, 200, false).unwrap(); // only 1

    assert_eq!(
        mgr.submit_petition(id, 100, 42),
        Err(PetitionError::NotEnoughSignatures)
    );
}

#[test]
fn submit_not_owner_fails() {
    let mut mgr = PetitionManager::new();
    let id = fully_signed_petition(&mut mgr);
    assert_eq!(
        mgr.submit_petition(id, 999, 42),
        Err(PetitionError::NotOwner)
    );
}

#[test]
fn submit_not_found() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        mgr.submit_petition(999, 100, 42),
        Err(PetitionError::NotFound)
    );
}

#[test]
fn owner_can_create_new_petition_after_submit() {
    let mut mgr = PetitionManager::new();
    let id = fully_signed_petition(&mut mgr);
    mgr.submit_petition(id, 100, 42).unwrap();

    // Owner freed — but now guilded, so can't create another
    // (in practice the server checks is_guilded; here we test the slot is free)
    assert!(mgr.find_by_owner(100).is_none());
}

// --- Guild tabard ---

fn sample_design() -> GuildTabardDesign {
    GuildTabardDesign {
        icon: 5,
        icon_color: 2,
        border: 3,
        border_color: 1,
        background_color: 7,
    }
}

#[test]
fn register_tabard_success() {
    let design = register_tabard(sample_design(), 100, 100, 200_000).unwrap();
    assert_eq!(design.icon, 5);
    assert_eq!(design.icon_color, 2);
    assert_eq!(design.border, 3);
    assert_eq!(design.border_color, 1);
    assert_eq!(design.background_color, 7);
}

#[test]
fn register_tabard_exact_cost() {
    assert!(register_tabard(sample_design(), 100, 100, TABARD_COST).is_ok());
}

#[test]
fn register_tabard_insufficient_funds() {
    assert_eq!(
        register_tabard(sample_design(), 100, 100, TABARD_COST - 1),
        Err(TabardError::InsufficientFunds)
    );
}

#[test]
fn register_tabard_not_guild_master() {
    assert_eq!(
        register_tabard(sample_design(), 200, 100, 200_000),
        Err(TabardError::NotGuildMaster)
    );
}

// --- Guild charter NPC ---

const CLOSE_DIST_SQ: f32 = 25.0; // 5 yards squared
const FAR_DIST_SQ: f32 = 400.0; // 20 yards squared

fn npc_ctx<'a>(flags: u32, dist_sq: f32, gold: u32, name: &'a str) -> CharterNpcContext<'a> {
    CharterNpcContext {
        npc_flags: flags,
        distance_squared: dist_sq,
        player: 100,
        guild_name: name,
        player_gold: gold,
        is_guilded: false,
        now: 1000,
    }
}

#[test]
fn buy_charter_from_npc_success() {
    let mut mgr = PetitionManager::new();
    let id = buy_charter_from_npc(
        &mut mgr,
        &npc_ctx(NPC_FLAG_GUILD_CHARTER, CLOSE_DIST_SQ, 5000, "New Guild"),
    )
    .unwrap();
    assert_eq!(mgr.get(id).unwrap().guild_name, "New Guild");
}

#[test]
fn buy_charter_not_charter_vendor() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        buy_charter_from_npc(&mut mgr, &npc_ctx(0, CLOSE_DIST_SQ, 5000, "Guild")),
        Err(CharterNpcError::NotCharterVendor)
    );
}

#[test]
fn buy_charter_out_of_range() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        buy_charter_from_npc(
            &mut mgr,
            &npc_ctx(NPC_FLAG_GUILD_CHARTER, FAR_DIST_SQ, 5000, "Guild")
        ),
        Err(CharterNpcError::OutOfRange)
    );
}

#[test]
fn buy_charter_npc_passes_purchase_errors() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        buy_charter_from_npc(
            &mut mgr,
            &npc_ctx(NPC_FLAG_GUILD_CHARTER, CLOSE_DIST_SQ, 0, "Guild")
        ),
        Err(CharterNpcError::Purchase(PetitionError::InsufficientFunds))
    );
}

#[test]
fn buy_charter_npc_at_exact_range() {
    let mut mgr = PetitionManager::new();
    let exact = CHARTER_NPC_RANGE * CHARTER_NPC_RANGE;
    assert!(
        buy_charter_from_npc(
            &mut mgr,
            &npc_ctx(NPC_FLAG_GUILD_CHARTER, exact, 5000, "Guild")
        )
        .is_ok()
    );
}

use super::*;

pub(super) fn print_response(response: &AdminResponse) {
    if print_listing_response(response)
        || print_account_response(response)
        || print_character_response(response)
        || print_equipment_response(response)
    {
        return;
    }

    if let AdminResponse::Error(error) = response {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn print_listing_response(response: &AdminResponse) -> bool {
    match response {
        AdminResponse::Pong => println!("pong"),
        AdminResponse::AccountsListed { accounts } => print_accounts(accounts),
        AdminResponse::CharactersListed { characters } => print_characters(characters),
        AdminResponse::CharacterFound { character } => print_character_search(character.as_ref()),
        AdminResponse::CombatLogEntries { count, entries } => print_combat_log(*count, entries),
        _ => return false,
    }
    true
}

fn print_account_response(response: &AdminResponse) -> bool {
    if print_account_check_response(response) || print_simple_account_response(response) {
        return true;
    }

    match response {
        AdminResponse::AccountKicked {
            username,
            disconnected,
        } => print_account_kicked(username, *disconnected),
        AdminResponse::AccountBanned {
            username,
            disconnected,
            reason,
        } => print_account_banned(username, *disconnected, reason),
        AdminResponse::AccountUnbanned { username } => print_account_unbanned(username),
        AdminResponse::CredentialsSet {
            account_id,
            username,
        } => print_credentials_set(*account_id, username),
        _ => return false,
    }
    true
}

fn print_account_check_response(response: &AdminResponse) -> bool {
    let AdminResponse::AccountChecked {
        username,
        exists,
        account_id,
        token_count,
        pending_registration,
        banned,
        ban_reason,
    } = response
    else {
        return false;
    };

    print_account_check(
        username,
        *exists,
        *account_id,
        *token_count,
        *pending_registration,
        *banned,
        ban_reason.as_deref(),
    );
    true
}

fn print_simple_account_response(response: &AdminResponse) -> bool {
    match response {
        AdminResponse::PasswordChecked { username, valid } => {
            print_password_check(username, *valid);
        }
        AdminResponse::AccountCreated { account_id, token } => {
            print_account_created(*account_id, token);
        }
        AdminResponse::RegistrationApproved { account_id, token } => {
            print_registration_approved(*account_id, token);
        }
        AdminResponse::CharacterCreated {
            username,
            character_id,
            name,
        } => println!("Character created: id={character_id} name={name} account={username}"),
        _ => return false,
    }
    true
}

fn print_character_response(response: &AdminResponse) -> bool {
    match response {
        AdminResponse::CharactersMoved {
            from_username,
            to_username,
            moved_count,
        } => {
            println!("Moved {moved_count} characters from {from_username} to {to_username}");
        }
        AdminResponse::GuildCreated {
            character_id,
            guild_id,
            guild_name,
        } => print_guild_created(*character_id, *guild_id, guild_name),
        AdminResponse::GuildRankSet {
            character_name,
            rank,
            rank_name,
        } => {
            println!("Guild rank set: {character_name} -> {rank} ({rank_name})");
        }
        AdminResponse::GuildBankRightsSet {
            guild_id,
            rank,
            rank_name,
        } => {
            println!("Guild bank rights set: guild {guild_id} rank {rank} ({rank_name})");
        }
        AdminResponse::LevelSet {
            character_name,
            level,
        } => {
            println!("Level set: {character_name} -> {level}");
        }
        AdminResponse::RestedSet {
            character_name,
            amount,
        } => {
            println!("Rested XP set: {character_name} -> {amount}");
        }
        AdminResponse::TaxiNodeLearned {
            character_name,
            node,
            node_name,
        } => {
            println!("Flight point learned: {character_name} -> {node} {node_name}");
        }
        AdminResponse::SpellLearned {
            character_name,
            spell,
        } => {
            println!("Spell learned: {character_name} -> {spell}");
        }
        AdminResponse::FlightStyleSet {
            character_name,
            skyriding,
        } => {
            let style = if *skyriding { "Skyriding" } else { "Steady" };
            println!("Flight style: {character_name} -> {style}");
        }
        AdminResponse::PositionSet {
            character_name,
            x,
            y,
            z,
        } => {
            println!("Position set: {character_name} -> ({x}, {y}, {z})");
        }
        AdminResponse::Damaged {
            character_name,
            amount,
        } => {
            println!("Damaged: {character_name}'s target took {amount}");
        }
        AdminResponse::Revived { character_name } => println!("Revived: {character_name}"),
        AdminResponse::ClockAdvanced { offset_secs, unix } => {
            println!("Instance lock clock: offset {offset_secs} s, now {unix}");
        }
        AdminResponse::InstanceLockList {
            character_name,
            locks,
        } => {
            println!("Instance locks of {character_name}: {}", locks.len());
            for lock in locks {
                println!("  {lock}");
            }
        }
        AdminResponse::InstanceUnbound {
            character_name,
            count,
        } => println!("Unbound {count} instance locks of {character_name}"),
        AdminResponse::EconomyStatus { status } => {
            println!(
                "{}",
                serde_json::to_string_pretty(status).expect("serialize economy status")
            );
        }
        AdminResponse::NpcInfoList { lines } | AdminResponse::GameObjectInfoList { lines } => {
            for line in lines {
                println!("{line}");
            }
        }
        AdminResponse::Killed { character_name } => {
            println!("Killed: {character_name}");
        }
        AdminResponse::FollowerPartyStaged {
            character_name,
            followers,
        } => {
            println!(
                "HARNESS staged: {character_name} and {followers} existing followers; not gameplay traversal"
            );
        }
        AdminResponse::Teleported {
            character_name,
            map_id,
            online,
        } => {
            let how = if *online { "teleporting" } else { "saved" };
            println!("Teleport: {character_name} -> map {map_id} ({how})");
        }
        AdminResponse::TradingPostTenderGranted { username, tender } => {
            println!("Trading Post Tender: {username} -> {tender}");
        }
        AdminResponse::TradingPostRefundWindowSet { seconds } => {
            println!("Trading Post refund window: {seconds}s");
        }
        AdminResponse::RenownGranted {
            character_name,
            major_faction,
            level,
            standing,
        } => {
            println!(
                "Renown {major_faction}: {character_name} -> level {level}, standing {standing}"
            );
        }
        AdminResponse::GoldSet {
            character_name,
            copper,
        } => {
            println!("Gold set: {character_name} -> {copper} copper");
        }
        AdminResponse::ItemGranted {
            character_name,
            item_id,
            name,
            count,
        } => {
            println!("Item granted: {character_name} <- {count} x {name} ({item_id})");
        }
        AdminResponse::MailDelivered {
            character_name,
            count,
        } => {
            println!("Mail delivered: {character_name} <- {count} mail");
        }
        AdminResponse::QuestCompleted {
            character_name,
            quest_id,
        } => {
            println!("Quest objectives complete: {character_name} quest {quest_id}");
        }
        AdminResponse::DurabilitySet {
            character_name,
            percent,
        } => {
            println!("Durability set: {character_name} -> {percent}%");
        }
        AdminResponse::AppearanceSet {
            character_name,
            appearance,
        } => print_appearance_set(character_name, appearance),
        _ => return false,
    }
    true
}

fn print_equipment_response(response: &AdminResponse) -> bool {
    match response {
        AdminResponse::EquipmentListed {
            character_id,
            character_name,
            items,
        }
        | AdminResponse::EquipmentGranted {
            character_id,
            character_name,
            items,
        } => {
            println!(
                "character_id={character_id} name={character_name} equipped={}",
                items.len()
            );
            for item in items {
                println!(
                    "slot={:?} guid={} item_id={} name={:?} count={} required_level={} quality={}",
                    item.slot,
                    item.item_guid,
                    item.item_id,
                    item.name,
                    item.stack_count,
                    item.required_level,
                    item.quality
                );
            }
        }
        AdminResponse::EquipmentAppearanceSet {
            character_name,
            slot,
            display_info_id,
        } => {
            println!("Equipment appearance set: {character_name} {slot} -> {display_info_id}");
        }
        AdminResponse::EquipmentAppearanceCleared {
            character_name,
            slot,
        } => {
            println!("Equipment appearance cleared: {character_name} {slot}");
        }
        _ => return false,
    }
    true
}

fn print_accounts(accounts: &[AccountInfo]) {
    if accounts.is_empty() {
        println!("No accounts found.");
        return;
    }
    for account in accounts {
        println!(
            "username={} account_id={} token_count={}",
            account.username, account.account_id, account.token_count
        );
    }
}

fn print_characters(characters: &[CharacterInfo]) {
    if characters.is_empty() {
        println!("No characters found.");
        return;
    }
    for character in characters {
        print_character_record(character);
    }
}

fn print_character_search(character: Option<&CharacterInfo>) {
    match character {
        Some(character) => print_character_record(character),
        None => println!("Character not found."),
    }
}

fn print_character_record(character: &CharacterInfo) {
    println!(
        "username={} account_id={} character_id={} name={} level={} race={} class={}",
        character.username,
        character.account_id,
        character.character_id,
        character.name,
        character.level,
        character.race,
        character.class
    );
}

fn print_account_check(
    username: &str,
    exists: bool,
    account_id: Option<u64>,
    token_count: usize,
    pending_registration: bool,
    banned: bool,
    ban_reason: Option<&str>,
) {
    println!("username={username}");
    println!("exists={exists}");
    println!(
        "account_id={}",
        account_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "<none>".to_string())
    );
    println!("token_count={token_count}");
    println!("pending_registration={pending_registration}");
    println!("banned={banned}");
    println!("ban_reason={}", ban_reason.unwrap_or("<none>"));
}

fn print_password_check(username: &str, valid: bool) {
    println!("username={username}");
    println!("valid={valid}");
}

fn print_account_created(account_id: u64, token: &str) {
    println!("Account created: id={account_id} token={token}");
}

fn print_account_kicked(username: &str, disconnected: bool) {
    println!("Account kicked: username={username} disconnected={disconnected}");
}

fn print_account_banned(username: &str, disconnected: bool, reason: &str) {
    println!("Account banned: username={username} disconnected={disconnected} reason={reason}");
}

fn print_account_unbanned(username: &str) {
    println!("Account unbanned: username={username}");
}

fn print_registration_approved(account_id: u64, token: &str) {
    println!("Registration approved: id={account_id} token={token}");
}

fn print_credentials_set(account_id: u64, username: &str) {
    println!("Credentials set: account_id={account_id} username={username}");
}

fn print_guild_created(character_id: u64, guild_id: u32, guild_name: &str) {
    println!(
        "Guild assigned: character_id={character_id} guild_id={guild_id} guild_name={guild_name}"
    );
}

fn print_appearance_set(character_name: &str, appearance: &CharacterAppearance) {
    println!(
        "Appearance set: {character_name} -> sex={} skin_color={} face={} eye_color={} hair_style={} hair_color={} facial_style={}",
        appearance.sex,
        appearance.skin_color,
        appearance.face,
        appearance.eye_color,
        appearance.hair_style,
        appearance.hair_color,
        appearance.facial_style
    );
}

fn print_combat_log(count: usize, entries: &[CombatLogSummary]) {
    println!("Combat log: {count} entries");
    for entry in entries {
        println!(
            "t={:.2} {} attacker={} target={} amount={} spell_id={}",
            entry.timestamp,
            entry.event_type,
            entry.attacker,
            entry.target,
            entry.amount,
            entry.spell_id
        );
    }
}

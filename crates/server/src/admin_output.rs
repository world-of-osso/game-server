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
        _ => return false,
    }
    true
}

fn print_account_response(response: &AdminResponse) -> bool {
    match response {
        AdminResponse::AccountChecked {
            username,
            exists,
            account_id,
            token_count,
            pending_registration,
            banned,
            ban_reason,
        } => print_account_check(
            username,
            *exists,
            *account_id,
            *token_count,
            *pending_registration,
            *banned,
            ban_reason.as_deref(),
        ),
        AdminResponse::PasswordChecked { username, valid } => {
            println!("username={username}");
            println!("valid={valid}");
        }
        AdminResponse::AccountCreated { account_id, token } => {
            println!("Account created: id={account_id} token={token}");
        }
        AdminResponse::AccountKicked {
            username,
            disconnected,
        } => {
            println!("Account kicked: username={username} disconnected={disconnected}");
        }
        AdminResponse::AccountBanned {
            username,
            disconnected,
            reason,
        } => {
            println!(
                "Account banned: username={username} disconnected={disconnected} reason={reason}"
            );
        }
        AdminResponse::AccountUnbanned { username } => {
            println!("Account unbanned: username={username}");
        }
        AdminResponse::RegistrationApproved { account_id, token } => {
            println!("Registration approved: id={account_id} token={token}");
        }
        AdminResponse::CredentialsSet {
            account_id,
            username,
        } => {
            println!("Credentials set: account_id={account_id} username={username}");
        }
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
        AdminResponse::LevelSet {
            character_name,
            level,
        } => {
            println!("Level set: {character_name} -> {level}");
        }
        AdminResponse::PositionSet {
            character_name,
            x,
            y,
            z,
        } => {
            println!("Position set: {character_name} -> ({x}, {y}, {z})");
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

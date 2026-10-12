use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) struct DescriptionSources {
    effects: BTreeMap<(u32, u32), Vec<(f64, String)>>,
    spells: BTreeMap<u32, String>,
}

pub(super) fn load_description_sources(conn: &Connection) -> rusqlite::Result<DescriptionSources> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT p.TraitDefinitionID,x.Field_10_0_0_44649_003,e.Amount,t.Name_lang
         FROM profession_trait p
         JOIN profession_trait_x_effect x ON x.ProfessionTraitID=p.ID
         JOIN profession_effect e ON e.ID=x.ProfessionEffectID
         JOIN profession_effect_type t ON t.EnumID=e.ProfessionEffectTypeEnumID
         ORDER BY x.ID",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(((row.get(0)?, row.get(1)?), (row.get(2)?, row.get(3)?)))
    })?;
    let mut effects: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for row in rows {
        let (key, value) = row?;
        effects.entry(key).or_default().push(value);
    }
    let spells = conn
        .prepare("SELECT ID,Name_lang FROM spell_name")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(DescriptionSources { effects, spells })
}

impl DescriptionSources {
    pub(super) fn resolve(&self, definition: u32, text: &str) -> Result<String, String> {
        let mut remaining = text;
        let mut resolved = String::new();
        while let Some(start) = remaining.find('$') {
            resolved.push_str(&remaining[..start]);
            let (replacement, consumed) =
                self.resolve_token(definition, &remaining[start + 1..])?;
            resolved.push_str(&replacement);
            remaining = &remaining[start + 1 + consumed..];
        }
        resolved.push_str(remaining);
        Ok(resolved)
    }

    fn resolve_token(&self, definition: u32, token: &str) -> Result<(String, usize), String> {
        let lower = token.to_ascii_lowercase();
        let (prefix, value_kind) = if lower.starts_with("ev") {
            (2, true)
        } else if lower.starts_with("en") {
            (2, false)
        } else if lower.starts_with("@spellname") {
            return self.resolve_spell_name(token);
        } else {
            return Err("unsupported source description variable".into());
        };
        let (index, digits) = read_index(&token[prefix..])?;
        let slot = index.checked_sub(1).ok_or("effect indices start at one")?;
        let effects = self
            .effects
            .get(&(definition, slot))
            .ok_or_else(|| format!("missing profession effect binding {index}"))?;
        let [(amount, name)] = effects.as_slice() else {
            return Err(format!("ambiguous profession effect binding {index}"));
        };
        let replacement = if value_kind {
            amount.to_string()
        } else {
            name.clone()
        };
        Ok((replacement, prefix + digits))
    }

    fn resolve_spell_name(&self, token: &str) -> Result<(String, usize), String> {
        const PREFIX: usize = "@spellname".len();
        let (spell, digits) = read_index(&token[PREFIX..])?;
        let name = self
            .spells
            .get(&spell)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| format!("missing source spell name {spell}"))?;
        Ok((name.clone(), PREFIX + digits))
    }
}

fn read_index(text: &str) -> Result<(u32, usize), String> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    let index = text[..digits]
        .parse()
        .map_err(|_| "missing numeric source variable index")?;
    Ok((index, digits))
}

/// `ObjectMgr::ParseSpawnDifficulties`: comma-separated `Difficulty.db2` IDs.
pub(crate) fn parse_spawn_difficulties(text: &str) -> Vec<u32> {
    text.split(',')
        .map(|token| {
            token
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("bad spawnDifficulties {text:?}"))
        })
        .collect()
}



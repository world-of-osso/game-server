//! Immutable fixture catalog boundary. The first tests exercise today's uncached reader.
use std::path::Path;
use std::sync::Arc;

pub(crate) fn load<T>(
    path: &Path,
    read: impl FnOnce() -> rusqlite::Result<T>,
) -> rusqlite::Result<Arc<T>> {
    std::fs::metadata(path)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    read().map(Arc::new)
}

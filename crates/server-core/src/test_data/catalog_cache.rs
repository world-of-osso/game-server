//! Complete immutable fixture catalogs, keyed by canonical file path and BLAKE3 content.
//! Apps own their resource copies; this cache never retains a database connection or World.
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fs::{File, Metadata};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileVersion {
    device: u64,
    inode: u64,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl From<Metadata> for FileVersion {
    fn from(metadata: Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}

#[derive(Clone, Copy)]
struct Fingerprint {
    version: FileVersion,
    digest: [u8; 32],
}

struct Input {
    path: PathBuf,
    digest: [u8; 32],
}

type CatalogSlot<T> = Mutex<Option<Arc<T>>>;
type ErasedSlot = Arc<dyn Any + Send + Sync>;

struct InputCatalogs {
    digest: [u8; 32],
    types: HashMap<TypeId, ErasedSlot>,
}

impl InputCatalogs {
    fn cache_slot<T: Send + Sync + 'static>(&mut self, digest: [u8; 32]) -> ErasedSlot {
        if self.digest != digest {
            self.digest = digest;
            self.types.clear();
        }
        self.types
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Arc::new(CatalogSlot::<T>::new(None)))
            .clone()
    }
}

/// Cache file-backed read-only fixture connections. Writable and in-memory databases
/// retain ordinary SQL reads because their mutable state is not an immutable file input.
pub fn load_connection<T: Clone + Send + Sync + 'static>(
    connection: &rusqlite::Connection,
    read: impl FnOnce() -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    if connection.is_readonly(rusqlite::DatabaseName::Main)? {
        if let Some(path) = connection.path() {
            return load(Path::new(path), read).map(|data| data.as_ref().clone());
        }
    }
    read()
}

/// `read` must construct the complete definition catalog for `T`, never a mutable
/// App, connection, entity collection or a caller-dependent filtered view.
pub fn load<T: Send + Sync + 'static>(
    path: &Path,
    read: impl FnOnce() -> rusqlite::Result<T>,
) -> rusqlite::Result<Arc<T>> {
    let input = fingerprint_input(path).map_err(|error| {
        let context = io::Error::new(
            error.kind(),
            format!("fingerprint fixture input {}: {error}", path.display()),
        );
        rusqlite::Error::ToSqlConversionFailure(Box::new(context))
    })?;
    let slot = catalog_slot::<T>(input)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let mut cached = slot
        .lock()
        .expect("fixture catalog initialization poisoned");
    if let Some(value) = cached.as_ref() {
        return Ok(value.clone());
    }
    let value = Arc::new(read()?);
    *cached = Some(value.clone());
    Ok(value)
}

fn fingerprint_input(path: &Path) -> io::Result<Input> {
    static FINGERPRINTS: OnceLock<Mutex<HashMap<PathBuf, Fingerprint>>> = OnceLock::new();
    let path = path.canonicalize()?;
    let version = FileVersion::from(path.metadata()?);
    let fingerprints = FINGERPRINTS.get_or_init(Default::default);
    let previous = fingerprints
        .lock()
        .expect("fixture fingerprint cache poisoned")
        .get(&path)
        .copied();
    if let Some(previous) = previous.filter(|previous| previous.version == version) {
        return Ok(Input {
            path,
            digest: previous.digest,
        });
    }
    let digest = hash_snapshot(&path, version)?;
    fingerprints
        .lock()
        .expect("fixture fingerprint cache poisoned")
        .insert(path.clone(), Fingerprint { version, digest });
    Ok(Input { path, digest })
}

fn hash_snapshot(path: &Path, before: FileVersion) -> io::Result<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(File::open(path)?)?;
    if FileVersion::from(path.metadata()?) != before {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "fixture input changed while its content was being hashed",
        ));
    }
    Ok(*hasher.finalize().as_bytes())
}

fn remove_deleted_inputs(
    inputs: &mut HashMap<PathBuf, InputCatalogs>,
) -> io::Result<Vec<InputCatalogs>> {
    let mut deleted = Vec::new();
    for path in inputs.keys() {
        let exists = path.try_exists().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("inspect cached fixture source {}: {error}", path.display()),
            )
        })?;
        if !exists {
            deleted.push(path.clone());
        }
    }
    Ok(deleted
        .into_iter()
        .map(|path| {
            inputs
                .remove(&path)
                .expect("deleted fixture input collected under the registry lock")
        })
        .collect())
}

fn catalog_slot<T: Send + Sync + 'static>(input: Input) -> io::Result<Arc<CatalogSlot<T>>> {
    static CATALOGS: OnceLock<Mutex<HashMap<PathBuf, InputCatalogs>>> = OnceLock::new();
    let (slot, expired) = {
        let mut inputs = CATALOGS
            .get_or_init(Default::default)
            .lock()
            .expect("fixture catalog registry poisoned");
        let expired = remove_deleted_inputs(&mut inputs)?;
        let catalogs = inputs.entry(input.path).or_insert_with(|| InputCatalogs {
            digest: input.digest,
            types: HashMap::new(),
        });
        (catalogs.cache_slot::<T>(input.digest), expired)
    };
    drop(expired);
    match slot.downcast::<CatalogSlot<T>>() {
        Ok(slot) => Ok(slot),
        Err(_) => unreachable!("fixture catalog TypeId does not match its stored slot"),
    }
}

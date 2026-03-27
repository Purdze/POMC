use crate::installations::{Id, Installation, InstallationError};
use crate::storage::data_dir;

use fslock::LockFile;

fn lock() -> Result<LockFile, InstallationError> {
    let path = data_dir().join("installations.lock");
    let mut lock = LockFile::open(&path)?;
    lock.lock()?;
    Ok(lock)
}

fn load() -> Result<Vec<Installation>, InstallationError> {
    let path = data_dir().join("installations.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn save(list: &[Installation]) -> Result<(), InstallationError> {
    let json = serde_json::to_string_pretty(list)?;
    std::fs::write(data_dir().join("installations.json"), json)?;
    Ok(())
}

pub fn get_all() -> Result<Vec<Installation>, InstallationError> {
    let _lock = lock()?;
    load()
}

pub fn register(installation: Installation) -> Result<(), InstallationError> {
    let _lock = lock()?;
    let mut list = load()?;

    if list.iter().any(|i| i.directory == installation.directory) {
        return Err(InstallationError::DirectoryAlreadyExists);
    }

    list.push(installation);
    save(&list)
}

pub fn unregister(id: &Id) -> Result<(), InstallationError> {
    let _lock = lock()?;
    let mut list = load()?;

    list.retain(|i| i.id != *id);

    save(&list)
}

pub fn get(id: &Id) -> Result<Option<Installation>, InstallationError> {
    let _lock = lock()?;
    let list = load()?;

    Ok(list.into_iter().find(|i| i.id == *id))
}

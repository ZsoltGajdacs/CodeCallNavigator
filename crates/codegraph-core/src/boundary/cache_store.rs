//! SQLite keeps only current decision rows and their implementation capability.
use super::{cache_key::FORMAT, cache_paths};
const APPLICATION_ID: i64 = 0x4347494e;
use anyhow::{Result, ensure};
use rusqlite::{Connection, OpenFlags};
use std::{collections::BTreeMap, path::Path};

pub(super) fn read(path: &Path, implementation: &str) -> Result<BTreeMap<String, String>> {
    cache_paths::validate(path)?;
    let db = read_owned(path)?;
    let version: String = db.query_row(
        "SELECT value FROM cache_meta WHERE name='format'",
        [],
        |row| row.get(0),
    )?;
    let recorded: String = db.query_row(
        "SELECT value FROM cache_meta WHERE name='implementation'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        version == FORMAT.to_string() && recorded == implementation,
        "Inference cache format or implementation differs"
    );
    let mut statement = db.prepare("SELECT key,value FROM boundary_decisions")?;
    Ok(statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?)
}
pub(super) fn write(
    path: &Path,
    implementation: &str,
    rows: &BTreeMap<String, String>,
) -> Result<()> {
    cache_paths::validate(path)?;
    let existing = path.exists();
    if existing {
        read_owned(path)?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.busy_timeout(std::time::Duration::from_millis(100))?;
    if existing {
        owned(&db)?;
    }
    let transaction = db.transaction()?;
    transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
    transaction.execute_batch(
        "DROP TABLE IF EXISTS cache_meta; DROP TABLE IF EXISTS boundary_decisions;
        CREATE TABLE cache_meta(name TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE boundary_decisions(key TEXT PRIMARY KEY,value TEXT NOT NULL);",
    )?;
    for (name, value) in [
        ("format", FORMAT.to_string()),
        ("implementation", implementation.into()),
    ] {
        transaction.execute(
            "INSERT INTO cache_meta VALUES (?1,?2)",
            rusqlite::params![name, value],
        )?;
    }
    for (key, value) in rows {
        transaction.execute(
            "INSERT INTO boundary_decisions VALUES (?1,?2)",
            rusqlite::params![key, value],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn read_owned(path: &Path) -> Result<Connection> {
    let db = Connection::open_with_flags(
        cache_paths::readonly_uri(path)?,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    owned(&db)?;
    Ok(db)
}

fn owned(db: &Connection) -> Result<()> {
    let id: i64 = db.pragma_query_value(None, "application_id", |row| row.get(0))?;
    ensure!(
        id == APPLICATION_ID,
        "Existing SQLite file is not an owned inference cache"
    );
    Ok(())
}

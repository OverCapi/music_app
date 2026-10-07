//! One-time import of the JSON files written by older versions
//! (`library.json`, `playlists.json`, `account.json`). Each imported file is renamed
//! to `*.json.bak` so the import never runs twice.

use std::path::Path;

use rusqlite::Connection;

use crate::error::AppResult;

pub fn run(_conn: &mut Connection, _data_dir: &Path) -> AppResult<()> {
    // TODO(agent B): import in one transaction, then rename the files.
    Ok(())
}

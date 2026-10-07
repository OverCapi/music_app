use tauri::State;

use super::{Account, AccountStore};
use crate::error::AppResult;
use crate::group::GroupClient;

#[tauri::command]
#[specta::specta]
pub fn get_account(account: State<'_, AccountStore>) -> AppResult<Account> {
    account.account()
}

#[tauri::command]
#[specta::specta]
pub fn set_server_url(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
    url: String,
) -> AppResult<Account> {
    if account.set_server_url(&url)? {
        // Logged out of the old server: its group is gone too.
        group.leave();
    }
    account.account()
}

#[tauri::command]
#[specta::specta]
pub async fn register(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
    username: String,
    password: String,
) -> AppResult<Account> {
    let result = account.register(&username, &password).await?;
    // The group belonged to the previous account, if any.
    group.leave();
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn login(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
    username: String,
    password: String,
) -> AppResult<Account> {
    let result = account.login(&username, &password).await?;
    group.leave();
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn logout(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
) -> AppResult<Account> {
    group.leave();
    account.logout()
}

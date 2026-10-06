import { load, type Store } from '@tauri-apps/plugin-store'

let storePromise: Promise<Store> | null = null

/** Shared persistent key/value store (settings.json in the app data dir). */
export function settings(): Promise<Store> {
  storePromise ??= load('settings.json', { defaults: {}, autoSave: 200 })
  return storePromise
}

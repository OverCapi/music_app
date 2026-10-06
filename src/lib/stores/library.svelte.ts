import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { Track } from '../types'
import { AUDIO_EXTENSIONS } from '../utils'
import { settings } from './settings'

function compareTracks(a: Track, b: Track): number {
  return (
    a.artist.localeCompare(b.artist, undefined, { sensitivity: 'base' }) ||
    a.album.localeCompare(b.album, undefined, { sensitivity: 'base' }) ||
    (a.trackNo ?? 0) - (b.trackNo ?? 0) ||
    a.title.localeCompare(b.title, undefined, { sensitivity: 'base' })
  )
}

class Library {
  tracks = $state<Track[]>([])
  folders = $state<string[]>([])
  files = $state<string[]>([])
  scanning = $state(false)
  error = $state<string | null>(null)

  async init() {
    const store = await settings()
    this.folders = (await store.get<string[]>('folders')) ?? []
    this.files = (await store.get<string[]>('files')) ?? []
    await this.rescan()
  }

  async rescan() {
    this.scanning = true
    this.error = null
    try {
      const [fromFolders, fromFiles] = await Promise.all([
        invoke<Track[]>('scan_folders', { paths: this.folders }),
        invoke<Track[]>('read_files', { paths: this.files }),
      ])
      const byId = new Map<string, Track>()
      for (const t of [...fromFolders, ...fromFiles]) byId.set(t.id, t)
      this.tracks = [...byId.values()].sort(compareTracks)
    } catch (e) {
      this.error = String(e)
    } finally {
      this.scanning = false
    }
  }

  async addFolder() {
    const picked = await open({ directory: true, multiple: true, title: 'Ajouter un dossier' })
    if (picked) await this.addFolders(picked)
  }

  async importFiles() {
    const picked = await open({
      multiple: true,
      title: 'Importer des fichiers',
      filters: [{ name: 'Audio', extensions: AUDIO_EXTENSIONS }],
    })
    if (picked) await this.addFiles(picked)
  }

  /** Routes dropped paths: directories become scanned folders, files become manual imports. */
  async addPaths(paths: string[]) {
    const isDir = await invoke<boolean[]>('classify_paths', { paths })
    const dirs = paths.filter((_, i) => isDir[i])
    const files = paths.filter((_, i) => !isDir[i])
    this.folders = [...new Set([...this.folders, ...dirs])]
    this.files = [...new Set([...this.files, ...files])]
    await this.persistAndRescan()
  }

  async addFolders(paths: string[]) {
    this.folders = [...new Set([...this.folders, ...paths])]
    await this.persistAndRescan()
  }

  async addFiles(paths: string[]) {
    this.files = [...new Set([...this.files, ...paths])]
    await this.persistAndRescan()
  }

  async removeFolder(path: string) {
    this.folders = this.folders.filter((f) => f !== path)
    await this.persistAndRescan()
  }

  private async persistAndRescan() {
    const store = await settings()
    await store.set('folders', $state.snapshot(this.folders))
    await store.set('files', $state.snapshot(this.files))
    await this.rescan()
  }
}

export const library = new Library()

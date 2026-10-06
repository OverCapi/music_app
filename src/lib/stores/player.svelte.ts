import type { RepeatMode, Track } from '../types'
import { fileSrc, shuffled } from '../utils'
import { settings } from './settings'

const RESTART_THRESHOLD = 3

/**
 * Single audio engine for the app.
 *
 * `queue` holds upcoming tracks. Its first `userCount` items were added explicitly
 * by the user ("Lire ensuite" / "Ajouter à la file") and are kept when a new context
 * starts; the rest is what remains of the current context (album, library, search...).
 */
class Player {
  current = $state<Track | null>(null)
  isPlaying = $state(false)
  position = $state(0)
  duration = $state(0)
  volume = $state(0.8)
  muted = $state(false)
  shuffle = $state(false)
  repeat = $state<RepeatMode>('off')
  queue = $state<Track[]>([])
  userCount = $state(0)
  unplayable = $state(new Set<string>())

  private audio = new Audio()
  private history: Track[] = []
  private context: Track[] = []

  constructor() {
    const a = this.audio
    a.preload = 'auto'
    a.volume = this.volume
    a.addEventListener('timeupdate', () => (this.position = a.currentTime))
    a.addEventListener('durationchange', () => (this.duration = a.duration || 0))
    a.addEventListener('play', () => (this.isPlaying = true))
    a.addEventListener('pause', () => (this.isPlaying = false))
    a.addEventListener('ended', () => this.onEnded())
    a.addEventListener('error', () => this.onError())
    this.setupMediaSession()
  }

  async init() {
    const store = await settings()
    this.setVolume((await store.get<number>('volume')) ?? this.volume)
    this.repeat = (await store.get<RepeatMode>('repeat')) ?? 'off'
    this.shuffle = (await store.get<boolean>('shuffle')) ?? false
  }

  /** Plays `track` and makes the rest of `context` (after it) the upcoming queue. */
  play(track: Track, context: Track[] = [track]) {
    this.context = context
    const start = context.findIndex((t) => t.id === track.id)
    const rest = context.slice(start + 1)
    this.queue = [...this.queue.slice(0, this.userCount), ...(this.shuffle ? shuffled(rest) : rest)]
    if (this.current) this.history.push(this.current)
    this.load(track)
  }

  toggle() {
    if (!this.current) return
    if (this.audio.paused) this.audio.play().catch(() => {})
    else this.audio.pause()
  }

  next() {
    if (this.queue.length === 0) {
      if (this.repeat === 'all' && this.context.length > 0) {
        this.queue = this.shuffle ? shuffled(this.context) : [...this.context]
      } else {
        this.audio.pause()
        this.seek(0)
        return
      }
    }
    const [nextTrack, ...rest] = this.queue
    this.queue = rest
    if (this.userCount > 0) this.userCount--
    if (this.current) this.history.push(this.current)
    this.load(nextTrack)
  }

  prev() {
    const previous = this.history.at(-1)
    if (this.position > RESTART_THRESHOLD || !previous) {
      this.seek(0)
      return
    }
    this.history.pop()
    if (this.current) {
      const q = [...this.queue]
      q.splice(this.userCount, 0, this.current)
      this.queue = q
    }
    this.load(previous)
  }

  seek(seconds: number) {
    if (!this.current) return
    this.audio.currentTime = Math.max(0, Math.min(seconds, this.duration || 0))
    this.position = this.audio.currentTime
  }

  seekBy(delta: number) {
    this.seek(this.position + delta)
  }

  setVolume(v: number) {
    this.volume = Math.max(0, Math.min(1, v))
    this.audio.volume = this.volume
    if (this.volume > 0 && this.muted) this.toggleMute()
    settings().then((s) => s.set('volume', this.volume))
  }

  toggleMute() {
    this.muted = !this.muted
    this.audio.muted = this.muted
  }

  toggleShuffle() {
    this.shuffle = !this.shuffle
    const userPart = this.queue.slice(0, this.userCount)
    let contextPart = this.queue.slice(this.userCount)
    if (this.shuffle) {
      contextPart = shuffled(contextPart)
    } else if (this.current) {
      // Restore the original context order after the current track.
      const remaining = new Set(contextPart.map((t) => t.id))
      const idx = this.context.findIndex((t) => t.id === this.current!.id)
      contextPart = this.context.slice(idx + 1).filter((t) => remaining.has(t.id))
    }
    this.queue = [...userPart, ...contextPart]
    settings().then((s) => s.set('shuffle', this.shuffle))
  }

  cycleRepeat() {
    const order: RepeatMode[] = ['off', 'all', 'one']
    this.repeat = order[(order.indexOf(this.repeat) + 1) % order.length]
    settings().then((s) => s.set('repeat', this.repeat))
  }

  playNext(track: Track) {
    this.queue = [track, ...this.queue]
    this.userCount++
    if (!this.current) this.next()
  }

  addToQueue(track: Track) {
    const q = [...this.queue]
    q.splice(this.userCount, 0, track)
    this.queue = q
    this.userCount++
    if (!this.current) this.next()
  }

  removeFromQueue(index: number) {
    this.queue = this.queue.filter((_, i) => i !== index)
    if (index < this.userCount) this.userCount--
  }

  /** Plays the queued item at `index`, dropping everything before it. */
  jumpTo(index: number) {
    const target = this.queue[index]
    if (!target) return
    this.queue = this.queue.slice(index + 1)
    this.userCount = Math.max(0, this.userCount - index - 1)
    if (this.current) this.history.push(this.current)
    this.load(target)
  }

  reorder(from: number, to: number) {
    if (from === to) return
    const q = [...this.queue]
    const [item] = q.splice(from, 1)
    q.splice(to, 0, item)
    this.queue = q
    // Moving across the boundary changes which section the track belongs to.
    if (from < this.userCount && to >= this.userCount) this.userCount--
    else if (from >= this.userCount && to < this.userCount) this.userCount++
  }

  private load(track: Track) {
    this.current = track
    this.position = 0
    this.duration = track.duration
    this.audio.src = fileSrc(track.path)!
    this.audio.play().catch(() => {})
    this.updateMediaSession()
  }

  private onEnded() {
    if (this.repeat === 'one') {
      this.seek(0)
      this.audio.play().catch(() => {})
    } else {
      this.next()
    }
  }

  private onError() {
    if (!this.current) return
    this.unplayable = new Set(this.unplayable).add(this.current.id)
    // Avoid looping forever if every remaining track fails.
    if (this.queue.some((t) => !this.unplayable.has(t.id))) this.next()
    else this.audio.pause()
  }

  private setupMediaSession() {
    const ms = navigator.mediaSession
    if (!ms) return
    ms.setActionHandler('play', () => this.toggle())
    ms.setActionHandler('pause', () => this.toggle())
    ms.setActionHandler('nexttrack', () => this.next())
    ms.setActionHandler('previoustrack', () => this.prev())
    ms.setActionHandler('seekto', (d) => d.seekTime != null && this.seek(d.seekTime))
  }

  private updateMediaSession() {
    const ms = navigator.mediaSession
    if (!ms || !this.current) return
    const cover = fileSrc(this.current.cover)
    ms.metadata = new MediaMetadata({
      title: this.current.title,
      artist: this.current.artist,
      album: this.current.album,
      artwork: cover ? [{ src: cover }] : [],
    })
  }
}

export const player = new Player()

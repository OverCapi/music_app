export interface Track {
  id: string
  path: string
  title: string
  artist: string
  album: string
  trackNo: number | null
  duration: number
  cover: string | null
}

export type RepeatMode = 'off' | 'all' | 'one'

export type View = 'library' | 'queue'

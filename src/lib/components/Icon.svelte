<script lang="ts" module>
  const PATHS = {
    play: 'M8 5.14v13.72a1 1 0 0 0 1.52.85l10.78-6.86a1 1 0 0 0 0-1.7L9.52 4.29A1 1 0 0 0 8 5.14Z',
    pause: 'M7 4h3.5v16H7zM13.5 4H17v16h-3.5z',
    next: 'M5 5.5v13l9-6.5-9-6.5ZM16 5h2.5v14H16z',
    prev: 'M19 5.5v13l-9-6.5 9-6.5ZM5.5 5H8v14H5.5z',
    shuffle:
      'M16 3h5v5M4 20 21 3M21 16v5h-5M15 15l6 6M4 4l5 5',
    repeat: 'M17 2l4 4-4 4M3 11V9a3 3 0 0 1 3-3h15M7 22l-4-4 4-4M21 13v2a3 3 0 0 1-3 3H3',
    volume: 'M11 5 6 9H2v6h4l5 4V5ZM15.5 8.5a5 5 0 0 1 0 7M19 5a10 10 0 0 1 0 14',
    mute: 'M11 5 6 9H2v6h4l5 4V5ZM22 9l-6 6M16 9l6 6',
    library: 'M4 19V5M8 19V5M12 19 15 5l5 14',
    queue: 'M3 6h13M3 12h13M3 18h9M17 15v6l5-3-5-3Z',
    folder: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z',
    file: 'M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9l-6-6ZM14 3v6h6M10 17a2 2 0 1 1-2-2V11l4-1',
    search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14ZM21 21l-5-5',
    refresh: 'M21 12a9 9 0 1 1-3-6.7L21 8M21 3v5h-5',
    close: 'M6 6l12 12M18 6 6 18',
    grip: 'M9 6h.01M15 6h.01M9 12h.01M15 12h.01M9 18h.01M15 18h.01',
    note: 'M9 18V5l12-2v13M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM21 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
    trash: 'M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14',
  } as const

  export type IconName = keyof typeof PATHS
  const FILLED: IconName[] = ['play', 'pause', 'next', 'prev']
</script>

<script lang="ts">
  let { name, size = 18 }: { name: IconName; size?: number } = $props()
  const filled = $derived(FILLED.includes(name))
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={filled ? 'currentColor' : 'none'}
  stroke={filled ? 'none' : 'currentColor'}
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={PATHS[name]} />
</svg>

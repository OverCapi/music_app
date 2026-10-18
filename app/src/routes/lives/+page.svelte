<script lang="ts">
  import { onMount } from "svelte";
  import type { LiveChannel, LiveStream } from "#lib/bindings.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";

  const playingId = $derived(player.state.live?.id);

  onMount(() => lives.loadFavorites());

  function followers(n: number | null) {
    return n === null
      ? ""
      : `${new Intl.NumberFormat("fr-FR", { notation: "compact" }).format(n)} abonnés`;
  }
</script>

{#snippet channelCard(c: LiveChannel)}
  <button class="channel-card" onclick={() => lives.openChannel(c)}>
    <img src={c.thumbnail} alt="" loading="lazy" />
    <span class="name">{c.name}</span>
    <span class="sub">{c.handle ?? ""} {followers(c.followers)}</span>
  </button>
{/snippet}

{#snippet liveGrid(items: LiveStream[])}
  <div class="grid" class:stale={lives.loading}>
    {#each items as stream (stream.id)}
      <button
        class="live card"
        class:playing={stream.id === playingId}
        onclick={() => lives.play(stream)}
      >
        <span class="thumb">
          <img src={stream.thumbnail} alt="" loading="lazy" />
          <span class="badge">En direct</span>
        </span>
        <span class="title">{stream.title}</span>
        <span class="channel">
          {stream.channel ?? ""}{#if stream.viewers !== null}
            · {stream.viewers.toLocaleString("fr-FR")} spectateurs{/if}
        </span>
      </button>
    {/each}
  </div>
{/snippet}

<section class="view">
  <header>
    <h1>Lives</h1>
  </header>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      lives.submit();
    }}
  >
    <input
      class="field"
      type="search"
      bind:value={lives.query}
      placeholder="Un artiste, une chaîne, ou le lien d'un live YouTube"
      aria-label="Rechercher un live"
    />
    <button class="pill primary" disabled={lives.loading || !lives.query.trim()}>Rechercher</button>
  </form>

  {#if lives.channel}
    {@const c = lives.channel}
    <div class="channel-head">
      <button
        class="icon"
        onclick={() => (lives.channel = null)}
        aria-label="Retour"
        title="Retour"
      >
        <Icon name="chevronLeft" />
      </button>
      <img src={c.thumbnail} alt="" />
      <div class="who">
        <h2>{c.name}</h2>
        <span class="sub">{c.handle ?? ""} {followers(c.followers)}</span>
      </div>
      <button
        class="pill secondary"
        class:fav={lives.isFavorite(c.id)}
        onclick={() => lives.toggleFavorite(c)}
        aria-pressed={lives.isFavorite(c.id)}
      >
        <Icon name={lives.isFavorite(c.id) ? "heartFilled" : "heart"} size={18} />
        {lives.isFavorite(c.id) ? "Favori" : "Ajouter aux favoris"}
      </button>
    </div>
    {#if lives.channelLoading}
      <p class="status">Recherche des lives…</p>
    {:else if lives.channelLives.length === 0}
      <p class="status">Aucun live en cours sur cette chaîne.</p>
    {:else}
      {@render liveGrid(lives.channelLives)}
    {/if}
  {:else}
    {#if lives.loading}
      <p class="status">Recherche en cours…</p>
    {:else if lives.searched === null}
      <p class="status">Cherche « lofi girl », ou colle l'adresse d'un live ou d'une chaîne.</p>
    {/if}

    {#if lives.channels.length > 0 && !lives.loading}
      <h3 class="section-title">Chaînes</h3>
      <div class="channels">
        {#each lives.channels as c (c.id)}
          {@render channelCard(c)}
        {/each}
      </div>
    {/if}

    {#if lives.searched !== null && !lives.loading}
      {#if lives.results.length === 0}
        <p class="status">Aucun live en cours pour « {lives.searched} ».</p>
      {:else}
        <h3 class="section-title">Lives</h3>
        {@render liveGrid(lives.results)}
      {/if}
    {/if}

    {#if lives.favorites.length > 0}
      <h3 class="section-title">Chaînes favorites</h3>
      <div class="channels">
        {#each lives.favorites as c (c.id)}
          {@render channelCard(c)}
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  header {
    margin-bottom: 1.25rem;
  }

  form {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 1.25rem;
  }

  input {
    flex: 1;
    min-width: 0;
  }

  .section-title {
    margin-top: 0;
  }

  .channels {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 16px;
    margin-bottom: 1.5rem;
  }

  .channel-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
    padding: 0.8rem;
    border-radius: 16px;
    background: var(--surface);
    color: var(--text);
    text-align: center;
    transition:
      background 0.15s,
      transform 0.15s;
  }

  .channel-card:hover,
  .channel-card:focus-visible {
    background: var(--surface-2);
    transform: translateY(-2px);
  }

  .channel-card img {
    width: 88px;
    height: 88px;
    margin-bottom: 0.5rem;
    border-radius: 50%;
    object-fit: cover;
    background: var(--surface-3);
  }

  .channel-card .name {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 700;
  }

  .sub {
    color: var(--muted);
    font-size: 0.8rem;
  }

  .channel-head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 1.25rem;
  }

  .channel-head img {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    object-fit: cover;
    background: var(--surface-3);
  }

  .who {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .who h2 {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 1.3rem;
  }

  .pill.fav {
    color: var(--accent);
  }

  .status {
    margin: 0 0 1rem;
    color: var(--muted);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
  }

  .grid.stale {
    opacity: 0.5;
  }

  .live {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.8rem;
    text-align: left;
    color: var(--text);
    transition:
      background 0.15s,
      transform 0.15s;
  }

  .live:hover,
  .live:focus-visible,
  .live.playing {
    background: var(--surface-2);
  }

  .live:hover {
    transform: translateY(-2px);
  }

  .thumb {
    position: relative;
    display: block;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: 6px;
    background: var(--surface-3);
    box-shadow: var(--shadow-float);
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .badge {
    position: absolute;
    top: 8px;
    left: 8px;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    background: #e5252a;
    color: #fff;
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .title {
    display: -webkit-box;
    overflow: hidden;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    font-weight: 600;
  }

  .channel {
    color: var(--muted);
    font-size: 0.8rem;
  }
</style>

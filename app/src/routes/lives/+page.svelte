<script lang="ts">
  import { onMount } from "svelte";
  import type { LiveChannel, LiveStream } from "#lib/bindings.ts";
  import { progressive } from "#lib/progressive.svelte.ts";
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import Icon from "#lib/components/ui/Icon.svelte";
  import IconButton from "#lib/components/ui/IconButton.svelte";
  import Section from "#lib/components/ui/Section.svelte";
  import ChannelCircle from "#lib/components/live/ChannelCircle.svelte";
  import LiveCard from "#lib/components/live/LiveCard.svelte";
  import LivePanel from "#lib/components/live/LivePanel.svelte";

  const shownLives = progressive(() => Math.max(lives.results.length, lives.channelLives.length));
  const playingId = $derived(player.state.live?.id);

  onMount(() => lives.loadFavorites());

  function followers(n: number | null) {
    return n === null
      ? ""
      : `${new Intl.NumberFormat("fr-FR", { notation: "compact" }).format(n)} abonnés`;
  }

  const sub = (c: LiveChannel) => [c.handle, followers(c.followers)].filter(Boolean).join(" ");
</script>

{#snippet liveGrid(items: LiveStream[])}
  <div class="grid" class:stale={lives.loading}>
    {#each items.slice(0, shownLives.count) as stream (stream.id)}
      <LiveCard {stream} playing={stream.id === playingId} onplay={() => lives.play(stream)} />
    {/each}
  </div>
{/snippet}

{#snippet channelGrid(items: LiveChannel[])}
  <div class="channels">
    {#each items as c (c.id)}
      <ChannelCircle channel={c} sub={sub(c)} onclick={() => lives.openChannel(c)} />
    {/each}
  </div>
{/snippet}

<div class="page">
  <h1>Lives</h1>

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
      placeholder="Artiste, chaîne ou lien d'un live"
      aria-label="Rechercher un live"
    />
    <Button
      variant="primary"
      size={44}
      type="submit"
      disabled={lives.loading || !lives.query.trim()}
    >
      Rechercher
    </Button>
  </form>

  <LivePanel />

  {#if lives.channel}
    {@const c = lives.channel}
    <div class="channel-head">
      <IconButton icon="chevronLeft" label="Retour" onclick={() => (lives.channel = null)} />
      <img src={c.thumbnail} alt="" />
      <div class="who">
        <h2>{c.name}</h2>
        <span class="muted">{sub(c)}</span>
      </div>
      <Button onclick={() => lives.toggleFavorite(c)} aria-pressed={lives.isFavorite(c.id)}>
        <Icon name={lives.isFavorite(c.id) ? "heartFilled" : "heart"} size={16} />
        {lives.isFavorite(c.id) ? "Favori" : "Ajouter aux favoris"}
      </Button>
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
      <Section label="Chaînes">{@render channelGrid(lives.channels)}</Section>
    {/if}

    {#if lives.searched !== null && !lives.loading}
      <Section label="En direct">
        {#if lives.results.length === 0}
          <p class="status">Aucun live pour « {lives.searched} »</p>
        {:else}
          {@render liveGrid(lives.results)}
        {/if}
      </Section>
    {/if}

    {#if lives.favorites.length > 0}
      <Section label="Chaînes favorites">{@render channelGrid(lives.favorites)}</Section>
    {/if}
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 32px;
    padding-top: 20px;
  }
  form {
    display: flex;
    max-width: 720px;
    gap: 10px;
  }
  form .field {
    flex: 1;
    min-width: 0;
  }
  .status {
    margin: 0;
    color: var(--mute);
    font-size: 14px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 18px;
    transition: opacity 0.3s;
  }
  .grid.stale {
    opacity: 0.5;
  }
  .channels {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: 22px;
  }
  .channel-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
  }
  .channel-head img {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    object-fit: cover;
  }
  .who {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  @container page (max-width: 520px) {
    .grid {
      grid-template-columns: 1fr;
    }
    .channels {
      grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    }
  }
</style>

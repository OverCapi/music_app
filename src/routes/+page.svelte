<script lang="ts">
  import { commands } from "#lib/bindings.ts";

  let count = $state<number | null>(null);
  commands.getLibrary().then((tracks) => (count = tracks.length));
</script>

<p>{count ?? "…"} titres</p>

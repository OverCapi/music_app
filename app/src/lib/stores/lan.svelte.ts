import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type Direction,
  type IncomingRequest,
  type LanDone,
  type LanProgress,
  type Peer,
} from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

/** Library sync with the other devices of the local network. */
class Lan {
  deviceName = $state("");
  peers = $state<Peer[]>([]);
  /** A request from another device, waiting for the user's answer. */
  incoming = $state<IncomingRequest | null>(null);
  /** This device is waiting for an answer or pulling a library. */
  requesting = $state(false);
  progress = $state<LanProgress | null>(null);
  /** Outcome of the last sync of this session. */
  done = $state<LanDone | null>(null);
  /** Last thing worth telling about a request we made (refusal, acceptance). */
  notice = $state<string | null>(null);
  /** Code of our pending request, shown on the other device too. */
  code = $state<string | null>(null);

  /** A sync is running here (our request, or one we accepted). */
  get syncing() {
    return this.requesting || this.progress !== null;
  }

  async request(peer: Peer, direction: Direction, mirror: boolean) {
    this.requesting = true;
    this.done = null;
    this.notice = null;
    this.progress = null;
    const code = String(crypto.getRandomValues(new Uint16Array(1))[0] % 10000).padStart(4, "0");
    this.code = code;
    const accepted = await attempt(() => commands.lanRequestSync(peer.id, direction, mirror, code));
    this.requesting = false;
    this.code = null;
    if (accepted === false) this.notice = `${peer.name} a refusé la synchronisation.`;
    else if (accepted && direction === "send")
      this.notice = `${peer.name} a accepté : la bibliothèque est en cours d'envoi.`;
  }

  async answer(accept: boolean) {
    this.incoming = null;
    await attempt(() => commands.lanAnswer(accept));
  }

  async setDeviceName(name: string) {
    const saved = await attempt(() => commands.setDeviceName(name));
    if (saved !== undefined) this.deviceName = saved;
    return saved !== undefined;
  }

  async init(): Promise<UnlistenFn> {
    const unlisteners = await Promise.all([
      events.lanPeers.listen(({ payload }) => (this.peers = payload.peers)),
      events.lanRequest.listen(({ payload }) => (this.incoming = payload.request)),
      events.lanProgress.listen(({ payload }) => (this.progress = payload)),
      events.lanDone.listen(({ payload }) => {
        this.progress = null;
        this.done = payload;
      }),
    ]);
    const state = await commands.getLanState();
    this.deviceName = state.device_name;
    this.peers = state.peers;
    this.incoming = state.incoming;
    return () => unlisteners.forEach((u) => u());
  }
}

export const lan = new Lan();

<script lang="ts">
  import { getContext } from 'svelte';
  import {
    ACTION_ID,
    MAX_INPUT_BYTES,
    PROTOCOL,
    SENDS_PER_SECOND,
    SINGING_TIMEOUT_MS,
    VERSION,
    isHello,
    readPresence,
    type GameMessage,
    type HostMessage,
    type Json,
    type MicrophoneState,
  } from '@tawks/activity-sdk/protocol';
  import { PitchTracker } from '@tawks/activity-sdk/pitch';
  import { getNestedValue } from '../../utils';
  import { SINGING_EVENT, SingingHold } from './singing-hold';
  import type { Props } from '../workflow-display.svelte';
  import { submitServerAction } from '../workflow';
  import type { ActivityInfo, WorkflowStateProvider } from '../../workflow';

  // Every activity runs without an origin (no allow-same-origin), so it has
  // no cookies or storage anywhere. Bundles on apps.tawks.app are also served
  // with a sandbox CSP; any other tawks host would share cookies with the app.
  const HOSTED_HOST = 'apps.tawks.app';

  let { display, inputs, workflow }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  const spec = $derived(typeof display.type !== 'string' && 'Activity' in display.type ? display.type.Activity : null);
  const gameState = $derived((spec ? (getNestedValue(inputs, spec.state) ?? null) : null) as Json);
  const lobbyId = $derived((getNestedValue(inputs, 'lobby_id') as string | undefined) ?? null);

  let activity = $state<ActivityInfo | null>(null);
  let problem = $state<string | null>(null);
  let frame = $state<HTMLIFrameElement>();
  let box = $state<HTMLDivElement>();
  let port: MessagePort | null = null;

  type Client = { url: string };

  function checkClient(info: ActivityInfo): Client | string {
    let url: URL;
    try {
      url = new URL(info.client_url);
    } catch {
      return 'This game has an invalid address.';
    }
    const hosted = url.hostname === HOSTED_HOST;
    const tawksHost = url.hostname === 'tawks.app' || url.hostname.endsWith('.tawks.app');
    if (url.origin === location.origin || (tawksHost && !hosted)) {
      return 'This game is not allowed to load here.';
    }
    return { url: url.href };
  }

  const client = $derived(activity ? checkClient(activity) : null);
  const ready = $derived(client && typeof client !== 'string' ? client : null);

  const workflowId = $derived(workflow?.workflow_id);

  $effect(() => {
    const id = workflowId;
    activity = null;
    problem = null;
    if (!id || !provider?.findActivity) return;
    provider
      .findActivity(id)
      .then((found) => {
        if (workflowId !== id) return;
        if (found) activity = found;
        else problem = 'This game is not registered.';
      })
      .catch(() => (problem = 'This game could not be loaded.'));
  });

  $effect(() => {
    if (typeof client === 'string') problem = client;
  });

  const plain = (value: Json): Json => JSON.parse(JSON.stringify(value ?? null));

  // The voice client publishes the push-to-talk binding here and listens for
  // presses reported back (see the activity SDK's `push_to_talk` messages).
  const PUSH_TO_TALK_EVENT = 'tawks:push-to-talk';
  const PUSH_TO_TALK_BINDING_EVENT = 'tawks:push-to-talk-binding';
  const currentBinding = () =>
    ((window as unknown as { tawksPushToTalkBinding?: string | null }).tawksPushToTalkBinding ?? null);

  function post(message: HostMessage) {
    port?.postMessage(message);
  }

  let tokens = SENDS_PER_SECOND * 2;
  let refilledAt = performance.now();
  function allowSend() {
    const now = performance.now();
    tokens = Math.min(SENDS_PER_SECOND * 2, tokens + ((now - refilledAt) / 1000) * SENDS_PER_SECOND);
    refilledAt = now;
    if (tokens < 1) return false;
    tokens -= 1;
    return true;
  }

  // A game may only hold the microphone open while the player is looking at
  // it, and never for longer than a single long press.
  const MAX_TALK_MS = 120_000;
  let talking = false;
  let talkWatch: ReturnType<typeof setInterval> | null = null;
  let talkStarted = 0;

  function frameFocused() {
    return !!frame && document.activeElement === frame && document.visibilityState === 'visible';
  }

  function setTalking(down: boolean) {
    if (down && (!frameFocused() || talking)) return;
    if (!down && !talking) return;
    talking = down;
    window.dispatchEvent(new CustomEvent(PUSH_TO_TALK_EVENT, { detail: { down } }));
    if (talkWatch) clearInterval(talkWatch);
    talkWatch = null;
    if (down) {
      talkStarted = performance.now();
      talkWatch = setInterval(() => {
        if (!frameFocused() || performance.now() - talkStarted > MAX_TALK_MS) setTalking(false);
      }, 250);
    }
  }

  // Singing games get pitch from the microphone, never the sound. The player
  // agrees once per game, and can stop it from the pill at any time.
  let askingMic = $state(false);
  let micOn = $state(false);
  let micAllowed = false;
  let tracker: PitchTracker | null = null;
  let micStarting = false;
  let micRequest = 0;

  function reportMic(state: MicrophoneState) {
    post({ type: 'microphone', state });
  }

  async function startMic() {
    askingMic = false;
    micAllowed = true;
    if (tracker) return reportMic('on');
    if (micStarting) return;
    if (!navigator.mediaDevices?.getUserMedia) return reportMic('unavailable');
    const request = ++micRequest;
    const next = new PitchTracker((samples) => post({ type: 'pitch', samples }));
    micStarting = true;
    try {
      await next.start();
    } catch {
      next.stop();
      if (request === micRequest) {
        micStarting = false;
        reportMic('denied');
      }
      return;
    }
    if (request !== micRequest) {
      next.stop();
      return;
    }
    micStarting = false;
    tracker = next;
    micOn = true;
    reportMic('on');
  }

  function stopMic(state: MicrophoneState = 'off') {
    micRequest += 1;
    micStarting = false;
    tracker?.stop();
    tracker = null;
    micOn = false;
    askingMic = false;
    reportMic(state);
  }

  function onMicrophone(on: boolean) {
    if (!on) return stopMic();
    if (micAllowed) return void startMic();
    askingMic = true;
  }

  // The voice client mutes the lobby mic while this holds; the frame going
  // quiet or away lets go, so the hold never outlives the game.
  let singingHeld = $state(false);
  const singingHold = new SingingHold((on) => {
    singingHeld = on;
    window.dispatchEvent(new CustomEvent(SINGING_EVENT, { detail: { on } }));
  }, SINGING_TIMEOUT_MS);

  // A game's status line goes to the host at most this often; the last one in a burst wins.
  const PRESENCE_EVERY_MS = 2_000;
  let presenceSentAt = 0;
  let presenceTimer: ReturnType<typeof setTimeout> | null = null;
  let presenceShown = false;

  function onPresence(raw: unknown) {
    const presence = readPresence(raw);
    if (presence === undefined || !workflowId || !provider?.setActivityPresence) return;
    const id = workflowId;
    if (presenceTimer) clearTimeout(presenceTimer);
    const send = () => {
      presenceTimer = null;
      presenceSentAt = performance.now();
      presenceShown = presence !== null;
      provider.setActivityPresence?.(id, presence);
    };
    const wait = PRESENCE_EVERY_MS - (performance.now() - presenceSentAt);
    if (wait <= 0) send();
    else presenceTimer = setTimeout(send, wait);
  }

  function clearPresence() {
    if (presenceTimer) clearTimeout(presenceTimer);
    presenceTimer = null;
    if (presenceShown && workflowId) provider?.setActivityPresence?.(workflowId, null);
    presenceShown = false;
  }

  function fail(id: number, error: string) {
    post({ type: 'result', id, ok: false, error });
  }

  function onGameMessage(event: MessageEvent) {
    const message = event.data as GameMessage;
    if (typeof message !== 'object' || message === null) return;
    if (message.type === 'push_to_talk') {
      // Presses inside the frame never reach tawks; the voice client listens for this.
      setTalking(message.down === true);
      return;
    }
    if (message.type === 'microphone') {
      onMicrophone(message.on === true);
      return;
    }
    if (message.type === 'fullscreen') {
      void onFullscreen(message.on === true);
      return;
    }
    if (message.type === 'presence') {
      onPresence(message.presence);
      return;
    }
    if (message.type === 'singing') {
      singingHold.beat(message.on === true);
      return;
    }
    if (message.type !== 'send' && message.type !== 'call') return;
    const id = message.id;
    if (!Number.isSafeInteger(id)) return;

    let action: string | undefined;
    let payload: unknown;
    if (message.type === 'send') {
      action = spec?.action;
      payload = { input: message.input };
    } else {
      const inputs = message.inputs;
      if (typeof message.action !== 'string' || !ACTION_ID.test(message.action)) {
        return fail(id, 'That action id is not allowed.');
      }
      if (typeof inputs !== 'object' || inputs === null || Array.isArray(inputs)) {
        return fail(id, 'Inputs must be an object.');
      }
      if (Object.keys(inputs).some((key) => key.startsWith('$'))) {
        return fail(id, 'Input keys starting with $ are reserved.');
      }
      action = message.action;
      payload = inputs;
    }

    let encoded: string | undefined;
    try {
      encoded = JSON.stringify(payload);
    } catch {
      encoded = undefined;
    }
    if (encoded === undefined) return fail(id, 'The move must be JSON.');
    if (new TextEncoder().encode(encoded).length > MAX_INPUT_BYTES) return fail(id, 'The move is too large.');
    if (!allowSend()) return fail(id, 'Too many moves, slow down.');
    if (!action || !workflow) return fail(id, 'The game is not ready.');

    submitServerAction(provider, workflow.instance_id, JSON.parse(encoded), action)
      .then(() => post({ type: 'result', id, ok: true }))
      .catch((error: unknown) => fail(id, error instanceof Error ? error.message : 'The move was not delivered.'));
  }

  function onWindowMessage(event: MessageEvent) {
    if (!ready || !frame?.contentWindow || event.source !== frame.contentWindow) return;
    if (!isHello(event.data) || event.origin !== 'null') return;

    port?.close();
    const channel = new MessageChannel();
    port = channel.port1;
    port.onmessage = onGameMessage;
    frame.contentWindow.postMessage({ protocol: PROTOCOL, version: VERSION, type: 'welcome' }, '*', [
      channel.port2,
    ]);
    post({
      type: 'init',
      me: { id: workflow.user_id },
      lobby: lobbyId ? { id: lobbyId } : null,
      state: plain(gameState),
    });
    post({ type: 'push_to_talk', binding: currentBinding() });
  }

  function onBindingChange() {
    post({ type: 'push_to_talk', binding: currentBinding() });
  }

  // A framed game may not go full screen itself, so it asks and the panel
  // around it goes instead. The frame fills the panel, so it comes to the same
  // thing for the player.
  function fullscreenNow() {
    return !!box && document.fullscreenElement === box;
  }

  async function onFullscreen(on: boolean) {
    try {
      if (on && box) await box.requestFullscreen();
      else if (!on && fullscreenNow()) await document.exitFullscreen();
    } catch {
      // Report whatever the browser left us in.
    }
    post({ type: 'fullscreen', on: fullscreenNow() });
  }

  function onFullscreenChange() {
    post({ type: 'fullscreen', on: fullscreenNow() });
  }

  $effect(() => {
    window.addEventListener('message', onWindowMessage);
    window.addEventListener(PUSH_TO_TALK_BINDING_EVENT, onBindingChange);
    document.addEventListener('fullscreenchange', onFullscreenChange);
    return () => {
      window.removeEventListener('message', onWindowMessage);
      window.removeEventListener(PUSH_TO_TALK_BINDING_EVENT, onBindingChange);
      document.removeEventListener('fullscreenchange', onFullscreenChange);
      port?.close();
      port = null;
      clearPresence();
      singingHold.release();
      setTalking(false);
      micRequest += 1;
      micStarting = false;
      tracker?.stop();
      tracker = null;
    };
  });

  $effect(() => {
    const state = plain(gameState);
    post({ type: 'state', state });
  });
</script>

<div
  bind:this={box}
  class="activity-frame relative h-full min-h-[70dvh] w-full overflow-hidden bg-black"
>
  {#if ready}
    <iframe
      bind:this={frame}
      src={ready.url}
      title={activity?.name ?? 'Game'}
      class="absolute inset-0 h-full w-full border-0"
      sandbox="allow-scripts allow-pointer-lock"
      allow="fullscreen; gamepad; autoplay"
      allowfullscreen
      referrerpolicy="no-referrer"
    ></iframe>
    {#if askingMic}
      <div class="absolute inset-0 z-10 flex items-center justify-center bg-black/70 p-6">
        <div class="max-w-sm rounded-xl border border-white/10 bg-zinc-950 p-5 text-sm text-zinc-300 shadow-2xl">
          <p class="font-semibold text-white">{activity?.name ?? 'This game'} wants to hear you sing</p>
          <p class="mt-2">
            It only gets the pitch and how loud you are, worked out here. Your voice is never sent to the game or anyone
            else.
          </p>
          <div class="mt-4 flex justify-end gap-2">
            <button class="rounded-md px-3 py-1.5 text-zinc-400 hover:text-white" onclick={() => stopMic('denied')}>
              Not now
            </button>
            <button
              class="bg-primary text-primary-foreground rounded-md px-3 py-1.5 font-semibold"
              onclick={() => void startMic()}
            >
              Allow
            </button>
          </div>
        </div>
      </div>
    {/if}
    {#if micOn || singingHeld}
      <div class="absolute bottom-2 left-1/2 z-10 flex -translate-x-1/2 items-center gap-2">
        {#if micOn}
          <div
            class="flex items-center gap-2 rounded-full border border-white/10 bg-black/70 px-3 py-1 text-xs whitespace-nowrap text-zinc-300"
          >
            <span class="bg-primary size-1.5 rounded-full"></span>
            Mic on for pitch
            <button class="text-zinc-400 hover:text-white" onclick={() => stopMic()}>Stop</button>
          </div>
        {/if}
        {#if singingHeld}
          <div
            class="flex items-center gap-2 rounded-full border border-white/10 bg-black/70 px-3 py-1 text-xs whitespace-nowrap text-zinc-300"
          >
            <span class="size-1.5 rounded-full bg-red-400"></span>
            Lobby mic muted while you sing
          </div>
        {/if}
      </div>
    {/if}
  {:else if problem}
    <div class="flex h-full items-center justify-center p-6 text-sm text-zinc-400">{problem}</div>
  {/if}
</div>

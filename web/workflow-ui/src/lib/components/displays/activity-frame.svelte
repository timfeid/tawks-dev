<script lang="ts">
  import { getContext } from 'svelte';
  import {
    ACTION_ID,
    MAX_INPUT_BYTES,
    PROTOCOL,
    SENDS_PER_SECOND,
    VERSION,
    isHello,
    type GameMessage,
    type HostMessage,
    type Json,
  } from '@tawks/activity-sdk/protocol';
  import { getNestedValue } from '../../utils';
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

  $effect(() => {
    window.addEventListener('message', onWindowMessage);
    window.addEventListener(PUSH_TO_TALK_BINDING_EVENT, onBindingChange);
    return () => {
      window.removeEventListener('message', onWindowMessage);
      window.removeEventListener(PUSH_TO_TALK_BINDING_EVENT, onBindingChange);
      port?.close();
      port = null;
      setTalking(false);
    };
  });

  $effect(() => {
    const state = plain(gameState);
    post({ type: 'state', state });
  });
</script>

<div class="relative h-full min-h-[70dvh] w-full overflow-hidden bg-black {display.class}">
  {#if ready}
    <iframe
      bind:this={frame}
      src={ready.url}
      title={activity?.name ?? 'Game'}
      class="absolute inset-0 h-full w-full border-0"
      sandbox="allow-scripts allow-pointer-lock"
      allow="fullscreen; gamepad; autoplay"
      referrerpolicy="no-referrer"
    ></iframe>
    <div
      class="pointer-events-none absolute top-2 left-2 rounded bg-black/60 px-2 py-0.5 text-xs text-zinc-300"
    >
      {activity?.name}
    </div>
  {:else if problem}
    <div class="flex h-full items-center justify-center p-6 text-sm text-zinc-400">{problem}</div>
  {/if}
</div>

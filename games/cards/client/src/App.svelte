<script lang="ts">
  import { setContext } from 'svelte';
  import { connect, type Activity, type Json } from '@tawks/activity-sdk';
  import BattlefieldCanvas from '@tawks/game-canvases/battlefield-canvas.svelte';

  const INSTANCE_ID = 'activity';

  let activity = $state<Activity | null>(null);
  let game = $state<Json>(null);
  let problem = $state<string | null>(null);

  // The battlefield was written against the tawks workflow provider; inside
  // the frame, every action it runs goes to the bot through the bridge.
  setContext('workflow-provider', {
    byInstance: {},
    activeChildForParent: {},
    returnedToParent: {},
    playAnimations() {},
    processAction(_instanceId: string, actionId: string, inputs: Record<string, Json>) {
      if (!activity) return Promise.reject(new Error('Not connected to tawks'));
      return activity.call(actionId, inputs).catch((error: Error) => console.warn(actionId, error.message));
    },
  });

  connect()
    .then((connected) => {
      activity = connected;
      game = connected.state;
      connected.onState((state) => (game = state));
    })
    .catch((error: Error) => (problem = error.message));

  const workflow = $derived({
    instance_id: INSTANCE_ID,
    workflow_id: 'cardgame_activity',
    user_id: activity?.me.id ?? '',
    current_node_id: 'table',
    responses: { game, lobby_id: activity?.lobby?.id ?? null },
    waiting: false,
    completed: false,
    visuals: [],
    attached_workflows: {},
  } as any);
</script>

{#if activity}
  <div class="relative h-full w-full">
    <BattlefieldCanvas {workflow} />
  </div>
{:else}
  <div class="flex h-full items-center justify-center text-sm text-zinc-400">
    {problem ?? 'Connecting to tawks...'}
  </div>
{/if}

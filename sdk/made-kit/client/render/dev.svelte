<script lang="ts">
  import MadeGame from '../src/made-game.svelte';
  import { LocalSource, type LocalRunner } from '../src/local-source';
  // @ts-expect-error the kit core is plain JS with no types
  import { createRunner } from '../../src/runner.js';
  // @ts-expect-error the example game is plain JS
  import thinIce from '../../examples/thin-ice.js';

  const params = new URLSearchParams(location.search);
  const seats = Math.max(2, Math.min(8, Number(params.get('seats') ?? 4)));
  const watch = params.has('watch');
  const roster = Array.from({ length: seats }, (_, i) => ({
    seat: i + 1,
    name: i === 0 && !watch ? 'You' : `Bot ${i + 1}`,
    bot: watch || i > 0,
    costume: null,
    connected: true,
  }));
  const start = () =>
    createRunner(thinIce, { seats: roster.map((s) => ({ ...s, skill: 0.5 })), seed: Math.floor(Math.random() * 1e6) }) as unknown as LocalRunner;
  const source = new LocalSource(start, roster, watch ? null : 1);
  (window as unknown as { madeSource: LocalSource }).madeSource = source;
</script>

<div class="h-screen w-screen">
  <MadeGame {source} />
</div>

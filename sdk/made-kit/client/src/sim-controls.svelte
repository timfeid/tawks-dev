<script lang="ts">
  import { Button } from '@tawks/ui/button/index.js';
  import { Slider } from '@tawks/ui/slider/index.js';
  import { Switch } from '@tawks/ui/switch/index.js';
  import { Input } from '@tawks/ui/input/index.js';
  import * as RadioGroup from '@tawks/ui/radio-group/index.js';
  import type { ParamSpec, ParamValue } from './contract';
  import { formatNumber } from './sim-renderer';

  /** The sim's controls. While one is being moved it shows the local value until the sim's own catches up. */
  let {
    params,
    values,
    disabled,
    onset,
  }: {
    params: ParamSpec[];
    values: Partial<Record<string, ParamValue>>;
    disabled: boolean;
    onset: (key: string, value: ParamValue) => void;
  } = $props();

  let editing = $state<Record<string, { value: ParamValue; until: number }>>({});
  const HOLD_MS = 600;
  const SEND_GAP_MS = 40;
  const lastSent: Record<string, number> = {};
  const pending: Record<string, ReturnType<typeof setTimeout>> = {};

  function shown(p: ParamSpec): ParamValue {
    const local = editing[p.key];
    if (local && local.until > performance.now()) return local.value;
    return values[p.key] ?? p.value;
  }

  // The slider snaps a value that is off its step, and that must not reach the sim as if someone moved it.
  let touchedUntil: Record<string, number> = {};
  const touch = (key: string) => (touchedUntil[key] = performance.now() + 1500);
  const touched = (key: string) => (touchedUntil[key] ?? 0) > performance.now();

  function change(p: ParamSpec, value: ParamValue) {
    editing = { ...editing, [p.key]: { value, until: performance.now() + HOLD_MS } };
    const now = performance.now();
    clearTimeout(pending[p.key]);
    const wait = (lastSent[p.key] ?? 0) + SEND_GAP_MS - now;
    const send = () => {
      lastSent[p.key] = performance.now();
      onset(p.key, value);
    };
    if (wait <= 0) send();
    else pending[p.key] = setTimeout(send, wait);
  }

  const asNumber = (v: ParamValue) => (typeof v === 'number' ? v : Number(v) || 0);
</script>

<div class="space-y-4" data-sim-controls>
  {#each params as p (p.key)}
    <div class="space-y-1.5" data-sim-control={p.key}>
      {#if p.kind === 'slider'}
        <div class="flex items-baseline justify-between gap-2 text-sm">
          <span class="font-medium" id="sim-{p.key}">{p.label}</span>
          <span class="text-muted-foreground tabular-nums">{formatNumber(asNumber(shown(p)), 3)}{p.unit ? ` ${p.unit}` : ''}</span>
        </div>
        <Slider
          type="single"
          value={asNumber(shown(p))}
          min={p.min ?? 0}
          max={p.max ?? 1}
          step={p.step ?? 0.01}
          {disabled}
          thumbLabel={p.label}
          onpointerdown={() => touch(p.key)}
          onpointermove={(event: PointerEvent) => event.buttons && touch(p.key)}
          onkeydown={() => touch(p.key)}
          onValueChange={(value: number | number[]) => {
            if (touched(p.key)) change(p, Array.isArray(value) ? value[0] : value);
          }}
        />
      {:else if p.kind === 'number'}
        <label class="flex items-center justify-between gap-3 text-sm">
          <span class="font-medium">{p.label}</span>
          <span class="flex items-center gap-2">
            <Input
              type="number"
              class="h-8 w-24 text-right tabular-nums"
              value={asNumber(shown(p))}
              min={p.min ?? undefined}
              max={p.max ?? undefined}
              step={p.step ?? undefined}
              {disabled}
              onchange={(event: Event) => {
                const n = Number((event.currentTarget as HTMLInputElement).value);
                if (Number.isFinite(n)) change(p, n);
              }}
            />
            {#if p.unit}<span class="text-muted-foreground text-xs">{p.unit}</span>{/if}
          </span>
        </label>
      {:else if p.kind === 'toggle'}
        <label class="flex items-center justify-between gap-3 text-sm">
          <span class="font-medium">{p.label}</span>
          <Switch checked={shown(p) === true} {disabled} aria-label={p.label} onCheckedChange={(checked: boolean) => change(p, checked)} />
        </label>
      {:else if p.kind === 'choice'}
        <p class="text-sm font-medium">{p.label}</p>
        <RadioGroup.Root
          class="flex flex-wrap gap-x-4 gap-y-2"
          value={String(shown(p))}
          {disabled}
          onValueChange={(value: string) => change(p, value)}
          aria-label={p.label}
        >
          {#each p.options as option (option.value)}
            <label class="flex items-center gap-2 text-sm">
              <RadioGroup.Item value={option.value} {disabled} />
              {option.label}
            </label>
          {/each}
        </RadioGroup.Root>
      {:else if p.kind === 'button'}
        <Button variant="secondary" class="w-full" {disabled} onclick={() => onset(p.key, true)}>{p.label}</Button>
      {/if}
      {#if p.help}<p class="text-muted-foreground text-xs">{p.help}</p>{/if}
    </div>
  {/each}
</div>

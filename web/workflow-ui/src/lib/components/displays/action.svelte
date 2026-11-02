<script lang="ts">
  import { getContext } from 'svelte';
  import { interpolate } from '../../utils';
  import * as look from '../../look';
  import { pressable, scrollToId } from '../../act.svelte';
  import type { Props } from '../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './icon/registry.js';

  let { display, inputs, workflow, buttonClicked }: Props = $props();
  let action = $derived(typeof display.type !== 'string' && 'Action' in display.type ? display.type.Action : null);
  let label = $derived(action ? interpolate(inputs, action.label) : '');
  let Icon = $derived(action?.icon ? LUCIDE_REGISTRY[interpolate(inputs, action.icon)] : undefined);
  const press = pressable(() => ({ workflow, inputs }));
  const formSubmit = getContext<(() => void) | undefined>('form-submit');
  let disabled = $derived(press.pending || press.disabled(action?.act));

  // A form clears its fields as it sends, so a second press would send them empty.
  let sentJustNow = false;
  function holdSecondSubmit(event: MouseEvent) {
    if (!action?.submit) return;
    if (sentJustNow) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    sentJustNow = true;
    setTimeout(() => (sentJustNow = false), 1000);
  }

  function onclick(event: MouseEvent) {
    if (!action) return;
    if (action.submit && formSubmit) {
      // The enclosing form sends the typed values along with this action.
      event.preventDefault();
      formSubmit();
      return;
    }
    press.run(action.act);
    if (action.scroll_to) scrollToId(interpolate(inputs, action.scroll_to));
    buttonClicked?.();
  }
</script>

{#if action}
  <button
    type={action.submit ? 'submit' : 'button'}
    class={[look.action[action.look], action.full_width && 'w-full', disabled && 'cursor-not-allowed opacity-50', press.pending && 'cursor-progress']}
    aria-label={action.look === 'Icon' ? label : undefined}
    title={action.look === 'Icon' ? label : undefined}
    aria-busy={press.pending}
    {disabled}
    onclickcapture={holdSecondSubmit}
    {onclick}
  >
    {#if Icon}<Icon class="size-4 shrink-0" />{/if}
    {#if action.look !== 'Icon'}<span>{label}</span>{/if}
  </button>
{/if}

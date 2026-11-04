<script lang="ts">
  import { getContext } from 'svelte';
  import { shown } from '../../member-names.svelte';
  import * as look from '../../look';
  import { pressable, scrollToId } from '../../act.svelte';
  import type { Props } from '../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './icon/registry.js';

  let { display, inputs, workflow, buttonClicked }: Props = $props();
  let action = $derived(typeof display.type !== 'string' && 'Action' in display.type ? display.type.Action : null);
  let label = $derived(action ? shown(inputs, action.label) : '');
  let Icon = $derived(action?.icon ? LUCIDE_REGISTRY[shown(inputs, action.icon)] : undefined);
  const press = pressable(() => ({ workflow, inputs }));
  const formSubmit = getContext<
    ((sent: { action: string; inputs: Record<string, any> } | null, from: Element | null) => void) | undefined
  >('form-submit');
  let disabled = $derived(press.pending || press.disabled(action?.act));

  // A double click on a form's button is one press, whatever the form is doing.
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
      // The enclosing form sends the typed values along with this action and
      // what this button carries, such as the row it sits in.
      event.preventDefault();
      if (press.disabled(action.act)) return;
      formSubmit(press.resolved(action.act), event.currentTarget as Element);
      return;
    }
    press.run(action.act);
    if (action.scroll_to) scrollToId(shown(inputs, action.scroll_to));
    buttonClicked?.();
  }
</script>

{#if action}
  <button
    type={action.submit ? 'submit' : 'button'}
    class={[look.action[action.look], 'max-w-full shrink-0', action.full_width && 'w-full', disabled && 'cursor-not-allowed opacity-50', press.pending && 'cursor-progress']}
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

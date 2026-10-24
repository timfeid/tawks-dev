<script lang="ts">
  import UserAvatar from './displays/user-avatar.svelte';
  import { interpolate, setNestedValue } from '../utils';
  import type { DisplayType, WorkflowDisplay, WorkflowResource } from '@tawks/workflow-types';
  import Carousel from './displays/carousel.svelte';
  import Card from './displays/card.svelte';
  import Text from './displays/text.svelte';
  import Progress from './displays/progress.svelte';
  import List from './displays/list.svelte';
  import Div from './displays/div.svelte';
  import Image from './displays/image.svelte';
  import Video from './displays/video.svelte';
  import AudioClip from './displays/audio-clip.svelte';
  import ActivityFrame from './displays/activity-frame.svelte';
  import ShareResult from './displays/share-result.svelte';
  import Page from './displays/page.svelte';
  import CoverImage from './displays/cover-image.svelte';
  import Table from './displays/table.svelte';
  import Button from './displays/button.svelte';
  import Condition from './displays/condition.svelte';
  import Input from './displays/input.svelte';
  import Tooltip from './displays/tooltip.svelte';
  import Tile from './displays/tile.svelte';
  import Countdown from './displays/countdown.svelte';
  import TooltipButton from './displays/tooltip-button.svelte';
  import Wysiwyg from './displays/wysiwyg.svelte';
  import Icon from './displays/icon/icon.svelte';
  import Badge from './displays/badge.svelte';
  import Chart from './displays/chart.svelte';
  import Time from './displays/time.svelte';
  import Searchable from './displays/searchable.svelte';
  import Popover from './displays/popover.svelte';
  import Form from './displays/form.svelte';
  import Loader from './displays/loader.svelte';
  import CommandPalette from './displays/command-palette.svelte';
  import ScoreTicker from './displays/score-ticker.svelte';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../workflow';

  export type Props = {
    workflow: WorkflowResource;
    display: WorkflowDisplay;
    inputs: Record<string, unknown>;
    inputsChanged?: (key: string, value: any) => void;
    buttonClicked?: () => void;
    from: string;
  };

  let originalProps: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');
  let inputOverrides = $state<Record<string, unknown>>({});
  let lastInstanceId = $state(originalProps.workflow.instance_id);

  $effect(() => {
    if (originalProps.workflow.instance_id !== lastInstanceId) {
      lastInstanceId = originalProps.workflow.instance_id;
      inputOverrides = {};
    }
  });

  function scrollOwnContainerToTop(event: MouseEvent) {
    event.preventDefault();
    let el = (event.currentTarget as HTMLElement | null)?.parentElement ?? null;
    while (el) {
      const overflowY = getComputedStyle(el).overflowY;
      if ((overflowY === 'auto' || overflowY === 'scroll') && el.scrollHeight > el.clientHeight) {
        el.scrollTo({ top: 0, behavior: 'smooth' });
        return;
      }
      el = el.parentElement;
    }
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  // Native shells (Tauri) register window.tawksOpenExternal so external
  // links leave the webview instead of navigating inside it.
  function openExternal(event: MouseEvent) {
    const href = (event.currentTarget as HTMLAnchorElement | null)?.href;
    const opener = (window as unknown as { tawksOpenExternal?: (url: string) => void }).tawksOpenExternal;
    if (href && opener) {
      event.preventDefault();
      opener(href);
    }
  }

  /** Bots may link out over http(s) or to an in-app path; nothing else. */
  function safeHref(href: string | null | undefined): string | null {
    if (!href || /\{[^{}]+\}/.test(href)) return null;
    const trimmed = href.trim();
    if (/^https?:\/\/[^\s]+$/i.test(trimmed)) return trimmed;
    if (/^\/(?![\/\\])/.test(trimmed) || trimmed === '#top') return trimmed;
    return null;
  }

  function handleInputsChanged(key: string, value: unknown) {
    inputOverrides = { ...inputOverrides, [key]: value };
    originalProps.inputsChanged?.(key, value);
  }

  let mergedInputs = $derived.by(() => {
    const merged = { ...originalProps.inputs };
    for (const [key, value] of Object.entries(inputOverrides)) {
      setNestedValue(merged, key, value as any);
    }
    return { ...merged, $user_id: originalProps.workflow.user_id };
  });

  let finalProps = $derived({
    ...originalProps,
    display: {
      ...originalProps.display,
      class: interpolate(mergedInputs, originalProps.display.class),
    },
    inputs: mergedInputs,
    inputsChanged: handleInputsChanged,
  });
</script>

{#if finalProps.display.type}
  {@const Custom = typeof finalProps.display.type === 'string' ? provider?.customComponents?.[finalProps.display.type] : Object.keys(finalProps.display.type).map(k => provider?.customComponents?.[k]).find(Boolean)}
  {#if Custom}
    <Custom {...finalProps} />
  {:else if finalProps.display.type === 'MultimediaLobby'}
    <div class="p-8 border-2 border-dashed rounded-xl bg-muted/10 text-center">
        <div class="font-bold text-lg mb-2">Multimedia Lobby</div>
        <div class="text-sm text-muted-foreground">ID: {interpolate(finalProps.inputs, '{lobby_id}')}</div>
    </div>
  {:else if finalProps.display.type === 'Loader'}
    <Loader {...finalProps} />
  {:else if typeof finalProps.display.type === 'string'}
    <div class="rounded border border-dashed p-3 text-xs text-muted-foreground {finalProps.display.class}">
      {finalProps.display.type} is drawn by tawks itself and is not available here.
    </div>
  {:else if 'Carousel' in finalProps.display.type}
    <Carousel {...finalProps} />
  {:else if 'Countdown' in finalProps.display.type}
    <Countdown {...finalProps} />
  {:else if 'TooltipButton' in finalProps.display.type}
    <TooltipButton {...finalProps} />
  {:else if 'Tooltip' in finalProps.display.type}
    <Tooltip {...finalProps} />
  {:else if 'Input' in finalProps.display.type}
    <Input {...finalProps} />
  {:else if 'Condition' in finalProps.display.type}
    <Condition {...finalProps} />
  {:else if 'Button' in finalProps.display.type}
    <Button {...finalProps} />
  {:else if 'Table' in finalProps.display.type}
    <Table {...finalProps} />
  {:else if 'Page' in finalProps.display.type}
    <Page {...finalProps} />
  {:else if 'CoverImage' in finalProps.display.type}
    <CoverImage {...finalProps} />
  {:else if 'Video' in finalProps.display.type}
    <Video {...finalProps} />
  {:else if 'AudioClip' in finalProps.display.type}
    <AudioClip {...finalProps} />
  {:else if 'Activity' in finalProps.display.type}
    <ActivityFrame {...finalProps} />
  {:else if 'ShareResult' in finalProps.display.type}
    <ShareResult {...finalProps} />
  {:else if 'Image' in finalProps.display.type}
    <Image {...finalProps} />
  {:else if 'Div' in finalProps.display.type}
    <Div {...finalProps} />
  {:else if 'List' in finalProps.display.type}
    <List {...finalProps} />
  {:else if 'Card' in finalProps.display.type}
    <Card {...finalProps} />
  {:else if 'Progress' in finalProps.display.type}
    <Progress {...finalProps} />
  {:else if 'Tile' in finalProps.display.type}
    <Tile {...finalProps} />
  {:else if 'Wysiwyg' in finalProps.display.type}
    <Wysiwyg {...finalProps} />
  {:else if 'Icon' in finalProps.display.type}
    <Icon {...finalProps} />
  {:else if 'Badge' in finalProps.display.type}
    <Badge {...finalProps} />
  {:else if 'Chart' in finalProps.display.type}
    <Chart {...finalProps} />
  {:else if 'Time' in finalProps.display.type}
    <Time {...finalProps} />
  {:else if 'Form' in finalProps.display.type}
    <Form {...finalProps} />
  {:else if 'Popover' in finalProps.display.type}
    <Popover {...finalProps} />
  {:else if 'Searchable' in finalProps.display.type}
    <Searchable {...finalProps} />
  {:else if 'CommandPalette' in finalProps.display.type}
    <CommandPalette {...finalProps} />
  {:else if 'ScoreTicker' in finalProps.display.type}
    <ScoreTicker {...finalProps} />
  {:else if 'UserAvatar' in finalProps.display.type}
    <UserAvatar {...finalProps} />
  {:else if 'Link' in finalProps.display.type}
    {@const linkType = finalProps.display.type.Link}
    {@const href = safeHref(interpolate(finalProps.inputs, linkType.href))}
    {#if href}
      {@const external = /^https?:\/\//i.test(href)}
      <a
        {href}
        class={finalProps.display.class}
        target={external ? '_blank' : undefined}
        rel={external ? 'noopener noreferrer' : undefined}
        onclick={href === '#top' ? scrollOwnContainerToTop : external ? openExternal : undefined}
      >
        {#each linkType.content as child}
          <svelte:self {...finalProps} display={child} />
        {/each}
      </a>
    {:else}
      <!-- href could not be resolved (missing slug, etc.) — render as a plain block so the card still shows -->
      <div class={finalProps.display.class}>
        {#each linkType.content as child}
          <svelte:self {...finalProps} display={child} />
        {/each}
      </div>
    {/if}
  {:else if 'Text' in finalProps.display.type}
    <Text {...finalProps} />
  {:else}
    {@const kind = typeof finalProps.display.type === 'string' ? finalProps.display.type : Object.keys(finalProps.display.type)[0]}
    <div class="rounded border border-dashed p-3 text-xs text-muted-foreground {finalProps.display.class}">
      {kind} is drawn by tawks itself and is not available here.
    </div>
  {/if}
{/if}

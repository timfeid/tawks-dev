<script lang="ts">
  import CustomDisplay from './custom-display.svelte';
  import UserAvatar from './displays/user-avatar.svelte';
  import { interpolate, pictureUrl, safeHref, withNestedValue } from '../utils';
  import { mediaUrl } from '../media';
  import * as look from '../look';
  import { clientInfo } from '../client';
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
  import Typography from './displays/typography.svelte';
  import Stack from './displays/stack.svelte';
  import Grid from './displays/grid.svelte';
  import Panel from './displays/panel.svelte';
  import Hero from './displays/hero.svelte';
  import Section from './displays/section.svelte';
  import Feature from './displays/feature.svelte';
  import Chip from './displays/chip.svelte';
  import NoticeBlock from './displays/notice.svelte';
  import TabBarBlock from './displays/tab-bar.svelte';
  import ButtonGroupBlock from './displays/button-group.svelte';
  import StepperBlock from './displays/stepper.svelte';
  import RowBlock from './displays/row.svelte';
  import ChoiceBlock from './displays/choice.svelte';
  import StatBlock from './displays/stat.svelte';
  import MeterBlock from './displays/meter.svelte';
  import SegmentsBlock from './displays/segments.svelte';
  import EmptyBlock from './displays/empty.svelte';
  import PersonBlock from './displays/person.svelte';
  import AvatarStackBlock from './displays/avatar-stack.svelte';
  import DividerBlock from './displays/divider.svelte';
  import LogBlock from './displays/log.svelte';
  import SplitBlock from './displays/split.svelte';
  import ActionBarBlock from './displays/action-bar.svelte';
  import MediaBlock from './displays/media.svelte';
  import OverlayBlock from './displays/overlay.svelte';
  import ItemsBlock from './displays/items.svelte';
  import Backdrop from './displays/backdrop.svelte';
  import Calendar from './displays/calendar.svelte';
  import ActionBlock from './displays/action.svelte';
  import PlayGame from './displays/play-game.svelte';
  import Discussion from './displays/discussion.svelte';
  import { getContext, type Component } from 'svelte';
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
  let brokenThumbs = $state<string[]>([]);
  function dropThumb(thumb: string) {
    if (!brokenThumbs.includes(thumb)) brokenThumbs = [...brokenThumbs, thumb];
  }
  let lastStep = $state(`${originalProps.workflow.instance_id}:${originalProps.workflow.current_node_id}`);

  $effect(() => {
    const step = `${originalProps.workflow.instance_id}:${originalProps.workflow.current_node_id}`;
    if (step !== lastStep) {
      lastStep = step;
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

  function handleInputsChanged(key: string, value: unknown) {
    inputOverrides = { ...inputOverrides, [key]: value };
    originalProps.inputsChanged?.(key, value);
  }

  let mergedInputs = $derived.by(() => {
    const inputs = originalProps.inputs;
    const userId = originalProps.workflow.user_id;
    const client = clientInfo();
    const overrides = Object.entries(inputOverrides);
    if (!overrides.length && inputs.$user_id === userId && inputs.$client === client) return inputs;
    let merged: Record<string, unknown> = { ...inputs, $user_id: userId, $client: client };
    for (const [key, value] of overrides) {
      merged = withNestedValue(merged, key, value as any);
    }
    return merged;
  });

  // Getters rather than a derived copy, so a child only reruns for the prop that changed.
  const finalProps: Props = {
    get workflow() {
      return originalProps.workflow;
    },
    get display() {
      return originalProps.display;
    },
    get from() {
      return originalProps.from;
    },
    get buttonClicked() {
      return originalProps.buttonClicked;
    },
    get inputs() {
      return mergedInputs;
    },
    inputsChanged: handleInputsChanged,
  };

  // A lookup, not a chain of {#if} branches: every branch Svelte has to pass
  // through is another nested effect, and nested pages ran out of stack.
  const blocks: Record<string, Component<Props>> = {
    Typography,
    Stack,
    Grid,
    Panel,
    Hero,
    Section,
    Feature,
    Chip,
    Notice: NoticeBlock,
    TabBar: TabBarBlock,
    ButtonGroup: ButtonGroupBlock,
    Stepper: StepperBlock,
    Row: RowBlock,
    Choice: ChoiceBlock,
    Stat: StatBlock,
    Meter: MeterBlock,
    Segments: SegmentsBlock,
    Empty: EmptyBlock,
    Person: PersonBlock,
    AvatarStack: AvatarStackBlock,
    Divider: DividerBlock,
    Log: LogBlock,
    Split: SplitBlock,
    ActionBar: ActionBarBlock,
    Media: MediaBlock,
    Backdrop,
    Calendar,
    Items: ItemsBlock,
    Overlay: OverlayBlock,
    Action: ActionBlock,
    Carousel,
    Countdown,
    TooltipButton,
    Tooltip,
    Input,
    Condition,
    Button,
    Table,
    Page,
    CoverImage,
    Video,
    AudioClip,
    ShareResult,
    Image,
    Div,
    List,
    Card,
    Progress,
    Tile,
    Wysiwyg,
    Icon,
    Badge,
    Chart,
    Time,
    Form,
    Popover,
    Searchable,
    CommandPalette,
    ScoreTicker,
    UserAvatar,
    Text,
    PlayGame,
    Discussion,
  };

  let mark = $derived(originalProps.display.mark ? interpolate(mergedInputs, originalProps.display.mark) : '');
</script>

{#if mark}
  <div data-mark={mark}>{@render drawn()}</div>
{:else}
  {@render drawn()}
{/if}

{#snippet drawn()}
{#if finalProps.display.type}
  {@const Custom = typeof finalProps.display.type === 'string' ? provider?.customComponents?.[finalProps.display.type] : Object.keys(finalProps.display.type).map(k => provider?.customComponents?.[k]).find(Boolean)}
  {@const Block = typeof finalProps.display.type === 'string' ? undefined : blocks[Object.keys(finalProps.display.type)[0]]}
  {#if Custom}
    <CustomDisplay component={Custom} componentProps={finalProps} {provider} type={typeof finalProps.display.type === 'string' ? finalProps.display.type : Object.keys(finalProps.display.type)[0]} />
  {:else if finalProps.display.type === 'MultimediaLobby'}
    <div class="p-8 border-2 border-dashed rounded-xl bg-muted/10 text-center">
        <div class="font-bold text-lg mb-2">Multimedia Lobby</div>
        <div class="text-sm text-muted-foreground">ID: {interpolate(finalProps.inputs, '{lobby_id}')}</div>
    </div>
  {:else if finalProps.display.type === 'Loader'}
    <Loader {...finalProps} />
  {:else if typeof finalProps.display.type === 'string'}
    <div class="rounded border border-dashed p-3 text-xs text-muted-foreground">
      {finalProps.display.type} is drawn by tawks itself and is not available here.
    </div>
  {:else if Block}
    <Block {...finalProps} />
  {:else if 'Activity' in finalProps.display.type}
    <CustomDisplay component={ActivityFrame} componentProps={finalProps} {provider} type="ActivityFrame" />
  {:else if 'Link' in finalProps.display.type}
    {@const linkType = finalProps.display.type.Link}
    {@const href = safeHref(interpolate(finalProps.inputs, linkType.href))}
    {#if href}
      {@const external = /^https?:\/\//i.test(href)}
      {@const thumb = linkType.thumb ? pictureUrl(interpolate(finalProps.inputs, linkType.thumb)) : null}
      <a
        {href}
        class={look.link[linkType.style ?? 'Plain']}
        target={external ? '_blank' : undefined}
        rel={external ? 'noopener noreferrer' : undefined}
        onclick={href === '#top' ? scrollOwnContainerToTop : external ? openExternal : undefined}
      >
        {#if thumb && !brokenThumbs.includes(thumb)}
          <div class="flex items-start gap-4">
            <div class="flex min-w-0 flex-1 flex-col">
              {#each linkType.content as child}
                <svelte:self {...finalProps} display={child} />
              {/each}
            </div>
            <img
              src={mediaUrl(thumb)}
              alt=""
              loading="lazy"
              class={look.thumb}
              onerror={() => dropThumb(thumb)}
              onload={(event) => {
                if ((event.currentTarget as HTMLImageElement).naturalWidth < 8) dropThumb(thumb);
              }}
            />
          </div>
        {:else}
          {#each linkType.content as child}
            <svelte:self {...finalProps} display={child} />
          {/each}
        {/if}
      </a>
    {:else}
      <!-- href could not be resolved (missing slug, etc.) — render as a plain block so the card still shows -->
      <div>
        {#each linkType.content as child}
          <svelte:self {...finalProps} display={child} />
        {/each}
      </div>
    {/if}
  {:else if !('Unknown' in finalProps.display.type)}
    {@const kind = typeof finalProps.display.type === 'string' ? finalProps.display.type : Object.keys(finalProps.display.type)[0]}
    <div class="rounded border border-dashed p-3 text-xs text-muted-foreground">
      {kind} is drawn by tawks itself and is not available here.
    </div>
  {/if}
{/if}
{/snippet}

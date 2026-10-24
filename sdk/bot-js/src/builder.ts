import type {
  ActionType,
  CardLayout,
  CreateWorkflowDefinition,
  DisplayType,
  DisplayVisual,
  IconLibrary,
  InjectWorkflowPlace,
  InputType,
  JsonValue,
  NodeCondition,
  NotificationVariant,
  StickyPosition,
  SubmitWhen,
  TableHeader,
  TimeDisplayOpts,
  TransformOp,
  WorkflowAction,
  WorkflowDisplay,
  WorkflowInput,
  WorkflowNode,
  WorkflowVisual,
} from '@tawks/workflow-types';

export type Dict<T> = Partial<Record<string, T>>;

export type CommandItem = {
  id: string;
  label: string;
  keywords: string[];
  icon: string | null;
  shortcut: string | null;
  action_id: string;
  action_input_map: Dict<string> | null;
  data: Dict<JsonValue>;
};

export type CommandGroup = { heading: string | null; items: CommandItem[] };

export type CommandPaletteConfig = { placeholder: string | null; groups: CommandGroup[] };

export class WorkflowDefinitionBuilder {
  #id: string;
  #name: string;
  #description: string | null = null;
  #startingServerAction: string | null = null;
  #nodes: Dict<WorkflowNode> = {};
  #actions: WorkflowAction[] = [];
  #responses: Dict<JsonValue> = {};
  #context: Dict<JsonValue> = {};
  #multiInstancePerUser = false;

  constructor(id: string, name: string) {
    this.#id = id;
    this.#name = name;
  }

  description(desc: string): this {
    this.#description = desc;
    return this;
  }

  startingServerAction(actionId: string): this {
    this.#startingServerAction = actionId;
    return this;
  }

  context(key: string, value: JsonValue): this {
    this.#context[key] = value;
    return this;
  }

  response(key: string, value: JsonValue): this {
    this.#responses[key] = value;
    return this;
  }

  responses(map: Dict<JsonValue>): this {
    Object.assign(this.#responses, map);
    return this;
  }

  addNode(id: string, node: WorkflowNode): this {
    this.#nodes[id] = node;
    return this;
  }

  addAction(action: WorkflowAction): this {
    this.#actions.push(action);
    return this;
  }

  addActions(actions: Iterable<WorkflowAction>): this {
    this.#actions.push(...actions);
    return this;
  }

  /**
   * Opt this workflow definition into multi-instance-per-user behavior.
   * Default is single-instance: every start reuses the existing non-terminal
   * instance for the same user (or attached to the same parent + inject slot
   * for child workflows).
   */
  multiInstancePerUser(): this {
    this.#multiInstancePerUser = true;
    return this;
  }

  build(): CreateWorkflowDefinition {
    return {
      id: this.#id,
      name: this.#name,
      description: this.#description,
      nodes: { ...this.#nodes },
      responses: { ...this.#responses },
      context: { ...this.#context },
      actions: [...this.#actions],
      starting_server_action: this.#startingServerAction,
      multi_instance_per_user: this.#multiInstancePerUser,
    };
  }
}

export class WorkflowNodeBuilder {
  #title: string;
  #description: string | null = null;
  #visuals: WorkflowVisual[] = [];
  #layout: string | null = null;
  #condition: NodeCondition | null = null;
  #parentId: string | null = null;

  constructor(title: string) {
    this.#title = title;
  }

  description(desc: string): this {
    this.#description = desc;
    return this;
  }

  visual(visual: WorkflowVisual): this {
    this.#visuals.push(visual);
    return this;
  }

  visuals(visuals: Iterable<WorkflowVisual>): this {
    this.#visuals.push(...visuals);
    return this;
  }

  condition(condition: NodeCondition): this {
    this.#condition = condition;
    return this;
  }

  layout(layout: string): this {
    this.#layout = layout;
    return this;
  }

  parentId(parent: string): this {
    this.#parentId = parent;
    return this;
  }

  build(): WorkflowNode {
    return {
      title: this.#title,
      description: this.#description,
      visuals: [...this.#visuals],
      layout: this.#layout,
      condition: this.#condition,
      parent_id: this.#parentId,
    };
  }
}

export class WorkflowVisualBuilder {
  #id: string;
  #display: DisplayVisual | null = null;

  constructor(id: string) {
    this.#id = id;
  }

  display(display: DisplayVisual): this {
    this.#display = display;
    return this;
  }

  animation(
    id: string,
    animationType: string,
    trigger: string | null,
    delayMs: number | null,
    meta: Dict<JsonValue>,
  ): this {
    this.#display = { Animation: { id, animation_type: animationType, trigger, delay_ms: delayMs, meta } };
    return this;
  }

  loading(actionId: string, inputs: Dict<string>): this {
    this.#display = { Loading: { action_id: actionId, inputs } };
    return this;
  }

  notification(variant: NotificationVariant, content: WorkflowDisplay[]): this {
    this.#display = { Notification: { variant, content } };
    return this;
  }

  modal(content: WorkflowDisplay[]): this {
    this.#display = { Modal: { content } };
    return this;
  }

  sticky(position: StickyPosition, content: WorkflowDisplay[]): this {
    this.#display = { Sticky: { position, content } };
    return this;
  }

  tile(content: WorkflowDisplay[]): this {
    this.#display = { Tile: { content } };
    return this;
  }

  block(cls: string, content: WorkflowDisplay[]): this {
    this.#display = { Block: { class: cls, content } };
    return this;
  }

  redirect(urlKey: string): this {
    this.#display = { Redirect: { url_key: urlKey } };
    return this;
  }

  build(): WorkflowVisual {
    if (this.#display === null) throw new Error('Display must be set');
    return { id: this.#id, display: this.#display };
  }

  chat(chatId: string): this {
    this.#display = { Chat: { chat_id: chatId } };
    return this;
  }
}

export class WorkflowDisplayBuilder {
  #class: string;
  #kind: DisplayType | null = null;

  constructor(cls: string) {
    this.#class = cls;
  }

  userAvatar(key: string): this {
    return this.#set({ UserAvatar: { key } });
  }

  commandPalette(key: string): this {
    return this.#set({ CommandPalette: { key } });
  }

  static sectionTitle(title: string): WorkflowDisplay {
    return new WorkflowDisplayBuilder('text-lg font-semibold px-4').text(title).build();
  }

  loader(): this {
    return this.#set('Loader');
  }

  popover(
    openedServerActionId: string | null,
    closedServerActionId: string | null,
    trigger: WorkflowDisplay[],
    content: WorkflowVisual[],
    contentClass: string,
  ): this {
    return this.#set({
      Popover: {
        trigger,
        content,
        opened_server_action_id: openedServerActionId,
        closed_server_action_id: closedServerActionId,
        content_class: contentClass,
      },
    });
  }

  searchable(
    dataUrl: string,
    inputKey: string,
    targetKey: string,
    as: string,
    placeholder: string | null,
    listContent: WorkflowDisplay[],
  ): this {
    return this.#set({
      Searchable: {
        data_url: dataUrl,
        target_key: targetKey,
        as,
        placeholder,
        list_content: listContent,
        input_key: inputKey,
      },
    });
  }

  barChart(chartDataKey: string, config: JsonValue, x: string, y: string, axis: string | null = null): this {
    return this.#set({ Chart: { type: 'Bar', chart_data_key: chartDataKey, config, x, y, axis } });
  }

  areaChart(chartDataKey: string, config: JsonValue, x: string, y: string, axis: string | null = null): this {
    return this.#set({ Chart: { type: 'Area', chart_data_key: chartDataKey, config, x, y, axis } });
  }

  lineChart(chartDataKey: string, config: JsonValue, x: string, y: string, axis: string | null = null): this {
    return this.#set({ Chart: { type: 'Line', chart_data_key: chartDataKey, config, x, y, axis } });
  }

  build(): WorkflowDisplay {
    return { class: this.#class, type: this.#kind ?? { Div: { content: [] } } };
  }

  countdown(until: string, as: string, text: string, shakeUnderMsLeft: number | null = null): this {
    return this.#set({ Countdown: { until, as, text, shake_under_ms_left: shakeUnderMsLeft } });
  }

  tile(workflow: string): this {
    return this.#set({ Tile: { workflow } });
  }

  tooltipButton(
    tooltipContent: WorkflowDisplay[],
    content: WorkflowDisplay[],
    inputMap: Dict<string> | null,
    actionId: string,
    variant: string,
    size: string,
  ): this {
    return this.#set({
      TooltipButton: {
        tooltip_content: tooltipContent,
        content,
        input_map: inputMap,
        action_id: actionId,
        variant,
        size,
      },
    });
  }

  tooltip(tooltipHtml: string, cls: string, viewDisplay: WorkflowDisplay[]): this {
    return this.#set({ Tooltip: { class: cls, tooltip_html: tooltipHtml, view_display: viewDisplay } });
  }

  input(id: string, input: WorkflowInput): this {
    return this.#set({ Input: { id, input } });
  }

  mutlimediaLobby(): this {
    return this.#set('MultimediaLobby');
  }

  condition(condition: NodeCondition, content: WorkflowDisplay[]): this {
    return this.#set({ Condition: { condition, content } });
  }

  submit(
    content: WorkflowDisplay[],
    inputMap: Dict<JsonValue> | null,
    actionId: string,
    variant: string,
    size: string,
  ): this {
    return this.#set({ Button: { content, input_map: inputMap, action_id: actionId, variant, size, type: 'submit' } });
  }

  button(
    content: WorkflowDisplay[],
    inputMap: Dict<JsonValue> | null,
    actionId: string,
    variant: string,
    size: string,
  ): this {
    return this.#set({ Button: { content, input_map: inputMap, action_id: actionId, variant, size, type: 'button' } });
  }

  progress(progressPercentageKey: string, percentageClass: string): this {
    return this.#set({
      Progress: { progress_percentage_key: progressPercentageKey, percentage_class: percentageClass },
    });
  }

  badge(variant: string, text: string): this {
    return this.#set({ Badge: { variant, text } });
  }

  revealCards(revealCardKeys: string[]): this {
    return this.#set({ RevealCards: { reveal_card_keys: revealCardKeys } });
  }

  selectCards(cardIdKeys: string[]): this {
    return this.#set({ SelectCards: { card_id_keys: cardIdKeys } });
  }

  time(key: string, opts: TimeDisplayOpts): this {
    return this.#set({ Time: { opts, key } });
  }

  scoreTicker(
    valueKey: string,
    lucideIcon: string,
    iconClass: string,
    containerClass: string | null = null,
    popTextClass: string | null = null,
    popContainerClass: string | null = null,
  ): this {
    return this.#set({
      ScoreTicker: {
        value_key: valueKey,
        icon: new WorkflowDisplayBuilder(iconClass).lucideIcon(lucideIcon, null).build(),
        container_class: containerClass,
        pop_text_class: popTextClass,
        pop_container_class: popContainerClass,
      },
    });
  }

  form(serverActionId: string, content: WorkflowDisplay[], submitWhen: SubmitWhen): this {
    return this.#set({ Form: { server_action_id: serverActionId, content, submit_when: submitWhen } });
  }

  activity(state: string, action: string): this {
    return this.#set({ Activity: { state, action } });
  }

  battlefieldCanvas(): this {
    return this.#set('BattlefieldCanvas');
  }

  /** The 3D rail map for Tracklines; the client reads the game from `responses.game`. */
  tracklinesCanvas(): this {
    return this.#set('TracklinesCanvas');
  }

  /**
   * The party stage for real-time lobby minigames; the client reads its
   * socket address and join pass from `responses.party`.
   */
  partyCanvas(): this {
    return this.#set('PartyCanvas');
  }

  /** The remote play stage; the client reads its relay address and pass from `responses.remote_play`. */
  remotePlayCanvas(): this {
    return this.#set('RemotePlayCanvas');
  }

  /** NHL feed selector and HLS player; reads `tn_date` or `tg_id`. */
  nhlStreams(): this {
    return this.#set('NhlStreams');
  }

  /**
   * Render an FPS WebTransport client. The four `*Key` arguments name keys in
   * `workflow.responses` from which the client reads its server URL, the match
   * id, its per-player token, and the self-signed cert hash (hex-encoded
   * SHA-256 of the cert DER).
   */
  fpsCanvas(serverUrlKey: string, matchIdKey: string, tokenKey: string, certHashKey: string): this {
    return this.#set({
      FpsCanvas: {
        server_url_key: serverUrlKey,
        match_id_key: matchIdKey,
        token_key: tokenKey,
        cert_hash_key: certHashKey,
      },
    });
  }

  link(href: string, content: WorkflowDisplay[]): this {
    return this.#set({ Link: { href, content } });
  }

  text(text: string): this {
    return this.#set({ Text: { text } });
  }

  lucideIcon(name: string, ariaLabel: string | null = null): this {
    const icon = IconBuilder.lucide(name);
    if (ariaLabel !== null) icon.ariaLabel(ariaLabel);
    return this.#set(icon.intoDisplayType());
  }

  icon(lib: IconLibrary, name: string, ariaLabel: string | null = null, url: string | null = null): this {
    return this.#set({ Icon: { lib, name, aria_label: ariaLabel, url } });
  }

  card(content: WorkflowDisplay[], layout: CardLayout): this {
    return this.#set({ Card: { content, layout } });
  }

  video(src: string, posterSrc: string | null = null): this {
    return this.#set({ Video: { src, poster_src: posterSrc } });
  }

  audioClip(src: string, seconds: string, marks: number[]): this {
    return this.#set({ AudioClip: { src, seconds, marks } });
  }

  dotsBoard(board: string, action: string): this {
    return this.#set({ DotsBoard: { board, action } });
  }

  wordWheel(puzzle: string, action: string): this {
    return this.#set({ WordWheel: { puzzle, action } });
  }

  snakeRun(level: string, action: string): this {
    return this.#set({ SnakeRun: { level, action } });
  }

  shareResult(value: string, label: string): this {
    return this.#set({ ShareResult: { value, label } });
  }

  railyardBoard(board: string, action: string, saveAction: string): this {
    return this.#set({ RailyardBoard: { board, action, save_action: saveAction } });
  }

  image(src: string, altText: string): this {
    return this.#set({ Image: { src, alt_text: altText } });
  }

  list(itemsKey: string, as: string, content: WorkflowDisplay[]): this {
    return this.#set({ List: { items_key: itemsKey, as, content } });
  }

  div(content: WorkflowDisplay[]): this {
    return this.#set({ Div: { content } });
  }

  carousel(itemsKey: string, as: string, content: WorkflowDisplay[]): this {
    return this.#set({
      Carousel: { items_key: itemsKey, as, content, on_select_action: null, input_map: null, active_key: null },
    });
  }

  /** A carousel whose swipe selection runs a workflow action. */
  carouselSelect(
    itemsKey: string,
    as: string,
    content: WorkflowDisplay[],
    actionId: string,
    inputMap: Dict<JsonValue> | null,
    activeKey: string,
  ): this {
    return this.#set({
      Carousel: {
        items_key: itemsKey,
        as,
        content,
        on_select_action: actionId,
        input_map: inputMap,
        active_key: activeKey,
      },
    });
  }

  table(itemsKey: string, as: string, headers: TableHeader[], row: WorkflowDisplay[]): this {
    return this.#set({ Table: { items_key: itemsKey, as, headers, row } });
  }

  wysiyg(contentKey: string): this {
    return this.#set({ Wysiwyg: { content_key: contentKey } });
  }

  #set(kind: DisplayType): this {
    this.#kind = kind;
    return this;
  }
}

export class WorkflowInputBuilder {
  #readonly: NodeCondition | null = null;
  #type: InputType | null = null;
  #defaultValue: JsonValue | null = null;
  #required = false;
  #onChangeAction: [string, Dict<string>] | null = null;

  readonly(cond: NodeCondition): this {
    this.#readonly = cond;
    return this;
  }

  required(required: boolean): this {
    this.#required = required;
    return this;
  }

  defaultValue(value: JsonValue): this {
    this.#defaultValue = value;
    return this;
  }

  onChangeAction(actionId: string, map: Dict<string>): this {
    this.#onChangeAction = [actionId, map];
    return this;
  }

  type(t: InputType): this {
    this.#type = t;
    return this;
  }

  build(): WorkflowInput {
    if (this.#type === null) throw new Error('input type must be set');
    return {
      readonly: this.#readonly,
      type: this.#type,
      default_value: this.#defaultValue,
      required: this.#required,
      on_change_action: this.#onChangeAction,
    };
  }
}

export class InputTypeBuilder {
  static text(placeholder: string | null = null): InputType {
    return { Text: { placeholder } };
  }

  static wysiwyg(): InputType {
    return 'Wysiwyg';
  }

  static select(
    filter: NodeCondition | null,
    itemsKey: string,
    as: string,
    value: string,
    content: WorkflowDisplay[],
    selectedContent: WorkflowDisplay[] | null,
    unselectedContent: WorkflowDisplay[] | null,
    cls: string,
  ): InputType {
    return {
      Select: {
        filter,
        items_key: itemsKey,
        as,
        value,
        content,
        selected_content: selectedContent,
        unselected_content: unselectedContent,
        class: cls,
      },
    };
  }

  static switch(): InputType {
    return 'Switch';
  }

  static checkbox(label: string): InputType {
    return { Checkbox: { label } };
  }

  static number(min: number | null = null, max: number | null = null): InputType {
    return { Number: { min, max } };
  }

  static textArea(placeholder: string | null = null, rows: number | null = null): InputType {
    return { TextArea: { placeholder, rows } };
  }

  static list(fromResponsesId: string, itemKey: string, displayField: string): InputType {
    return { List: { from_responses_id: fromResponsesId, item_key: itemKey, display_field: displayField } };
  }

  static serverActionLoader(target: string): InputType {
    return { ServerActionLoader: { target } };
  }

  static display(content: string): InputType {
    return { Display: { content } };
  }

  static multiSelectFromList(
    itemsKey: string,
    as: string,
    value: string,
    content: WorkflowDisplay[],
    cls: string,
    min: number | null = null,
    max: number | null = null,
  ): InputType {
    return { MultiSelectFromList: { items_key: itemsKey, as, value, content, class: cls, min, max } };
  }

  static selectFromList(
    itemsKey: string,
    as: string,
    value: string,
    cls: string,
    selectedClass: string,
    content: WorkflowDisplay[],
  ): InputType {
    return {
      SelectFromList: { items_key: itemsKey, as, value, class: cls, selected_class: selectedClass, content },
    };
  }
}

export class WorkflowActionBuilder {
  #id: string;
  #progressNode = true;
  #actionType: ActionType | null = null;
  #target: string | null = null;
  #allowWhileLoading = false;

  constructor(id: string) {
    this.#id = id;
  }

  progressNode(progress: boolean): this {
    this.#progressNode = progress;
    return this;
  }

  transformThenRun(
    ops: TransformOp[],
    reconcileNode: boolean,
    serverActionId: string,
    inputMap: Dict<string> | null = null,
  ): this {
    return this.#set({
      TransformThenRun: { ops, reconcile_node: reconcileNode, server_action_id: serverActionId, input_map: inputMap },
    });
  }

  transformResponses(ops: TransformOp[], reconcileNode: boolean): this {
    return this.#set({ TransformResponses: { ops, reconcile_node: reconcileNode } });
  }

  nextNode(): this {
    return this.#set('NextNode');
  }

  previousNode(): this {
    return this.#set('PreviousNode');
  }

  submit(): this {
    return this.#set('Submit');
  }

  noop(): this {
    return this.#set('Noop');
  }

  cancel(): this {
    return this.#set('Cancel');
  }

  runServerAction(): this {
    return this.#set('RunServerAction');
  }

  startInjectedWorkflow(
    inputMap: Dict<string> | null,
    context: Dict<JsonValue>,
    responses: Dict<JsonValue>,
    completeWorkflow: boolean,
    injectPlace: InjectWorkflowPlace,
    onCompleteOps: TransformOp[],
  ): this {
    return this.#set({
      StartWorkflow: {
        input_map: inputMap,
        responses,
        context,
        complete_workflow: completeWorkflow,
        inject_place: injectPlace,
        on_complete_ops: onCompleteOps,
      },
    });
  }

  startWorkflow(
    inputMap: Dict<string> | null,
    context: Dict<JsonValue>,
    responses: Dict<JsonValue>,
    completeWorkflow: boolean,
  ): this {
    return this.#set({
      StartWorkflow: {
        input_map: inputMap,
        responses,
        context,
        complete_workflow: completeWorkflow,
        inject_place: null,
        on_complete_ops: [],
      },
    });
  }

  target(target: string): this {
    this.#target = target;
    return this;
  }

  allowWhileLoading(): this {
    this.#allowWhileLoading = true;
    return this;
  }

  build(): WorkflowAction {
    if (this.#actionType === null) throw new Error('action_type must be set');
    return {
      id: this.#id,
      action_type: this.#actionType,
      allow_while_loading: this.#allowWhileLoading,
      target: this.#target,
      progress_node: this.#progressNode,
    };
  }

  #set(actionType: ActionType): this {
    this.#actionType = actionType;
    return this;
  }
}

export class Cond {
  static gte(field: string, value: JsonValue): NodeCondition {
    return { Gte: { field, value } };
  }

  static always(): NodeCondition {
    return 'Always';
  }

  static not(inner: NodeCondition): NodeCondition {
    return { Not: inner };
  }

  static and(list: NodeCondition[]): NodeCondition {
    return { And: list };
  }

  static or(list: Iterable<NodeCondition>): NodeCondition {
    return { Or: [...list] };
  }

  static someInList(list: string, as: string, condition: NodeCondition): NodeCondition {
    return { SomeInList: { list, as, condition } };
  }

  static responseExists(field: string): NodeCondition {
    return { ResponseExists: field };
  }

  static responseEquals(field: string, value: JsonValue): NodeCondition {
    return { ResponseEquals: { field, value } };
  }

  static responseListEmpty(field: string): NodeCondition {
    return { ResponseListEmpty: field };
  }

  static responseListNotEmpty(field: string): NodeCondition {
    return { Not: { ResponseListEmpty: field } };
  }
}

export const tableHeader = (
  label: string,
  key: string,
  width: string | null = null,
  align: string | null = null,
): TableHeader => ({ label, key, width, align });

export class IconBuilder {
  #lib: IconLibrary;
  #name: string;
  #ariaLabel: string | null = null;
  #url: string | null = null;

  /** Generic constructor (use presets where possible). */
  constructor(lib: IconLibrary, name: string) {
    this.#lib = lib;
    this.#name = name;
  }

  static lucide(name: string): IconBuilder {
    return new IconBuilder('Lucide', name);
  }

  /** URL-backed icon (e.g., remote PNG/SVG). */
  static fromUrl(url: string): IconBuilder {
    const icon = new IconBuilder('Url', '');
    icon.#url = url;
    return icon;
  }

  name(name: string): this {
    this.#name = name;
    return this;
  }

  ariaLabel(aria: string): this {
    this.#ariaLabel = aria;
    return this;
  }

  intoDisplayType(): DisplayType {
    return { Icon: { lib: this.#lib, name: this.#name, aria_label: this.#ariaLabel, url: this.#url } };
  }

  intoWorkflowDisplay(): WorkflowDisplay {
    return { class: '', type: this.intoDisplayType() };
  }

  intoWorkflowDisplayWithClass(cls: string): WorkflowDisplay {
    return { class: cls, type: this.intoDisplayType() };
  }
}

export class CommandPaletteBuilder {
  #placeholder: string | null = null;
  #groups: CommandGroup[] = [];
  #allowSearch = false;

  allowSearch(allow: boolean): this {
    this.#allowSearch = allow;
    return this;
  }

  placeholder(text: string): this {
    this.#placeholder = text;
    return this;
  }

  addGroup(group: CommandGroup): this {
    this.#groups.push(group);
    return this;
  }

  groups(groups: CommandGroup[]): this {
    this.#groups = [...groups];
    return this;
  }

  build(): CommandPaletteConfig {
    return { placeholder: this.#placeholder, groups: [...this.#groups] };
  }
}

export class CommandGroupBuilder {
  #heading: string | null = null;
  #items: CommandItem[] = [];

  heading(text: string): this {
    this.#heading = text;
    return this;
  }

  addItem(item: CommandItem): this {
    this.#items.push(item);
    return this;
  }

  items(items: CommandItem[]): this {
    this.#items = [...items];
    return this;
  }

  build(): CommandGroup {
    return { heading: this.#heading, items: [...this.#items] };
  }
}

export class CommandItemBuilder {
  #id: string;
  #label: string;
  #keywords: string[] = [];
  #icon: string | null = null;
  #shortcut: string | null = null;
  #action: [string, Dict<string> | null] | null = null;
  #data: Dict<JsonValue> = {};

  constructor(id: string, label: string) {
    this.#id = id;
    this.#label = label;
  }

  keyword(kw: string): this {
    this.#keywords.push(kw);
    return this;
  }

  keywords(kws: string[]): this {
    this.#keywords = [...kws];
    return this;
  }

  icon(icon: string): this {
    this.#icon = icon;
    return this;
  }

  shortcut(shortcut: string): this {
    this.#shortcut = shortcut;
    return this;
  }

  action(actionId: string, inputMap: Dict<string> | null = null): this {
    this.#action = [actionId, inputMap];
    return this;
  }

  build(): CommandItem {
    if (this.#action === null) throw new Error('CommandItem must have an action');
    return {
      id: this.#id,
      label: this.#label,
      keywords: [...this.#keywords],
      icon: this.#icon,
      shortcut: this.#shortcut,
      action_id: this.#action[0],
      action_input_map: this.#action[1],
      data: { ...this.#data },
    };
  }

  withData(key: string, val: JsonValue): this {
    this.#data[key] = val;
    return this;
  }
}

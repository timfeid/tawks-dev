import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  Cond,
  CommandGroupBuilder,
  CommandItemBuilder,
  CommandPaletteBuilder,
  IconBuilder,
  InputTypeBuilder,
  WorkflowActionBuilder,
  WorkflowDefinitionBuilder,
  WorkflowDisplayBuilder,
  WorkflowInputBuilder,
  WorkflowNodeBuilder,
  WorkflowVisualBuilder,
  tableHeader,
} from '../dist/index.js';

// Serialized by the Rust builder (workflow_model::builder) from the same chains below.
const rust: Record<string, string> = {
  definition: '{"id":"hello","name":"Hello","description":"A greeting","nodes":{"home":{"title":"Home","description":"Start","visuals":[{"id":"page","display":{"Block":{"class":"p-4","content":[{"class":"text-lg font-semibold px-4","type":{"Text":{"text":"Hi"}}},{"class":"flex","type":{"Div":{"content":[{"class":"","type":{"Text":{"text":"a"}}},{"class":"","type":{"Text":{"text":"b"}}}]}}}]}}}],"layout":"wide","condition":"Always","parent_id":null},"done":{"title":"Done","description":null,"visuals":[{"id":"n","display":{"Notification":{"variant":"Success","content":[{"class":"","type":{"Text":{"text":"ok"}}}]}}},{"id":"c","display":{"Chat":{"chat_id":"chat-1"}}}],"layout":null,"condition":null,"parent_id":"home"}},"responses":{"count":3,"name":null},"context":{"niche":"dev"},"actions":[{"id":"go","action_type":"RunServerAction","allow_while_loading":false,"target":"sa-hello.go","progress_node":true},{"id":"next","action_type":"NextNode","allow_while_loading":true,"target":null,"progress_node":false}],"starting_server_action":"sa-hello.load","multi_instance_per_user":true}',
  definition_default: '{"id":"a","name":"A","description":null,"nodes":{},"responses":{},"context":{},"actions":[],"starting_server_action":null,"multi_instance_per_user":false}',
  visual_animation: '{"id":"v","display":{"Animation":{"id":"a1","animation_type":"confetti","trigger":"load","delay_ms":250,"meta":{"n":5}}}}',
  visual_loading: '{"id":"v","display":{"Loading":{"action_id":"sa","inputs":{"x":"{y}"}}}}',
  visual_sticky: '{"id":"v","display":{"Sticky":{"position":"Toolbar","content":[]}}}',
  visual_misc: '[{"id":"m","display":{"Modal":{"content":[{"class":"","type":{"Text":{"text":"m"}}}]}}},{"id":"t","display":{"Tile":{"content":[]}}},{"id":"r","display":{"Redirect":{"url_key":"url"}}}]',
  displays: '[{"class":"x","type":{"Div":{"content":[]}}},{"class":"st","type":{"ScoreTicker":{"value_key":"score","icon":{"class":"size-4","type":{"Icon":{"lib":"Lucide","name":"star","aria_label":null,"url":null}}},"container_class":"c","pop_text_class":null,"pop_container_class":"pc"}}},{"class":"","type":{"Carousel":{"items_key":"items","as":"item","content":[{"class":"","type":{"Text":{"text":"{item.name}"}}}],"on_select_action":null,"input_map":null,"active_key":null}}},{"class":"","type":{"Carousel":{"items_key":"items","as":"item","content":[],"on_select_action":"pick","input_map":{"id":"{item.id}"},"active_key":"active"}}},{"class":"","type":{"Popover":{"trigger":[{"class":"","type":{"Text":{"text":"t"}}}],"content":[{"id":"p","display":{"Modal":{"content":[]}}}],"opened_server_action_id":"open","closed_server_action_id":null,"content_class":"w-64"}}},{"class":"","type":{"Searchable":{"data_url":"/api","target_key":"pick","as":"row","placeholder":"Search","list_content":[{"class":"","type":{"Text":{"text":"{row}"}}}],"input_key":"q"}}},{"class":"","type":{"Chart":{"type":"Bar","chart_data_key":"data","config":{"k":{"label":"K"}},"x":"day","y":"k","axis":null}}},{"class":"","type":{"Chart":{"type":"Line","chart_data_key":"data","config":[],"x":"x","y":"y","axis":"left"}}},{"class":"","type":{"Chart":{"type":"Area","chart_data_key":"data","config":null,"x":"x","y":"y","axis":null}}},{"class":"","type":{"Countdown":{"until":"ends","as":"left","text":"{left}s","shake_under_ms_left":5000}}},{"class":"","type":{"TooltipButton":{"tooltip_content":[{"class":"","type":{"Text":{"text":"tip"}}}],"content":[{"class":"","type":{"Text":{"text":"b"}}}],"input_map":null,"action_id":"act","variant":"ghost","size":"sm"}}},{"class":"","type":{"Tooltip":{"class":"tc","tooltip_html":"<b>x</b>","view_display":[]}}},{"class":"","type":{"Button":{"content":[{"class":"","type":{"Text":{"text":"Go"}}}],"input_map":null,"action_id":"go","variant":"default","size":"lg","type":"submit"}}},{"class":"","type":{"Button":{"content":[{"class":"","type":{"Text":{"text":"Go"}}}],"input_map":{"a":1},"action_id":"go","variant":"outline","size":"default","type":"button"}}},{"class":"","type":{"Icon":{"lib":"Lucide","name":"heart","aria_label":"Like","url":null}}},{"class":"","type":{"Icon":{"lib":"Url","name":"","aria_label":null,"url":"https://x/y.png"}}},{"class":"","type":{"Video":{"src":"v.mp4","poster_src":null}}},{"class":"","type":{"AudioClip":{"src":"a.mp3","seconds":"{secs}","marks":[0.5,2.25]}}},{"class":"","type":{"Table":{"items_key":"rows","as":"r","headers":[{"label":"Name","key":"name","width":"10rem","align":null}],"row":[{"class":"","type":{"Text":{"text":"{r.name}"}}}]}}},{"class":"","type":{"Time":{"opts":{"timeago":true},"key":"at"}}},{"class":"","type":{"Time":{"opts":{"timeago":false,"format":"PP","showTimeForToday":true,"timeForTodayFormat":null},"key":"at"}}},{"class":"","type":{"Time":{"opts":{"timeago":null,"format":null,"showTimeForToday":null,"timeForTodayFormat":null},"key":"at"}}},{"class":"","type":{"Form":{"server_action_id":"save","content":[],"submit_when":"OnChange"}}},{"class":"","type":{"Card":{"content":[],"layout":{"Image":{"image_key":"img"}}}}},{"class":"","type":{"Card":{"content":[],"layout":"Vertical"}}},{"class":"","type":"MultimediaLobby"},{"class":"","type":"Loader"},{"class":"","type":{"Wysiwyg":{"content_key":"body"}}},{"class":"","type":{"FpsCanvas":{"server_url_key":"u","match_id_key":"m","token_key":"t","cert_hash_key":"c"}}},{"class":"","type":{"RailyardBoard":{"board":"board","action":"run","save_action":"save"}}},{"class":"","type":{"Condition":{"condition":{"ResponseExists":"x"},"content":[{"class":"","type":{"Text":{"text":"x"}}}]}}},{"class":"","type":{"Progress":{"progress_percentage_key":"pct","percentage_class":"bg-green"}}},{"class":"","type":{"RevealCards":{"reveal_card_keys":["a"]}}},{"class":"","type":{"UserAvatar":{"key":"uid"}}},{"class":"","type":"PartyCanvas"}]',
  inputs: '[{"readonly":null,"type":{"Text":{"placeholder":null}},"default_value":null,"required":false,"on_change_action":null},{"readonly":{"Not":"Always"},"type":{"Select":{"filter":"Always","items_key":"opts","as":"o","value":"{o.id}","content":[{"class":"","type":{"Text":{"text":"{o.label}"}}}],"selected_content":null,"unselected_content":[],"class":"grid"}},"default_value":"a","required":true,"on_change_action":["changed",{"v":"{v}"}]},{"readonly":null,"type":{"Number":{"min":1,"max":null}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"TextArea":{"placeholder":"Say","rows":4}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"MultiSelectFromList":{"items_key":"i","as":"a","value":"v","content":[],"class":"c","min":null,"max":3}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"SelectFromList":{"items_key":"i","as":"a","value":"v","class":"c","selected_class":"sc","content":[]}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":"Switch","default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":"Wysiwyg","default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"List":{"from_responses_id":"r","item_key":"k","display_field":"f"}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"Checkbox":{"label":"ok"}},"default_value":null,"required":false,"on_change_action":null}]',
  input_display: '{"class":"w","type":{"Input":{"id":"name","input":{"readonly":null,"type":{"Text":{"placeholder":null}},"default_value":null,"required":false,"on_change_action":null}}}}',
  actions: '[{"id":"t","action_type":{"TransformThenRun":{"ops":[{"Set":{"path":"responses.x","value":1}}],"reconcile_node":true,"server_action_id":"sa","input_map":{"a":"{b}"}}},"allow_while_loading":false,"target":null,"progress_node":true},{"id":"r","action_type":{"TransformResponses":{"ops":[{"Set":{"path":"responses.x","value":1}}],"reconcile_node":false}},"allow_while_loading":false,"target":null,"progress_node":true},{"id":"s","action_type":{"StartWorkflow":{"input_map":null,"responses":{},"context":{"c":true},"complete_workflow":true,"inject_place":null,"on_complete_ops":[]}},"allow_while_loading":false,"target":null,"progress_node":true},{"id":"i","action_type":{"StartWorkflow":{"input_map":null,"responses":{},"context":{},"complete_workflow":false,"inject_place":{"Modal":{"title":"T"}},"on_complete_ops":[{"Set":{"path":"responses.x","value":1}}]}},"allow_while_loading":false,"target":null,"progress_node":true},{"id":"p","action_type":"PreviousNode","allow_while_loading":false,"target":null,"progress_node":true},{"id":"n","action_type":"Noop","allow_while_loading":false,"target":null,"progress_node":true},{"id":"c","action_type":"Cancel","allow_while_loading":false,"target":null,"progress_node":true},{"id":"u","action_type":"Submit","allow_while_loading":false,"target":null,"progress_node":true}]',
  conditions: '[{"Gte":{"field":"score","value":10}},{"And":["Always",{"ResponseExists":"a"}]},{"Or":[{"ResponseEquals":{"field":"a","value":{"b":[1,"c"]}}}]},{"SomeInList":{"list":"players","as":"p","condition":{"ResponseEquals":{"field":"p.ready","value":true}}}},{"ResponseListEmpty":"l"},{"Not":{"ResponseListEmpty":"l"}}]',
  palette: '{"placeholder":"Type","groups":[{"heading":"Files","items":[{"id":"open","label":"Open","keywords":["o","p"],"icon":"folder","shortcut":"cmd+o","action_id":"open","action_input_map":null,"data":{"n":1}}]},{"heading":null,"items":[{"id":"x","label":"X","keywords":["k"],"icon":null,"shortcut":null,"action_id":"x","action_input_map":{"a":"b"},"data":{}}]}]}',
  icons: '[{"class":"","type":{"Icon":{"lib":"Url","name":"","aria_label":"X","url":"https://i/x.svg"}}},{"class":"size-4","type":{"Icon":{"lib":"Lucide","name":"b","aria_label":null,"url":null}}},{"class":"","type":{"Icon":{"lib":"Emoji","name":"smile","aria_label":null,"url":null}}}]',
};

const sameAsRust = (name: string, value: unknown) => assert.equal(JSON.stringify(value), rust[name], name);
const text = (t: string) => new WorkflowDisplayBuilder('').text(t).build();

test('definition, node and visual builders match Rust', () => {
  const def = new WorkflowDefinitionBuilder('hello', 'Hello')
    .description('A greeting')
    .startingServerAction('sa-hello.load')
    .context('niche', 'dev')
    .response('count', 3)
    .responses({ name: null })
    .addNode(
      'home',
      new WorkflowNodeBuilder('Home')
        .description('Start')
        .layout('wide')
        .condition(Cond.always())
        .visual(
          new WorkflowVisualBuilder('page')
            .block('p-4', [
              WorkflowDisplayBuilder.sectionTitle('Hi'),
              new WorkflowDisplayBuilder('flex').div([text('a'), text('b')]).build(),
            ])
            .build(),
        )
        .build(),
    )
    .addNode(
      'done',
      new WorkflowNodeBuilder('Done')
        .parentId('home')
        .visuals([
          new WorkflowVisualBuilder('n').notification('Success', [text('ok')]).build(),
          new WorkflowVisualBuilder('c').chat('chat-1').build(),
        ])
        .build(),
    )
    .addAction(new WorkflowActionBuilder('go').runServerAction().target('sa-hello.go').build())
    .addActions([new WorkflowActionBuilder('next').nextNode().progressNode(false).allowWhileLoading().build()])
    .multiInstancePerUser()
    .build();
  assert.deepEqual(def, JSON.parse(rust.definition));
  sameAsRust('definition_default', new WorkflowDefinitionBuilder('a', 'A').build());

  sameAsRust('visual_animation', new WorkflowVisualBuilder('v').animation('a1', 'confetti', 'load', 250, { n: 5 }).build());
  sameAsRust('visual_loading', new WorkflowVisualBuilder('v').loading('sa', { x: '{y}' }).build());
  sameAsRust('visual_sticky', new WorkflowVisualBuilder('v').sticky('Toolbar', []).build());
  sameAsRust('visual_misc', [
    new WorkflowVisualBuilder('m').modal([text('m')]).build(),
    new WorkflowVisualBuilder('t').tile([]).build(),
    new WorkflowVisualBuilder('r').redirect('url').build(),
  ]);
});

test('display variants match Rust', () => {
  const d = () => new WorkflowDisplayBuilder('');
  sameAsRust('displays', [
    new WorkflowDisplayBuilder('x'),
    new WorkflowDisplayBuilder('st').scoreTicker('score', 'star', 'size-4', 'c', null, 'pc'),
    d().carousel('items', 'item', [text('{item.name}')]),
    d().carouselSelect('items', 'item', [], 'pick', { id: '{item.id}' }, 'active'),
    d().popover('open', null, [text('t')], [new WorkflowVisualBuilder('p').modal([]).build()], 'w-64'),
    d().searchable('/api', 'q', 'pick', 'row', 'Search', [text('{row}')]),
    d().barChart('data', { k: { label: 'K' } }, 'day', 'k', null),
    d().lineChart('data', [], 'x', 'y', 'left'),
    d().areaChart('data', null, 'x', 'y'),
    d().countdown('ends', 'left', '{left}s', 5000),
    d().tooltipButton([text('tip')], [text('b')], null, 'act', 'ghost', 'sm'),
    d().tooltip('<b>x</b>', 'tc', []),
    d().submit([text('Go')], null, 'go', 'default', 'lg'),
    d().button([text('Go')], { a: 1 }, 'go', 'outline', 'default'),
    d().lucideIcon('heart', 'Like'),
    d().icon('Url', '', null, 'https://x/y.png'),
    d().video('v.mp4'),
    d().audioClip('a.mp3', '{secs}', [0.5, 2.25]),
    d().table('rows', 'r', [tableHeader('Name', 'name', '10rem', null)], [text('{r.name}')]),
    d().time('at', { timeago: true }),
    d().time('at', { timeago: false, format: 'PP', showTimeForToday: true, timeForTodayFormat: null }),
    d().time('at', { timeago: null, format: null, showTimeForToday: null, timeForTodayFormat: null }),
    d().form('save', [], 'OnChange'),
    d().card([], { Image: { image_key: 'img' } }),
    d().card([], 'Vertical'),
    d().mutlimediaLobby(),
    d().loader(),
    d().wysiyg('body'),
    d().fpsCanvas('u', 'm', 't', 'c'),
    d().railyardBoard('board', 'run', 'save'),
    d().condition(Cond.responseExists('x'), [text('x')]),
    d().progress('pct', 'bg-green'),
    d().revealCards(['a']),
    d().userAvatar('uid'),
    d().partyCanvas(),
  ].map((b) => b.build()));
});

test('display variants without a Rust fixture', () => {
  const d = () => new WorkflowDisplayBuilder('c');
  assert.deepEqual(WorkflowDisplayBuilder.sectionTitle('Hi'), { class: 'text-lg font-semibold px-4', type: { Text: { text: 'Hi' } } });
  assert.deepEqual(d().image('a.png', 'A').build(), { class: 'c', type: { Image: { src: 'a.png', alt_text: 'A' } } });
  assert.deepEqual(d().list('xs', 'x', []).build(), { class: 'c', type: { List: { items_key: 'xs', as: 'x', content: [] } } });
  assert.deepEqual(d().badge('secondary', 'New').build(), { class: 'c', type: { Badge: { variant: 'secondary', text: 'New' } } });
  assert.deepEqual(d().link('/x', []).build(), { class: 'c', type: { Link: { href: '/x', content: [] } } });
  assert.deepEqual(d().tile('wf').build().type, { Tile: { workflow: 'wf' } });
  assert.deepEqual(d().selectCards(['k']).build().type, { SelectCards: { card_id_keys: ['k'] } });
  assert.deepEqual(d().activity('state', 'move').build().type, { Activity: { state: 'state', action: 'move' } });
  assert.deepEqual(d().dotsBoard('b', 'a').build().type, { DotsBoard: { board: 'b', action: 'a' } });
  assert.deepEqual(d().wordWheel('p', 'a').build().type, { WordWheel: { puzzle: 'p', action: 'a' } });
  assert.deepEqual(d().snakeRun('l', 'a').build().type, { SnakeRun: { level: 'l', action: 'a' } });
  assert.deepEqual(d().shareResult('v', 'Share').build().type, { ShareResult: { value: 'v', label: 'Share' } });
  assert.deepEqual(d().commandPalette('cmds').build().type, { CommandPalette: { key: 'cmds' } });
  assert.deepEqual(
    ['battlefieldCanvas', 'tracklinesCanvas', 'remotePlayCanvas', 'nhlStreams'].map((m) => (d() as any)[m]().build().type),
    ['BattlefieldCanvas', 'TracklinesCanvas', 'RemotePlayCanvas', 'NhlStreams'],
  );
});

test('input builders match Rust', () => {
  const input = (t: unknown) => new WorkflowInputBuilder().type(t as any).build();
  const inputs = [
    input(InputTypeBuilder.text(null)),
    new WorkflowInputBuilder()
      .type(InputTypeBuilder.select(Cond.always(), 'opts', 'o', '{o.id}', [text('{o.label}')], null, [], 'grid'))
      .required(true)
      .defaultValue('a')
      .readonly(Cond.not(Cond.always()))
      .onChangeAction('changed', { v: '{v}' })
      .build(),
    input(InputTypeBuilder.number(1, null)),
    input(InputTypeBuilder.textArea('Say', 4)),
    input(InputTypeBuilder.multiSelectFromList('i', 'a', 'v', [], 'c', null, 3)),
    input(InputTypeBuilder.selectFromList('i', 'a', 'v', 'c', 'sc', [])),
    input(InputTypeBuilder.switch()),
    input(InputTypeBuilder.wysiwyg()),
    input(InputTypeBuilder.list('r', 'k', 'f')),
    input(InputTypeBuilder.checkbox('ok')),
  ];
  sameAsRust('inputs', inputs);
  sameAsRust('input_display', new WorkflowDisplayBuilder('w').input('name', inputs[0]).build());
  assert.deepEqual(InputTypeBuilder.serverActionLoader('t'), { ServerActionLoader: { target: 't' } });
  assert.deepEqual(InputTypeBuilder.display('hi'), { Display: { content: 'hi' } });
  assert.throws(() => new WorkflowInputBuilder().build(), /input type must be set/);
});

test('action builders match Rust', () => {
  const op = { Set: { path: 'responses.x', value: 1 } };
  sameAsRust('actions', [
    new WorkflowActionBuilder('t').transformThenRun([op], true, 'sa', { a: '{b}' }).build(),
    new WorkflowActionBuilder('r').transformResponses([op], false).build(),
    new WorkflowActionBuilder('s').startWorkflow(null, { c: true }, {}, true).build(),
    new WorkflowActionBuilder('i').startInjectedWorkflow(null, {}, {}, false, { Modal: { title: 'T' } }, [op]).build(),
    new WorkflowActionBuilder('p').previousNode().build(),
    new WorkflowActionBuilder('n').noop().build(),
    new WorkflowActionBuilder('c').cancel().build(),
    new WorkflowActionBuilder('u').submit().build(),
  ]);
  assert.throws(() => new WorkflowActionBuilder('x').build(), /action_type must be set/);
  assert.throws(() => new WorkflowVisualBuilder('x').build(), /Display must be set/);
});

test('conditions match Rust', () => {
  sameAsRust('conditions', [
    Cond.gte('score', 10),
    Cond.and([Cond.always(), Cond.responseExists('a')]),
    Cond.or([Cond.responseEquals('a', { b: [1, 'c'] })]),
    Cond.someInList('players', 'p', Cond.responseEquals('p.ready', true)),
    Cond.responseListEmpty('l'),
    Cond.responseListNotEmpty('l'),
  ]);
});

test('command palette and icon builders match Rust', () => {
  const item = new CommandItemBuilder('open', 'Open').keyword('o').keyword('p').icon('folder').shortcut('cmd+o').action('open').withData('n', 1).build();
  const item2 = new CommandItemBuilder('x', 'X').keywords(['k']).action('x', { a: 'b' }).build();
  const group = new CommandGroupBuilder().heading('Files').addItem(item).build();
  sameAsRust(
    'palette',
    new CommandPaletteBuilder().allowSearch(true).placeholder('Type').addGroup(group).addGroup(new CommandGroupBuilder().items([item2]).build()).build(),
  );
  assert.throws(() => new CommandItemBuilder('a', 'A').build(), /must have an action/);
  sameAsRust('icons', [
    IconBuilder.fromUrl('https://i/x.svg').ariaLabel('X').intoWorkflowDisplay(),
    IconBuilder.lucide('a').name('b').intoWorkflowDisplayWithClass('size-4'),
    new IconBuilder('Emoji', 'smile').intoWorkflowDisplay(),
  ]);
});

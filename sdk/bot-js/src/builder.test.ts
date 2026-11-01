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
  ui,
} from '../dist/index.js';

// Serialized by the Rust builders (workflow_model::builder and workflow_model::ui) from the same chains below.
const rust: Record<string, string> = {
  definition: '{"id":"hello","name":"Hello","description":"A greeting","nodes":{"home":{"title":"Home","description":"Start","visuals":[{"id":"page","display":{"Block":{"content":[{"type":{"Text":{"text":"Hi"}}},{"type":{"Div":{"content":[{"type":{"Text":{"text":"a"}}},{"type":{"Text":{"text":"b"}}}]}}}]}}}],"condition":"Always","parent_id":null},"done":{"title":"Done","description":null,"visuals":[{"id":"n","display":{"Notification":{"variant":"Success","content":[{"type":{"Text":{"text":"ok"}}}]}}},{"id":"c","display":{"Chat":{"chat_id":"chat-1"}}}],"condition":null,"parent_id":"home"}},"responses":{"count":3,"name":null},"context":{"niche":"dev"},"actions":[{"id":"go","action_type":"RunServerAction","allow_while_loading":false,"target":"sa-hello.go","progress_node":true},{"id":"next","action_type":"NextNode","allow_while_loading":true,"target":null,"progress_node":false}],"starting_server_action":"sa-hello.load","multi_instance_per_user":true}',
  definition_default: '{"id":"a","name":"A","description":null,"nodes":{},"responses":{},"context":{},"actions":[],"starting_server_action":null,"multi_instance_per_user":false}',
  visual_animation: '{"id":"v","display":{"Animation":{"id":"a1","animation_type":"confetti","trigger":"load","delay_ms":250,"meta":{"n":5}}}}',
  visual_loading: '{"id":"v","display":{"Loading":{"action_id":"sa","inputs":{"x":"{y}"}}}}',
  visual_sticky: '{"id":"v","display":{"Sticky":{"position":"Toolbar","content":[]}}}',
  visual_misc: '[{"id":"m","display":{"Modal":{"content":[{"type":{"Text":{"text":"m"}}}]}}},{"id":"t","display":{"Tile":{"content":[]}}},{"id":"r","display":{"Redirect":{"url_key":"url"}}},{"id":"p","display":{"Page":{"width":"Wide","content":[{"type":{"Text":{"text":"p"}}}],"center":true,"backdrop":"Dark","typeface":"Spooky"}}}]',
  displays: '[{"type":{"Div":{"content":[]}}},{"type":{"ScoreTicker":{"value_key":"score","icon":{"type":{"Icon":{"lib":"Lucide","name":"star","aria_label":null,"url":null,"size":"Inherit","tone":""}}}}}},{"type":{"Carousel":{"items_key":"items","as":"item","content":[{"type":{"Text":{"text":"{item.name}"}}}],"on_select_action":null,"input_map":null,"active_key":null,"item_size":"Medium"}}},{"type":{"Carousel":{"items_key":"items","as":"item","content":[],"on_select_action":"pick","input_map":{"id":"{item.id}"},"active_key":"active","item_size":"Medium"}}},{"type":{"Popover":{"trigger":[{"type":{"Text":{"text":"t"}}}],"content":[{"id":"p","display":{"Modal":{"content":[]}}}],"opened_server_action_id":"open","closed_server_action_id":null,"width":"Medium"}}},{"type":{"Searchable":{"data_url":"/api","target_key":"pick","as":"row","placeholder":"Search","list_content":[{"type":{"Text":{"text":"{row}"}}}],"input_key":"q"}}},{"type":{"Chart":{"type":"Bar","chart_data_key":"data","config":{"k":{"label":"K"}},"x":"day","y":"k","axis":null}}},{"type":{"Chart":{"type":"Line","chart_data_key":"data","config":[],"x":"x","y":"y","axis":"left"}}},{"type":{"Chart":{"type":"Area","chart_data_key":"data","config":null,"x":"x","y":"y","axis":null}}},{"type":{"Countdown":{"until":"ends","as":"left","text":"{left}s","shake_under_ms_left":5000,"role":"Body","tone":""}}},{"type":{"TooltipButton":{"tooltip_content":[{"type":{"Text":{"text":"tip"}}}],"content":[{"type":{"Text":{"text":"b"}}}],"input_map":null,"action_id":"act","variant":"ghost","size":"sm"}}},{"type":{"Tooltip":{"tooltip_html":"<b>x</b>","view_display":[]}}},{"type":{"Button":{"content":[{"type":{"Text":{"text":"Go"}}}],"input_map":null,"action_id":"go","variant":"default","size":"lg","type":"submit"}}},{"type":{"Button":{"content":[{"type":{"Text":{"text":"Go"}}}],"input_map":{"a":1},"action_id":"go","variant":"outline","size":"default","type":"button"}}},{"type":{"Icon":{"lib":"Lucide","name":"heart","aria_label":"Like","url":null,"size":"Inherit","tone":""}}},{"type":{"Icon":{"lib":"Url","name":"","aria_label":null,"url":"https://x/y.png","size":"Inherit","tone":""}}},{"type":{"Video":{"src":"v.mp4","poster_src":null}}},{"type":{"AudioClip":{"src":"a.mp3","seconds":"{secs}","marks":[0.5,2.25]}}},{"type":{"Table":{"items_key":"rows","as":"r","headers":[{"label":"Name","key":"name","width":"10rem","align":null,"numeric":false,"wide_only":false}],"row":[{"type":{"Text":{"text":"{r.name}"}}}],"row_state":null}}},{"type":{"Time":{"opts":{"timeago":true},"key":"at","role":"Body","tone":""}}},{"type":{"Form":{"server_action_id":"save","content":[],"submit_when":"OnChange"}}},{"type":{"Card":{"content":[],"layout":{"Image":{"image_key":"img"}}}}},{"type":{"Card":{"content":[],"layout":"Vertical"}}},{"type":"MultimediaLobby"},{"type":"Loader"},{"type":{"Wysiwyg":{"content_key":"body"}}},{"type":{"RailyardBoard":{"board":"board","action":"run","save_action":"save"}}},{"type":{"Condition":{"condition":{"ResponseExists":"x"},"content":[{"type":{"Text":{"text":"x"}}}]}}},{"type":{"Progress":{"progress_percentage_key":"pct","tone":""}}},{"type":{"Badge":{"variant":"secondary","text":"New","tone":"","icon":null}}},{"type":{"Link":{"href":"/x","content":[],"style":"Plain"}}},{"type":{"RevealCards":{"reveal_card_keys":["a"]}}},{"type":{"UserAvatar":{"key":"uid"}}},{"type":"PartyCanvas"},{"type":"GameInvite"}]',
  inputs: '[{"readonly":null,"type":{"Text":{"placeholder":null}},"default_value":null,"required":false,"on_change_action":null},{"readonly":{"Not":"Always"},"type":{"Select":{"filter":"Always","items_key":"opts","as":"o","value":"{o.id}","content":[{"type":{"Text":{"text":"{o.label}"}}}],"selected_content":null,"unselected_content":[]}},"default_value":"a","required":true,"on_change_action":["changed",{"v":"{v}"}]},{"readonly":null,"type":{"Number":{"min":1,"max":null}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"TextArea":{"placeholder":"Say","rows":4}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"MultiSelectFromList":{"items_key":"i","as":"a","value":"v","content":[],"columns":2,"min":null,"max":3}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"SelectFromList":{"items_key":"i","as":"a","value":"v","content":[],"columns":3}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":"Switch","default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":"Wysiwyg","default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"List":{"from_responses_id":"r","item_key":"k","display_field":"f"}},"default_value":null,"required":false,"on_change_action":null},{"readonly":null,"type":{"Checkbox":{"label":"ok"}},"default_value":null,"required":false,"on_change_action":null}]',
  input_display: '{"type":{"Input":{"id":"name","input":{"readonly":null,"type":{"Text":{"placeholder":null}},"default_value":null,"required":false,"on_change_action":null}}}}',
  actions: '[{"id":"t","action_type":{"TransformThenRun":{"ops":[{"Set":{"path":"responses.x","value":1}}],"reconcile_node":true,"server_action_id":"sa","input_map":{"a":"{b}"}}},"allow_while_loading":false,"target":null,"progress_node":true},{"id":"r","action_type":{"TransformResponses":{"ops":[{"Set":{"path":"responses.x","value":1}}],"reconcile_node":false}},"allow_while_loading":false,"target":null,"progress_node":true},{"id":"s","action_type":{"StartWorkflow":{"input_map":null,"responses":{},"context":{"c":true},"complete_workflow":true,"inject_place":null,"on_complete_ops":[]}},"allow_while_loading":false,"target":null,"progress_node":true},{"id":"i","action_type":{"StartWorkflow":{"input_map":null,"responses":{},"context":{},"complete_workflow":false,"inject_place":{"Modal":{"title":"T"}},"on_complete_ops":[{"Set":{"path":"responses.x","value":1}}]}},"allow_while_loading":false,"target":null,"progress_node":true},{"id":"p","action_type":"PreviousNode","allow_while_loading":false,"target":null,"progress_node":true},{"id":"n","action_type":"Noop","allow_while_loading":false,"target":null,"progress_node":true},{"id":"c","action_type":"Cancel","allow_while_loading":false,"target":null,"progress_node":true},{"id":"u","action_type":"Submit","allow_while_loading":false,"target":null,"progress_node":true}]',
  conditions: '[{"Gte":{"field":"score","value":10}},{"And":["Always",{"ResponseExists":"a"}]},{"Or":[{"ResponseEquals":{"field":"a","value":{"b":[1,"c"]}}}]},{"SomeInList":{"list":"players","as":"p","condition":{"ResponseEquals":{"field":"p.ready","value":true}}}},{"ResponseListEmpty":"l"},{"Not":{"ResponseListEmpty":"l"}}]',
  palette: '{"placeholder":"Type","groups":[{"heading":"Files","items":[{"id":"open","label":"Open","keywords":["o","p"],"icon":"folder","shortcut":"cmd+o","action_id":"open","action_input_map":null,"data":{"n":1}}]},{"heading":null,"items":[{"id":"x","label":"X","keywords":["k"],"icon":null,"shortcut":null,"action_id":"x","action_input_map":{"a":"b"},"data":{}}]}]}',
  icons: '[{"type":{"Icon":{"lib":"Url","name":"","aria_label":"X","url":"https://i/x.svg","size":"Inherit","tone":""}}},{"type":{"Icon":{"lib":"Lucide","name":"b","aria_label":null,"url":null,"size":"Inherit","tone":""}}},{"type":{"Icon":{"lib":"Emoji","name":"smile","aria_label":null,"url":null,"size":"Inherit","tone":""}}}]',
  ui: '[{"type":{"Hero":{"eyebrow":"New","title":"Back online","lead":"Play now","aside":[{"type":{"Chip":{"icon":"flask-conical","text":"Beta"}}}],"actions":[{"type":{"Link":{"href":"/download","content":[{"type":{"Typography":{"text":"Get the app","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"style":"Primary"}}},{"type":{"Link":{"href":"/docs","content":[{"type":{"Typography":{"text":"Docs","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"style":"Secondary"}}}]}}},{"type":{"Section":{"eyebrow":null,"title":"Improved","lead":"Lots","aside":[],"content":[{"type":{"Grid":{"columns":2,"gap":"Tight","content":[{"type":{"Feature":{"icon":"users","title":"Rosters","body":"All 32 teams.","href":"/teams"}}}],"fixed":true}}}]}}},{"type":{"Typography":{"text":"D","role":"Display","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}},{"type":{"Typography":{"text":"T","role":"Title","tone":"Primary","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}},{"type":{"Typography":{"text":"H","role":"Heading","tone":"","style":{"numeric":false,"clamp":0,"align":"Center","struck":false,"lines":false,"themed":false}}}},{"type":{"Typography":{"text":"S","role":"Subheading","tone":"","style":{"numeric":false,"clamp":2,"align":"Start","struck":false,"lines":false,"themed":false}}}},{"type":{"Typography":{"text":"L","role":"Lead","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}},{"type":{"Typography":{"text":"B","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":true,"lines":true,"themed":false}}}},{"type":{"Typography":{"text":"F","role":"Fine","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}},"mark":"small"},{"type":{"Typography":{"text":"E","role":"Eyebrow","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}},{"type":{"Typography":{"text":"C","role":"Code","tone":"","style":{"numeric":true,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}},{"type":{"Typography":{"text":"9","role":"Stat","tone":"","style":{"numeric":false,"clamp":0,"align":"End","struck":false,"lines":false,"themed":true}}}},{"type":{"Stack":{"direction":"Col","gap":"Normal","align":"Stretch","justify":"Start","wrap":false,"content":[]}}},{"type":{"Stack":{"direction":"Row","gap":"Tight","align":"Center","justify":"Between","wrap":true,"content":[]}}},{"type":{"Stack":{"direction":"Row","gap":"Loose","align":"End","justify":"Start","wrap":false,"content":[]}}},{"type":{"Panel":{"tone":"{g.tone}","content":[{"type":{"Typography":{"text":"p","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}]}}},{"type":{"Icon":{"lib":"Lucide","name":"star","aria_label":null,"url":null,"size":"Small","tone":"Gold"}}},{"type":{"Link":{"href":"/a","content":[{"type":{"Typography":{"text":"a","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"style":"Inline"}}},{"type":{"Link":{"href":"/c","content":[{"type":{"Typography":{"text":"c","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"style":"Card"}}},{"type":{"Image":{"src":"i.png","alt_text":"I"}}},{"type":{"Badge":{"variant":"","text":"Live","tone":"Live","icon":"radio"}}},{"type":{"Progress":{"progress_percentage_key":"pct","tone":"Success"}}},{"type":{"Countdown":{"until":"ends","as":"left","text":"{left}","shake_under_ms_left":null,"role":"Stat","tone":"Warning"}}},{"type":{"Time":{"opts":{"timeago":true},"key":"at","role":"Fine","tone":""}}},{"type":{"Time":{"opts":{"timeago":false,"format":"PP","showTimeForToday":null,"timeForTodayFormat":null},"key":"at","role":"Body","tone":""}}},{"type":{"ScoreTicker":{"value_key":"score","icon":{"type":{"Icon":{"lib":"Lucide","name":"star","aria_label":null,"url":null,"size":"Inherit","tone":""}}}}}},{"type":{"Overlay":{"corner":"TopLeft","content":[]}}},{"type":{"Condition":{"condition":{"ResponseExists":"x"},"content":[{"type":{"Typography":{"text":"x","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}]}}},{"type":{"Action":{"label":"Go","icon":null,"look":"Primary","act":{"action":"go","inputs":{"id":"{p.id}"},"disabled":"Always"},"full_width":true,"submit":true,"scroll_to":null}}},{"type":{"Action":{"label":"Back","icon":null,"look":"Secondary","act":{"action":"back","inputs":null,"disabled":null},"full_width":false,"submit":false,"scroll_to":"top"}}},{"type":{"Action":{"label":"Close","icon":"x","look":"Icon","act":{"action":"close","inputs":null,"disabled":null},"full_width":false,"submit":false,"scroll_to":null}}},{"type":{"Action":{"label":"Delete","icon":null,"look":"Danger","act":{"action":"del","inputs":null,"disabled":null},"full_width":false,"submit":false,"scroll_to":null}}},{"type":{"Notice":{"tone":"Danger","icon":"alert","title":"Error","body":"Oops","actions":[]}}},{"type":{"TabBar":{"style":"Segmented","tabs":[{"label":"A","icon":"home","active":"Always","act":{"action":"a","inputs":null,"disabled":null},"href":null},{"label":"B","icon":null,"active":"Always","act":null,"href":"/b"}]}}},{"type":{"Stepper":{"title":"Week 1","subtitle":"Jan","previous":{"action":"prev","inputs":null,"disabled":null},"next":null}}},{"type":{"Row":{"lead":[{"type":{"Typography":{"text":"1","role":"Stat","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"title":"{p.name}","meta":"m","trail":[{"type":{"Typography":{"text":"{p.score}","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"state":"{p.state}","href":"/p","act":null}}},{"type":{"Row":{"lead":[],"title":"x","meta":null,"trail":[],"state":"Selected","href":null,"act":{"action":"x","inputs":null,"disabled":null}}}},{"type":{"Choice":{"content":[{"type":{"Typography":{"text":"c","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"state":"Muted","act":{"action":"pick","inputs":null,"disabled":null},"direction":"Row"}}},{"type":{"Stat":{"value":"12","label":"Wins","tone":"Success"}}},{"type":{"Meter":{"label":"Done","value":"{pct}%","percent":"{pct}","tone":"Info"}}},{"type":{"Segments":{"items_key":"s","as":"x","state":"{x.state}","label":"Lives"}}},{"type":{"Empty":{"icon":"inbox","title":"Nothing","body":"Yet","loading":false,"actions":[]}}},{"type":{"Empty":{"icon":null,"title":"Loading","body":null,"loading":true,"actions":[]}}},{"type":{"Person":{"avatar":"{u.avatar}","name":"{u.name}","meta":"host","size":"Large"}}},{"type":{"AvatarStack":{"items_key":"ps","as":"p","avatar":"{p.avatar}","name":"{p.name}","max":3}}},{"type":{"Divider":{"label":null}}},{"type":{"Divider":{"label":"or"}}},{"type":{"Log":{"text":"{out}"}}},{"type":{"Split":{"media":[{"type":{"Image":{"src":"a","alt_text":"b"}}}],"content":[{"type":{"Typography":{"text":"c","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}]}}},{"type":{"ActionBar":{"content":[]}}},{"type":{"Media":{"src":"m.png","alt":"M","aspect":"Poster","size":"Small","shape":"Rounded","fit":"Cover","badge":[],"caption":[{"type":{"Typography":{"text":"cap","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"tint":"Red","caption_placement":"Over"}}},{"type":{"Media":{"src":"a.png","alt":"A","aspect":"Square","size":"ExtraSmall","shape":"Round","fit":"Cover","badge":[],"caption":[],"tint":"","caption_placement":"Over"}}},{"type":{"Media":{"src":"l.png","alt":"L","aspect":"Square","size":"Medium","shape":"Square","fit":"Contain","badge":[],"caption":[],"tint":"","caption_placement":"Over"}}},{"type":{"Items":{"items_key":"cells","as":"c","layout":{"Grid":{"columns":4,"gap":"Tight","fixed":true}},"content":[{"type":{"Typography":{"text":"{c}","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}]}}},{"type":{"Items":{"items_key":"xs","as":"x","layout":{"Scroll":{"gap":"Normal","rows":5}},"content":[]}}}]',
  ui_page: '{"id":"page","display":{"Page":{"width":"Normal","content":[{"type":{"Typography":{"text":"x","role":"Body","tone":"","style":{"numeric":false,"clamp":0,"align":"Start","struck":false,"lines":false,"themed":false}}}}],"center":false,"backdrop":"Default","typeface":"Default"}}}',
};

const sameAsRust = (name: string, value: unknown) => assert.equal(JSON.stringify(value), rust[name], name);
const text = (t: string) => new WorkflowDisplayBuilder().text(t).build();

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
        .condition(Cond.always())
        .visual(
          new WorkflowVisualBuilder('page')
            .block([text('Hi'), new WorkflowDisplayBuilder().div([text('a'), text('b')]).build()])
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
    new WorkflowVisualBuilder('p').page('Wide', [text('p')]).centered().backdrop('Dark').typeface('Spooky').build(),
  ]);
});

test('display variants match Rust', () => {
  const d = () => new WorkflowDisplayBuilder();
  sameAsRust('displays', [
    d(),
    d().scoreTicker('score', d().lucideIcon('star').build()),
    d().carousel('items', 'item', [text('{item.name}')]),
    d().carouselSelect('items', 'item', [], 'pick', { id: '{item.id}' }, 'active'),
    d().popover('open', null, [text('t')], [new WorkflowVisualBuilder('p').modal([]).build()]),
    d().searchable('/api', 'q', 'pick', 'row', 'Search', [text('{row}')]),
    d().barChart('data', { k: { label: 'K' } }, 'day', 'k', null),
    d().lineChart('data', [], 'x', 'y', 'left'),
    d().areaChart('data', null, 'x', 'y'),
    d().countdown('ends', 'left', '{left}s', 5000),
    d().tooltipButton([text('tip')], [text('b')], null, 'act', 'ghost', 'sm'),
    d().tooltip('<b>x</b>', []),
    d().submit([text('Go')], null, 'go', 'default', 'lg'),
    d().button([text('Go')], { a: 1 }, 'go', 'outline', 'default'),
    d().lucideIcon('heart', 'Like'),
    d().icon('Url', '', null, 'https://x/y.png'),
    d().video('v.mp4'),
    d().audioClip('a.mp3', '{secs}', [0.5, 2.25]),
    d().table('rows', 'r', [tableHeader('Name', 'name', '10rem', null)], [text('{r.name}')]),
    d().time('at', { timeago: true }),
    d().form('save', [], 'OnChange'),
    d().card([], { Image: { image_key: 'img' } }),
    d().card([], 'Vertical'),
    d().mutlimediaLobby(),
    d().loader(),
    d().wysiyg('body'),
    d().railyardBoard('board', 'run', 'save'),
    d().condition(Cond.responseExists('x'), [text('x')]),
    d().progress('pct'),
    d().badge('secondary', 'New'),
    d().link('/x', []),
    d().revealCards(['a']),
    d().userAvatar('uid'),
    d().partyCanvas(),
    d().gameInvite(),
  ].map((b) => b.build()));
});

test('display variants without a Rust fixture', () => {
  const d = () => new WorkflowDisplayBuilder();
  assert.deepEqual(d().image('a.png', 'A').build(), { type: { Image: { src: 'a.png', alt_text: 'A' } } });
  assert.deepEqual(d().list('xs', 'x', []).build(), { type: { List: { items_key: 'xs', as: 'x', content: [] } } });
  assert.deepEqual(d().time('at', { timeago: false, format: 'PP', showTimeForToday: true, timeForTodayFormat: null }).build().type, {
    Time: { opts: { timeago: false, format: 'PP', showTimeForToday: true, timeForTodayFormat: null }, key: 'at', role: 'Body', tone: '' },
  });
  assert.deepEqual(d().tile('wf').build().type, { Tile: { workflow: 'wf' } });
  assert.deepEqual(d().selectCards(['k']).build().type, { SelectCards: { card_id_keys: ['k'] } });
  assert.deepEqual(d().activity('state', 'move').build().type, { Activity: { state: 'state', action: 'move' } });
  assert.deepEqual(d().dotsBoard('b', 'a').build().type, { DotsBoard: { board: 'b', action: 'a' } });
  assert.deepEqual(d().wordWheel('p', 'a').build().type, { WordWheel: { puzzle: 'p', action: 'a' } });
  assert.deepEqual(d().snakeRun('l', 'a').build().type, { SnakeRun: { level: 'l', action: 'a' } });
  assert.deepEqual(d().shareResult('v', 'Share').build().type, { ShareResult: { value: 'v', label: 'Share', celebrate: false } });
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
      .type(InputTypeBuilder.select(Cond.always(), 'opts', 'o', '{o.id}', [text('{o.label}')], null, []))
      .required(true)
      .defaultValue('a')
      .readonly(Cond.not(Cond.always()))
      .onChangeAction('changed', { v: '{v}' })
      .build(),
    input(InputTypeBuilder.number(1, null)),
    input(InputTypeBuilder.textArea('Say', 4)),
    input(InputTypeBuilder.multiSelectFromList('i', 'a', 'v', [], 2, null, 3)),
    input(InputTypeBuilder.selectFromList('i', 'a', 'v', [], 3)),
    input(InputTypeBuilder.switch()),
    input(InputTypeBuilder.wysiwyg()),
    input(InputTypeBuilder.list('r', 'k', 'f')),
    input(InputTypeBuilder.checkbox('ok')),
  ];
  sameAsRust('inputs', inputs);
  sameAsRust('input_display', new WorkflowDisplayBuilder().input('name', inputs[0]).build());
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
    IconBuilder.lucide('a').name('b').intoWorkflowDisplay(),
    new IconBuilder('Emoji', 'smile').intoWorkflowDisplay(),
  ]);
});

test('ui building blocks match Rust', () => {
  const always = Cond.always();
  sameAsRust('ui', [
    ui.hero('Back online', {
      eyebrow: 'New',
      lead: 'Play now',
      aside: [ui.chip('Beta', { icon: 'flask-conical' })],
      actions: [ui.linkPrimary('/download', 'Get the app'), ui.linkSecondary('/docs', 'Docs')],
    }),
    ui.section('Improved', [ui.grid(2, [ui.feature('Rosters', 'All 32 teams.', { icon: 'users', href: '/teams' })], { fixed: true, gap: 'Tight' })], {
      lead: 'Lots',
    }),
    ui.displayTitle('D'),
    ui.title('T', { tone: 'Primary' }),
    ui.heading('H', { align: 'Center' }),
    ui.subheading('S', { clamp: 2 }),
    ui.lead('L'),
    ui.body('B', { struck: true, lines: true }),
    ui.fine('F', { mark: 'small' }),
    ui.eyebrow('E'),
    ui.code('C', { numeric: true }),
    ui.statText('9', { themed: true, align: 'End' }),
    ui.column('Normal', []),
    ui.row('Tight', [], { justify: 'Between' }),
    ui.row('Loose', [], { wrap: false, align: 'End' }),
    ui.panel([ui.body('p')], { tone: '{g.tone}' }),
    ui.icon('star', 'Small', { tone: 'Gold' }),
    ui.linkInline('/a', 'a'),
    ui.linkCard('/c', [ui.body('c')]),
    ui.image('i.png', 'I'),
    ui.badge('Live', 'Live', { icon: 'radio' }),
    ui.progress('pct', 'Success'),
    ui.countdown('ends', 'left', '{left}', { role: 'Stat', tone: 'Warning' }),
    ui.time('at', { timeago: true }),
    ui.time('at', { timeago: false, format: 'PP', showTimeForToday: null, timeForTodayFormat: null }, { role: 'Body' }),
    ui.scoreTicker('score', ui.icon('star')),
    ui.overlay('TopLeft', []),
    ui.when(Cond.responseExists('x'), [ui.body('x')]),
    ui.primaryAction('Go', ui.act('go', { inputs: { id: '{p.id}' }, disabled: always }), { fullWidth: true, submit: true }),
    ui.secondaryAction('Back', ui.act('back'), { scrollTo: 'top' }),
    ui.iconAction('x', 'Close', ui.act('close')),
    ui.action('Delete', 'Danger', ui.act('del')),
    ui.notice('Danger', 'Oops', { title: 'Error', icon: 'alert', actions: [] }),
    ui.tabs('Segmented', [ui.tab('A', always, ui.act('a'), { icon: 'home' }), ui.tabLink('B', always, '/b')]),
    ui.stepper('Week 1', ui.act('prev'), null, { subtitle: 'Jan' }),
    ui.listRow('{p.name}', { lead: [ui.statText('1')], trail: [ui.body('{p.score}')], meta: 'm', state: '{p.state}', href: '/p' }),
    ui.listRow('x', { act: ui.act('x'), state: 'Selected' }),
    ui.choice([ui.body('c')], ui.act('pick'), { state: 'Muted', across: true }),
    ui.stat('12', 'Wins', { tone: 'Success' }),
    ui.meter('{pct}', { label: 'Done', value: '{pct}%', tone: 'Info' }),
    ui.segments('s', 'x', '{x.state}', { label: 'Lives' }),
    ui.empty('Nothing', { body: 'Yet', icon: 'inbox' }),
    ui.loading('Loading'),
    ui.person('{u.avatar}', '{u.name}', { meta: 'host', size: 'Large' }),
    ui.avatarStack('ps', 'p', '{p.avatar}', '{p.name}', { max: 3 }),
    ui.divider(),
    ui.labelledDivider('or'),
    ui.log('{out}'),
    ui.split([ui.image('a', 'b')], [ui.body('c')]),
    ui.actionBar([]),
    ui.media('m.png', 'M', 'Poster', 'Small', { tint: 'Red', badge: [], caption: [ui.body('cap')] }),
    ui.avatar('a.png', 'A', 'ExtraSmall'),
    ui.logo('l.png', 'L', 'Medium'),
    ui.items('cells', 'c', { Grid: { columns: 4, gap: 'Tight', fixed: true } }, [ui.body('{c}')]),
    ui.items('xs', 'x', { Scroll: { gap: 'Normal', rows: 5 } }, []),
  ]);
  sameAsRust('ui_page', ui.page('Normal', [ui.body('x')]));
});

test('a page built from ui helpers sends no classes', () => {
  const def = ui.workflow('hi', 'Hi', {
    home: ui.node('Home', [
      ui.page('Narrow', [
        ui.hero('Hi', { lead: 'Say hello', actions: [ui.linkPrimary('/x', 'Go')] }),
        ui.form('go', [ui.column('Tight', [ui.textInput('name', 'Name'), ui.numberInput('n', 1, 9), ui.switchInput('on'), ui.submit('Go', 'go')])]),
        ui.section('More', [ui.grid(2, [ui.feature('A', 'B'), ui.panel([ui.stat('1', 'One')])])]),
        ui.items('xs', 'x', 'Divided', [ui.listRow('{x.name}', { lead: [ui.avatar('{x.avatar}', '{x.name}', 'Small')] })]),
        ui.when(Cond.responseExists('n'), [ui.notice('Info', 'Saved'), ui.badge('{n}')]),
      ]),
    ]),
  });
  const keys = new Set<string>();
  const walk = (value: unknown) => {
    if (Array.isArray(value)) value.forEach(walk);
    else if (value && typeof value === 'object')
      for (const [key, inner] of Object.entries(value)) {
        keys.add(key);
        walk(inner);
      }
  };
  walk(JSON.parse(JSON.stringify(def)));
  assert.ok(keys.has('Typography') || keys.has('Hero'));
  assert.ok(![...keys].some((key) => key === 'class' || key.endsWith('_class')));
});

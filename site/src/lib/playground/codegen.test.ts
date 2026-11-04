import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { typescriptFor } from './codegen.ts';
import * as ui from '../../../../sdk/bot-js/src/ui.ts';
import { Cond, WorkflowDisplayBuilder } from '../../../../sdk/bot-js/src/builder.ts';

const data = JSON.parse(readFileSync(new URL('./generated.json', import.meta.url), 'utf8'));
const builder = new URL('../../../../sdk/bot-js/src/builder.ts', import.meta.url).href;
const uiModule = new URL('../../../../sdk/bot-js/src/ui.ts', import.meta.url).href;
const dir = mkdtempSync(join(tmpdir(), 'tawks-codegen-'));

/** Runs the generated TypeScript against the real builders and helpers. */
async function rebuild(key: string, definition: unknown) {
  const code = typescriptFor(definition).replace(/import \{ (.*) \} from '@tawks\/bot';/, (_, names: string) => {
    const builders = names.split(', ').filter((n) => n !== 'ui');
    return [
      builders.length ? `import { ${builders.join(', ')} } from '${builder}';` : '',
      names.split(', ').includes('ui') ? `import * as ui from '${uiModule}';` : '',
    ].join('\n');
  });
  const file = join(dir, `${key}.ts`);
  writeFileSync(file, code);
  const { definition: rebuilt } = await import(file);
  return { code, rebuilt: JSON.parse(JSON.stringify(rebuilt)) };
}

// The generated TypeScript has to give back the very definition it was generated from.
for (const template of data.templates) {
  test(`the TypeScript for ${template.key} rebuilds it exactly`, async () => {
    const { code, rebuilt } = await rebuild(template.key, template.definition);
    assert.deepEqual(rebuilt, template.definition);
    assert.match(code, /ui\.page\(/);
  });
}

const onePage = (id: string, content: unknown[]) =>
  ui.workflow(id, id, { home: ui.node('Home', [ui.page('Normal', content as never, { id: 'p', center: true, typeface: 'Condensed' })]) });

test('every palette display rebuilds exactly', async () => {
  const definition = onePage('palette', data.palette.map((p: { display: unknown }) => p.display));
  const { rebuilt } = await rebuild('palette', definition);
  assert.deepEqual(rebuilt, definition);
});

test('every building block rebuilds exactly with its options', async () => {
  const always = Cond.always();
  const definition = onePage('blocks', [
    ui.hero('Hi', { eyebrow: 'E', lead: 'L', aside: [ui.chip('c', { icon: 'x' })], actions: [ui.linkPrimary('/', 'Go'), ui.linkSecondary('/b', 'B')], mark: 'top' }),
    ui.section('S', [ui.grid(3, [ui.feature('F', 'B', { icon: 'i', href: '/f' })], { gap: 'Tight', fixed: true })], { lead: 'L' }),
    ui.title('T', { tone: 'Primary', clamp: 2, align: 'Center', numeric: true, struck: true, lines: true, themed: true }),
    ui.displayTitle('D'),
    ui.statText('9', { mark: 'score' }),
    ui.row('Tight', [ui.body('a')], { justify: 'Between', wrap: false }),
    ui.column('Loose', [], { align: 'Center' }),
    ui.panel([ui.fine('p')], { tone: '{g.tone}' }),
    ui.icon('star'),
    ui.icon('star', 'Large', { tone: 'Gold', label: 'Star' }),
    ui.linkInline('/i', 'i'),
    ui.linkCard('/c', [ui.body('c')]),
    ui.link('/p', 'Plain', [ui.body('p')]),
    ui.image('a.png', 'A'),
    ui.badge('New', 'Live', { icon: 'radio' }),
    ui.progress('pct', 'Success'),
    ui.countdown('ends', 'left', '{left}', { role: 'Stat', tone: 'Warning', shakeUnderMsLeft: 500 }),
    ui.time('at', { timeago: true }, { role: 'Body', tone: 'Muted' }),
    ui.scoreTicker('score', ui.icon('star')),
    ui.overlay('TopLeft', [ui.body('o')]),
    ui.when(Cond.responseExists('x'), [ui.body('x')]),
    ui.form('go', [ui.textInput('name', 'Name', { required: true, default: 'a' }), ui.numberInput('n', 1, null), ui.switchInput('on'), ui.submit('Go', 'go')], { onChange: true }),
    ui.primaryAction('Go', ui.act('go', { inputs: { id: '{p.id}' }, disabled: always }), { fullWidth: true }),
    ui.secondaryAction('Back', ui.act('back'), { scrollTo: 'top', icon: 'arrow-left' }),
    ui.iconAction('x', 'Close', ui.act('close')),
    ui.action('Delete', 'Danger', ui.act('del'), { submit: true }),
    ui.notice('Danger', 'Oops', { title: 'Error', icon: 'alert', actions: [ui.primaryAction('Retry', ui.act('retry'))] }),
    ui.tabs('Segmented', [ui.tab('A', always, ui.act('a'), { icon: 'home' }), ui.tabLink('B', Cond.not(always), '/b')]),
    ui.buttonGroup([ui.groupButton('1', ui.act('give', { inputs: { amount: 1 } })), ui.groupButton('3', ui.act('give', { inputs: { amount: 3 } }))]),
    ui.stepper('Week', ui.act('prev'), null, { subtitle: 'Jan' }),
    ui.listRow('{p.name}', { lead: [ui.statText('1')], trail: [ui.body('{p.score}')], meta: 'm', state: '{p.state}', href: '/p', act: ui.act('open') }),
    ui.choice([ui.body('c')], ui.act('pick'), { state: 'Muted', across: true }),
    ui.stat('12', 'Wins', { tone: 'Success' }),
    ui.meter('{pct}', { label: 'Done', value: '{pct}%', tone: 'Info' }),
    ui.segments('s', 'x', '{x.state}', { label: 'Lives' }),
    ui.empty('Nothing', { body: 'Yet', icon: 'inbox', actions: [] }),
    ui.loading('Loading'),
    ui.person('{u.avatar}', '{u.name}', { meta: 'host', size: 'Large' }),
    ui.avatarStack('ps', 'p', '{p.avatar}', '{p.name}', { max: 3 }),
    ui.divider(),
    ui.labelledDivider('or'),
    ui.log('{out}'),
    ui.split([ui.image('a', 'b')], [ui.body('c')]),
    ui.actionBar([ui.submit('Save', 'save')]),
    ui.media('m.png', 'M', 'Poster', 'Small', { tint: 'Red', caption: [ui.body('cap')] }),
    ui.avatar('a.png', 'A', 'ExtraSmall'),
    ui.logo('l.png', 'L', 'Medium'),
    ui.items('cells', 'c', { Grid: { columns: 4, gap: 'Tight', fixed: true } }, [ui.body('{c}')]),
    new WorkflowDisplayBuilder().text('legacy').build(),
    { type: { Typography: { text: 'no style', role: 'Body', tone: '' } } },
  ]);
  const { code, rebuilt } = await rebuild('blocks', definition);
  assert.deepEqual(rebuilt, definition);
  assert.doesNotMatch(code, /class/);
});

test('classes from older definitions are left out', async () => {
  const definition = onePage('old', [{ class: 'text-lg', type: { Text: { text: 'x' } } }]);
  const { code } = await rebuild('old', definition);
  assert.doesNotMatch(code, /class/);
});

test('dropdowns use their builders', () => {
  const page = (type: any) => ({
    id: 'd', name: 'D', responses: {}, context: {}, actions: [], description: null, starting_server_action: null, multi_instance_per_user: false,
    nodes: { main: { title: 'M', description: null, condition: 'Always', parent_id: null, visuals: [{ id: 'v', display: { Block: { content: [
      { type: { Input: { id: 'stake', input: { readonly: null, type, default_value: null, required: false, on_change_action: null } } } },
    ] } } }] } },
  });
  const fixed = typescriptFor(page({ Select: { filter: null, items_key: '', as: '', value: '', content: [], selected_content: null, unselected_content: null, options: [{ value: '5', label: '$5' }], option_label: null, label: 'Stake', placeholder: 'Pick' } }) as any);
  assert.match(fixed, /InputTypeBuilder\.dropdown\('Stake',/);
  const rows = typescriptFor(page({ Select: { filter: null, items_key: 'rows', as: 'g', value: '{g.id}', content: [], selected_content: null, unselected_content: null, options: [], option_label: '{g.name}', label: 'Game', placeholder: null } }) as any);
  assert.match(rows, /InputTypeBuilder\.dropdownFrom\('Game', 'rows', 'g', '\{g\.id\}', '\{g\.name\}'\)/);
});

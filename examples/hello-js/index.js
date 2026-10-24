import { TawksBot, result, ui } from '@tawks/bot';

const bot = new TawksBot({
  rspcUrl: process.env.RSPC_URL ?? 'https://rspc.tawks.app/rspc',
  eventsUrl: process.env.EVENTS_WS_URL ?? 'wss://events.tawks.app/',
  token: process.env.BOT_TOKEN,
});

// Each person who opens the page gets their own instance, so the count is theirs.
const wave = await bot.serverAction('hello.wave', (ctx) => {
  const name = ctx.input('name')?.trim() || ctx.user.display_name;
  const waves = (ctx.number('waves') ?? 0) + 1;
  return result.responses({
    name,
    waves,
    reply: waves === 1 ? `The bot waves back at ${name}.` : `The bot has waved back ${waves} times.`,
  });
});

const hello = ui.workflow(
  'hello',
  'Hello',
  {
    home: ui.node('Hello', [
      ui.page([
        ui.text('Hello, {name}!', 'text-2xl font-bold'),
        ui.form('wave', [ui.textInput('name', 'Your name'), ui.submit('Wave', 'wave')], { cls: 'flex gap-2' }),
        ui.text('{reply}', 'text-muted-foreground'),
      ]),
    ]),
  },
  {
    responses: { name: 'there', reply: '', waves: 0 },
    actions: [ui.actions.run('wave', wave)],
  },
);

console.log('registered', await bot.workflow(hello));
await bot.run();

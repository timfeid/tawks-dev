// A live snake draft with a pick clock and autopick. It shows what a hub bot is
// for: state that has to be right to the second, for several people at once.
// Everything lasting lives in SQLite and the clock lives in a wake, so a crash
// at any moment loses nothing; the page is rebuilt from the database.
import { result, ui } from '@tawks/bot';
import { hubBot } from '@tawks/bot/hub';

const PICK_SECONDS = 30;
const ROUNDS = 5;
const PAGE = 'draft';

const SAMPLE_PLAYERS = [
  'Connor Reyes', 'Malik Osei', 'Jonas Lindqvist', 'Theo Brandt', 'Darius Cole', 'Sam Okafor',
  'Luca Moretti', 'Evan Park', 'Isaac Novak', 'Noah Tanaka', 'Oskar Berg', 'Mateo Silva',
  'Andre Fontaine', 'Ravi Menon', 'Cole Whitaker', 'Hugo Lefevre', 'Kai Nakamura', 'Felix Weber',
  'Dante Rossi', 'Owen Murphy', 'Viktor Hale', 'Tariq Aziz', 'Liam Dorsey', 'Marco Vega',
];

const hub = await hubBot();
const db = hub.db;

// Append only. A rollback runs older code on this same file, so never edit an
// entry and never drop anything.
db.migrate([
  `CREATE TABLE players (id INTEGER PRIMARY KEY, name TEXT NOT NULL, rank INTEGER NOT NULL);
   CREATE TABLE teams (user_id TEXT PRIMARY KEY, name TEXT NOT NULL, seat INTEGER);
   CREATE TABLE picks (number INTEGER PRIMARY KEY, user_id TEXT NOT NULL, player_id INTEGER NOT NULL UNIQUE, auto INTEGER NOT NULL DEFAULT 0, at TEXT NOT NULL);
   CREATE TABLE draft (id INTEGER PRIMARY KEY CHECK (id = 1), phase TEXT NOT NULL, deadline TEXT);
   INSERT INTO draft (id, phase) VALUES (1, 'lobby');`,
]);

if (!db.get('SELECT 1 AS found FROM players LIMIT 1')) {
  db.tx(() => {
    SAMPLE_PLAYERS.forEach((name, index) => db.run('INSERT INTO players (name, rank) VALUES (?, ?)', name, index + 1));
  });
}

type Team = { user_id: string; name: string; seat: number };
const state = () => db.get<{ phase: string; deadline: string | null }>('SELECT phase, deadline FROM draft')!;
const teams = () => db.all<Team>('SELECT user_id, name, seat FROM teams ORDER BY seat, rowid');
const pickCount = () => db.get<{ n: number }>('SELECT count(*) AS n FROM picks')!.n;

/** Whose pick it is: seats go 1 to N, then N to 1. */
function teamOnClock(number: number, all: Team[]): Team {
  const round = Math.floor(number / all.length);
  const index = number % all.length;
  return all[round % 2 === 0 ? index : all.length - 1 - index];
}

/** What everyone sees. */
function shared() {
  const { phase, deadline } = state();
  const all = teams();
  const number = pickCount();
  const onClock = phase === 'drafting' ? teamOnClock(number, all) : null;
  const picks = db.all<{ number: number; user_id: string; name: string; auto: number }>(
    'SELECT p.number, p.user_id, x.name, p.auto FROM picks p JOIN players x ON x.id = p.player_id ORDER BY p.number',
  );
  return {
    phase,
    deadline: deadline ?? '',
    can_start: phase === 'lobby' && all.length >= 2,
    on_clock: onClock?.name ?? '',
    round: all.length ? Math.floor(number / all.length) + 1 : 1,
    pick_number: number + 1,
    team_count: all.length,
    teams: all.map((team) => ({
      name: team.name,
      roster: picks.filter((pick) => pick.user_id === team.user_id).map((pick) => pick.name + (pick.auto ? ' (auto)' : '')).join(', ') || 'No picks yet',
      state: team.user_id === onClock?.user_id ? 'Active' : 'Default',
    })),
    available: db
      .all<{ id: number; name: string; rank: number }>(
        'SELECT id, name, rank FROM players WHERE id NOT IN (SELECT player_id FROM picks) ORDER BY rank LIMIT 12',
      )
      .map((player) => ({ id: player.id, name: player.name, rank: `#${player.rank}` })),
  };
}

/** What only this person sees. */
function mine(userId: string) {
  const { phase } = state();
  const all = teams();
  return {
    joined: all.some((team) => team.user_id === userId),
    is_my_turn: phase === 'drafting' && teamOnClock(pickCount(), all).user_id === userId,
  };
}

/** Every open page hears about it, and the people playing learn whose turn it is. */
const pushAll = () =>
  hub.refresh(PAGE, () => [[null, shared()], ...teams().map((team): [string, ReturnType<typeof mine>] => [team.user_id, mine(team.user_id)])]);

async function startClock(number: number) {
  const deadline = new Date((await hub.now()).getTime() + PICK_SECONDS * 1000).toISOString();
  db.run('UPDATE draft SET deadline = ?', deadline);
  await hub.wakeAt('draft.clock', deadline, { number });
}

/** Records one pick and moves on, all or nothing. Refuses a stale or taken pick. */
async function makePick(userId: string, playerId: number, auto: boolean) {
  const all = teams();
  const number = pickCount();
  if (state().phase !== 'drafting') throw new Error('The draft is not running.');
  if (teamOnClock(number, all).user_id !== userId) throw new Error('It is not your pick.');
  if (db.get('SELECT 1 AS taken FROM picks WHERE player_id = ?', playerId)) throw new Error('That player is taken.');
  if (!db.get('SELECT 1 AS found FROM players WHERE id = ?', playerId)) throw new Error('No such player.');
  const at = (await hub.now()).toISOString();
  const finished = db.tx(() => {
    db.run('INSERT INTO picks (number, user_id, player_id, auto, at) VALUES (?, ?, ?, ?, ?)', number, userId, playerId, auto ? 1 : 0, at);
    if (number + 1 < all.length * ROUNDS) return false;
    db.run("UPDATE draft SET phase = 'done', deadline = NULL");
    return true;
  });
  if (finished) await hub.cancelWake('draft.clock');
  else await startClock(number + 1);
  await pushAll();
}

hub.onWake('draft', async (wake) => {
  const { number } = wake.data as { number: number };
  // A wake for a pick already made is old news.
  if (state().phase !== 'drafting' || pickCount() !== number) return;
  const best = db.get<{ id: number }>('SELECT id FROM players WHERE id NOT IN (SELECT player_id FROM picks) ORDER BY rank LIMIT 1');
  if (best) await makePick(teamOnClock(number, teams()).user_id, best.id, true);
});

hub.onForget((userId) => {
  db.run('DELETE FROM teams WHERE user_id = ? AND (SELECT phase FROM draft) = \'lobby\'', userId);
  db.run("UPDATE teams SET name = 'Deleted account' WHERE user_id = ?", userId);
});

const load = await hub.action('draft.load', (ctx) => result.responses({ ...shared(), ...mine(ctx.user.id) }));

const join = await hub.action('draft.join', async (ctx) => {
  if (ctx.anonymous) return result.error('Sign in to join the draft.');
  if (state().phase !== 'lobby') return result.error('The draft has started.');
  db.run('INSERT OR IGNORE INTO teams (user_id, name) VALUES (?, ?)', ctx.user.id, ctx.user.display_name);
  await pushAll();
  return result.none();
});

const start = await hub.action('draft.start', async (ctx) => {
  if (state().phase !== 'lobby') return result.error('The draft has started.');
  if (!teams().some((team) => team.user_id === ctx.user.id)) return result.error('Join the draft first.');
  const all = teams();
  if (all.length < 2) return result.error('A draft needs two teams.');
  db.tx(() => {
    // Seats follow the order people joined, which a test can rely on.
    db.all<{ user_id: string }>('SELECT user_id FROM teams ORDER BY rowid').forEach((team, seat) =>
      db.run('UPDATE teams SET seat = ? WHERE user_id = ?', seat, team.user_id),
    );
    db.run("UPDATE draft SET phase = 'drafting'");
  });
  await startClock(0);
  await pushAll();
  return result.none();
});

const pick = await hub.action('draft.pick', async (ctx) => {
  const player = ctx.number('player');
  if (player === undefined) return result.error('Pick a player.');
  try {
    await makePick(ctx.user.id, player, false);
  } catch (error) {
    return result.error(error instanceof Error ? error.message : 'That pick did not go through.');
  }
  return result.none();
});

await hub.page(
  ui.workflow(
    PAGE,
    'Draft room',
    {
      home: ui.node('Draft room', [
        ui.page('Wide', [
          ui.hero('Draft room', { lead: 'A snake draft with a clock on every pick. Out of time, and the best player left is yours.' }),
          ui.when(ui.cond.equals('phase', 'lobby'), [
            ui.section('Waiting to start', [
              ui.body('{team_count} teams so far.'),
              ui.when(ui.cond.not(ui.cond.equals('joined', true)), [ui.primaryAction('Join the draft', ui.act('join'))]),
              ui.when(ui.cond.and(ui.cond.equals('joined', true), ui.cond.equals('can_start', true)), [
                ui.primaryAction('Start the draft', ui.act('start')),
              ]),
            ]),
          ]),
          ui.when(ui.cond.equals('phase', 'drafting'), [
            ui.section('Round {round}, pick {pick_number}', [
              ui.row('Tight', [
                ui.body('On the clock: {on_clock}'),
                ui.countdown('{deadline}', 'left', '{left.total_seconds}s', { shakeUnderMsLeft: 5_000 }),
              ]),
              ui.when(ui.cond.equals('is_my_turn', true), [ui.notice('Success', 'It is your pick.')]),
            ]),
            ui.section('Best available', [
              ui.items('available', 'p', { Stack: { gap: 'Tight' } }, [
                ui.listRow('{p.name}', {
                  meta: '{p.rank}',
                  trail: [
                    ui.when(ui.cond.equals('is_my_turn', true), [
                      ui.secondaryAction('Pick', ui.act('pick', { inputs: { player: '{p.id}' } })),
                    ]),
                  ],
                }),
              ]),
            ]),
          ]),
          ui.when(ui.cond.equals('phase', 'done'), [ui.notice('Success', 'The draft is done.')]),
          ui.section('Teams', [
            ui.items('teams', 't', { Stack: { gap: 'Tight' } }, [ui.listRow('{t.name}', { meta: '{t.roster}', state: '{t.state}' })]),
          ]),
        ]),
      ]),
    },
    {
      start: load,
      responses: { phase: 'lobby', teams: [], available: [], team_count: 0, joined: false, can_start: false, is_my_turn: false },
      actions: [ui.actions.run('join', join), ui.actions.run('start', start), ui.actions.run('pick', pick)],
    },
  ),
);

// A crash can land between saving a pick and setting the next clock.
if (state().phase === 'drafting') {
  const { deadline } = state();
  await hub.wakeAt('draft.clock', deadline ?? (await hub.now()).toISOString(), { number: pickCount() });
}
await pushAll().catch(() => {});
await hub.run();

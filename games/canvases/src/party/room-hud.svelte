<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import { Shield, Banknote, CircleHelp, Settings2, ExternalLink, ArrowDownLeft, ChevronsDown, Bot, Swords, ChevronLeft, ChevronRight, CircleDot, Coins, Crosshair, Crown, Dices, Eye, Gamepad2, Candy, Heart, LoaderCircle, Maximize2, Minimize2, MousePointerClick, Play, Plus, Skull, Star, Target, Trophy, Users, Volume2, VolumeX, X, Zap, Wind, BatteryFull, Filter } from '@lucide/svelte';
  import type { BoutHud } from '@tawks/bout-sdk';
  import type { ReplayStatus } from './replay';
  import type { StickValue } from './input';
  import Joystick from './joystick.svelte';
  import PositionHud from './position-hud.svelte';
  import { ITEMS } from './board/items';
  import { grappleResult, type GrappleNotice } from './board/grapple';
  import ItemIcon from './item-icon.svelte';
  import SettingsPanel from './settings-panel.svelte';
  import BoardStandings from './board/standings.svelte';
  import CharacterPreview from './character-preview.svelte';
  import { onWardrobeSaved, wardrobeClient, type Wardrobe } from './costumes/client';
  import { DESIGNS } from './board/design';
  import { bound, gamepad, keyName, settings } from './settings';
  import type { Bonus, BoutControls, CharacterDef, ClientMsg, Highlight, Item, FeedbackResult, RoomView, RoundEvent, Taunt } from './protocol';
  import { UPGRADES, type Upgrade } from './protocol';
  import type { ConnectionStatus } from './connection';
  import{visibleTeams,visibleTeamFor}from'./team-visibility';
  import{isGameplayTyping}from'./typing';

  let {
    room,
    roster,
    me,
    spectating,
    status,
    hasPass,
    timeLeft,
    showGo,
    muted,
    filming = false,
    guest = false,
    touch,
    overview,
    fullscreen,
    pointerLocked,
    briefHidden,
    replayStatus=null,
    onskipreplay=()=>{},
    hud,
    firstPerson,
    errorMessage,
    feedbackResult=null,
    send,
    toggleMute,
    toggleOverview,
    toggleFullscreen,
    onpopout=undefined,poppedOut=false,
    onaim,
    lobView = null,
    teleporting = false,
    onteleport = () => {},
    onlook,
    onfire,
    oncrouch,
    onstick,
  }: {
    room: RoomView | null;
    roster: CharacterDef[];
    me: string | null;
    spectating: boolean;
    status: ConnectionStatus;
    hasPass: boolean;
    timeLeft: number | null;
    showGo: boolean;
    muted: boolean;
    filming?: boolean;
    guest?: boolean;
    touch: boolean;
    overview: boolean;
    fullscreen: boolean;
    pointerLocked: boolean;
    briefHidden: boolean;
    replayStatus?:ReplayStatus|null;
    onskipreplay?:()=>void;
    hud: BoutHud | null;
    firstPerson: boolean;
    errorMessage: string | null;
    feedbackResult?:FeedbackResult|null;
    send: (msg: ClientMsg) => void;
    toggleMute: () => void;
    toggleOverview: () => void;
    toggleFullscreen: () => void;
    onpopout?:()=>void;
    poppedOut?:boolean;
    onaim: (item: Item | null) => void;
    /** A bomb being thrown in first person: the power swing, the fuse and the wind. */
    lobView?: { power: number; charging: boolean; fuseLeft: number; wind: number; windAngle: number } | null;
    /** The sandbox's free camera, and clicking a space to jump there. */
    teleporting?: boolean;
    onteleport?: (on: boolean) => void;
    onlook: (dx: number, dy: number) => void;
    onfire: (down: boolean) => void;
    oncrouch: (down: boolean) => void;
    onstick: (value: StickValue) => void;
  } = $props();

  const byId = $derived(new Map(roster.map((c) => [c.id, c])));
  const mine = $derived(room?.members.find((m) => m.user_id === me) ?? null);
  // The look is chosen in the app's settings now. The lobby shows it, opens the
  // settings to change it, and asks for one before a first game.
  let wardrobe = $state<Wardrobe | null>(null);
  const wardrobeApi = wardrobeClient();
  $effect(() => {
    if (!wardrobeApi || guest) return;
    let stopped = false;
    void wardrobeApi.load().then((loaded) => { if (!stopped) wardrobe = loaded; }).catch(() => {});
    const stop = onWardrobeSaved((saved) => {
      wardrobe = saved;
      untrack(() => send({ type: 'refresh_costume' }));
    });
    return () => { stopped = true; stop(); };
  });
  const needsAvatar = $derived(!guest && !!wardrobe && !wardrobe.set_up);
  const myLook = $derived<CharacterDef | null>(mine && byId.get(mine.character) ? { ...byId.get(mine.character)!, costume: mine.costume } : null);
  const isHost = $derived(!!me && room?.host_id === me);
  const host = $derived(room?.members.find((m) => m.user_id === room?.host_id) ?? null);
  const phase = $derived(room?.phase.name ?? 'lobby');
  const seatsFull = $derived(!!room && room.members.length >= room.max_seats);
  const bySeat = (seat: number) => room?.members.find((m) => m.seat === seat);
  const bouts = $derived(new Map((room?.bouts ?? []).map((bout) => [bout.key, bout])));
  const allGames = $derived([
    ...(room?.bouts ?? []).map((bout) => [bout.key, bout.name] as [string, string]),
    ...(room?.table_games ?? []),
  ]);
  const gameChoice = $derived(allGames.find(([key]) => key === room?.minigame) ?? null);
  /** Set up for a whole Tawkathon; otherwise one bout or card game on its own. */
  const partyMode = $derived(room?.party_mode ?? true);
  const cityMode=$derived(room?.city_mode??false);
  let cityWatch=$state(false);
  const cityConfig=$derived(room?.city_config??{police:1,jobs:3,host_police:false});
  const tableGame = $derived((room?.table_games ?? []).some(([key]) => key === room?.minigame));
  const TABLE_MIN: Record<string, number> = { coven: 4, landlord: 2 };
  const tableMin = $derived(TABLE_MIN[room?.minigame ?? ''] ?? 2);
  const people = $derived((room?.members ?? []).filter((m) => !m.is_bot).length);
  const TABLE_COPY: Record<string, string> = {
    coven: 'Hidden witches, whispered accusations, four to twelve players at one table.',
    landlord: 'Deal, rent and steal your way to three districts. Two to five players.',
  };
  const gameCopy = (key: string) => bouts.get(key)?.rules ?? TABLE_COPY[key] ?? 'A quick bout.';
  const CONTROLS: Record<BoutControls, string> = {
    move: 'Move',
    move_action: 'Move and dash',
    move_aim: 'Move and aim',
    first_person: 'First person',
  };
  const vote = $derived(room?.phase.name === 'vote' ? room.phase : null);
  const boardTeams=$derived(visibleTeams(room,null,'board'));
  const boutTeams=$derived(visibleTeams(room,hud,'bout'));
  const phaseTeams=$derived(room?.board&&['board','vote','game_over'].includes(room.phase.name)?boardTeams:boutTeams);
  const sides=$derived(phaseTeams.length?phaseTeams.map(team=>({...team,names:team.seats.map(seat=>bySeat(seat)?.name??'Someone')})):null);
  const mySide=$derived(visibleTeamFor(boutTeams,mine?.seat));
  const myTeam=$derived(mySide?.id??null);
  const myVote = $derived(vote && mine ? (vote.votes.find((v) => v.seat === mine.seat)?.key ?? null) : null);
  const canVote = $derived(!!vote && !!mine?.in_round && !spectating);
  function cycleGame(change: number) {
    const games = allGames;
    const index = games.findIndex(([key]) => key === room?.minigame);
    if (!games.length) return;
    const next = games[(index + change + games.length) % games.length];
    if (next) send({ type: 'choose_game', key: next[0] });
  }

  const board = $derived(cityMode?null:room?.board ?? null);
  let hookNotice=$state<GrappleNotice|null>(null);
  let hookLogPrimed=false;
  let hookLastSeen:string|null=null;
  let hookSequence=0;
  const hookTimers=new Set<ReturnType<typeof setTimeout>>();
  function hookLater(callback:()=>void,milliseconds:number) {
    const timer=setTimeout(()=>{hookTimers.delete(timer);callback();},milliseconds);
    hookTimers.add(timer);
  }
  onDestroy(()=>{for(const timer of hookTimers)clearTimeout(timer);});
  $effect(()=>{
    const entries=board?.log;
    if(!entries){hookLogPrimed=false;hookLastSeen=null;return;}
    const stamped=entries.map(entry=>JSON.stringify(entry));
    const from=!hookLogPrimed?entries.length:hookLastSeen===null?0:stamped.lastIndexOf(hookLastSeen)+1;
    hookLogPrimed=true;
    hookLastSeen=stamped[stamped.length-1]??hookLastSeen;
    for(const entry of entries.slice(from)) {
      const notice=grappleResult(entry,room?.members??[],mine?.seat??null);
      if(!notice)continue;
      const sequence=++hookSequence;
      // Match the local flight, so the result appears when the hook catches.
      hookLater(()=>{if(sequence===hookSequence)hookNotice=notice;},notice.delay*1000);
      hookLater(()=>{if(sequence===hookSequence)hookNotice=null;},notice.delay*1000+4500);
    }
  });
  /** On the board, everyone is listed in turn order. */
  const listed = $derived.by(() => {
    const members = room?.members ?? [];
    if (!board) return members;
    return [...members].sort((a, b) => board.order.indexOf(a.seat) - board.order.indexOf(b.seat));
  });
  const myToken = $derived(board && mine ? (board.tokens.find((t) => t.seat === mine.seat) ?? null) : null);
  const myTurn = $derived(!!board && !!mine && board.turn_seat === mine.seat && room?.phase.name === 'board');
  const travelling=$derived(!!board?.walk && !board.candy_van && board.walk.elapsed>=board.walk.delay && myTurn && board.walk.seat===mine?.seat && !spectating);
  const turnHolder = $derived(board ? (bySeat(board.turn_seat)?.name ?? 'Someone') : '');
  function routeLabel(node: number) {
    const from=board?.spaces.find(space=>space.id===board.tokens.find(token=>token.seat===board.turn_seat)?.space);
    const to=board?.spaces.find(space=>space.id===node);
    if(!board||!from||!to)return 'Continue';
    const angle=Math.atan2(to.x-from.x,to.z-from.z);
    const heading=['South','East','North','West'][(Math.round(angle/(Math.PI/2))+4)%4];
    const x=to.x+Math.sin(angle)*10,z=to.z+Math.cos(angle)*10;
    const zone=[...(DESIGNS[board.map]?.zones??[])].sort((a,b)=>Math.hypot(a.x-x,a.z-z)-Math.hypot(b.x-x,b.z-z))[0];
    return zone?`${heading} · ${zone.name}`:heading;
  }

  const EVENTS: Record<RoundEvent, { title: string; text: string }> = {
    coin_shower: { title: 'Coin shower', text: 'Floating coins are worth double this round. Vacuum them in.' },
    windfall: { title: 'Windfall', text: 'Everyone picks up 5 coins.' },
    item_rain: { title: 'Item rain', text: 'Everyone with room in their bag gets an item.' },
    prize_moves: { title: 'The candy moves', text: 'The chest has packed up for a new spot.' },
    underdog: { title: 'Underdog', text: 'Last place draws an item, and buys the candy cheap this round.' },
    fog: { title: 'Fog', text: 'Nobody can see anyone else\'s coins this round.' },
    toll_road: { title: 'Toll road', text: 'Every space you walk costs a coin this round.' },
    market_day: { title: 'Market day', text: 'The shop puts out five items at half price this round.' },
    earthquake: { title: 'Earthquake', text: 'The shortcuts are closed. Everyone takes the long way round.' },
    bounty: { title: 'Bounty', text: 'A price on the leader\'s head: whoever hits them first this round takes 10 coins.' },
    plague: { title: 'The plague', text: 'Someone has caught it. It costs a heart every turn, until they land beside someone and pass it on.' },
    shakedown: { title: 'Shakedown', text: 'Last place has robbed the leader of half their coins.' },
    last_round: {
      title: 'Last round',
      text: 'Every chest is open at half price, floating coins are worth double, and the bout pays twice.',
    },
  };
  const event = $derived(board?.event ? EVENTS[board.event] : null);
  let bannerRound = $state<number | null>(null);
  const eventRound = $derived(board?.event && room?.phase.name === 'board' ? board.round : null);
  $effect(() => {
    const round = eventRound;
    if (round === null) return;
    bannerRound = round;
    const timer = setTimeout(() => (bannerRound = null), 4000);
    return () => clearTimeout(timer);
  });
  const FINALE_SECONDS = 24;
  const BONUS_SECONDS = 3;
  const HIGHLIGHTS: Record<Highlight, (value: number) => string> = {
    biggest_swing: (n) => `Took ${n} coins in one go`,
    longest_walk: (n) => `Covered ${n} spaces in a single turn`,
    most_knockouts: (n) => `Hit the floor ${n} ${n === 1 ? 'time' : 'times'}`,
    worst_dice: (n) => `Rolled ${(n / 10).toFixed(1)} on average, the worst of it`,
  };
  const BONUSES: Record<Bonus, { title: string; text: (count: number) => string }> = {
    champion: { title: 'Bout champion', text: (n) => `Won the most bouts (${n})` },
    explorer: { title: 'Explorer', text: (n) => `Walked the most spaces (${n})` },
    troublemaker: { title: 'Troublemaker', text: (n) => `Used the most items (${n})` },
  };
  /** Shouted from the sidelines, whoever's turn it is. */
  const TAUNTS: { taunt: Taunt; label: string }[] = [
    { taunt: 'laugh', label: 'Ha!' },
    { taunt: 'clap', label: 'Nice one' },
    { taunt: 'wave', label: 'Hello' },
    { taunt: 'cry', label: 'Not fair' },
    { taunt: 'point', label: "You're next" },
    { taunt: 'hurry', label: 'Any day now' },
  ];
  let shoutedAt = $state(0);
  let charge = $state(6);


  const myBet = $derived(
    board && mine ? (board.bets.find((bet) => bet.seat === mine.seat)?.coins ?? 0) : 0,
  );
  const pot = $derived((board?.bets ?? []).reduce((sum, bet) => sum + bet.coins, 0));
  const canShout = $derived(!spectating && !!mine);
  const myPrizeCost = $derived(
    board && mine && board.underdog === mine.seat ? board.underdog_cost : (board?.prize_cost ?? 0),
  );
  const diceSet = $derived(!!board && (board.dice > 1 || board.fixed_roll !== null));
  const diceNote = $derived.by(() => {
    if (!board) return '';
    const notes = [];
    if (board.fixed_roll !== null) notes.push(`Golden die set to ${board.fixed_roll}`);
    else if (board.dice > 1) notes.push(`Rolling ${board.dice} dice`);
    if (myToken?.cursed) notes.push('Cursed: rolls up to 3');
    if (board.fixed_roll === null && board.handicap === mine?.seat) notes.push('Catch-up die: up to 3 more');
    return notes.join(' · ');
  });
  let showSettings = $state(false);
  let showInstructions=$state(false);
  function helpKey(event:KeyboardEvent){if(inArena&&event.code==='KeyH'&&!event.repeat&&!isGameplayTyping(event.target)){showInstructions=!showInstructions;event.preventDefault();}}
  let showSandboxTools = $state(false);
  const UPGRADE_ICONS = { turbo: Zap, wide_nozzle: Wind, big_battery: BatteryFull, filter: Filter } satisfies Record<Upgrade, unknown>;

  // A controller works the menus: the D-pad or left stick moves between the
  // buttons of whatever menu is up, and the action button presses one. Not
  // in a bout or while aiming, where the controller is playing.
  let hudRoot = $state<HTMLElement | null>(null);
  $effect(() => {
    let frame = 0;
    let focused = -1;
    let heldSince = 0;
    let pressed = false;
    const poll = () => {
      frame = requestAnimationFrame(poll);
      const pad = gamepad();
      const busy = !!lobView || phase === 'playing' || phase === 'practice' || phase === 'countdown';
      if (!pad || busy || !hudRoot) return;
      const menus = [...hudRoot.querySelectorAll<HTMLElement>('[data-pad-menu]')].filter((menu) => menu.offsetParent !== null);
      const menu = showSettings ? null : menus[menus.length - 1];
      const buttons = menu ? [...menu.querySelectorAll<HTMLButtonElement>('button:not([disabled])')] : [];
      if (!buttons.length) return;
      const { deadzone, action } = settings().pad;
      const [sx = 0, sy = 0] = pad.axes;
      const dir =
        pad.buttons[15]?.pressed || pad.buttons[13]?.pressed || sx > 0.6 || sy > 0.6 ? 1 : pad.buttons[14]?.pressed || pad.buttons[12]?.pressed || sx < -0.6 || sy < -0.6 ? -1 : 0;
      const now = performance.now();
      if (dir !== 0 && Math.abs(sx) + Math.abs(sy) >= deadzone) {
        if (heldSince === 0 || now - heldSince > 260) {
          focused = focused < 0 ? 0 : (focused + dir + buttons.length) % buttons.length;
          buttons[focused].focus({ preventScroll: true });
          heldSince = heldSince === 0 ? now : now - 160;
        }
      } else {
        heldSince = 0;
      }
      if (focused >= buttons.length) focused = buttons.length - 1;
      const down = !!pad.buttons[action]?.pressed;
      if (down && !pressed) {
        const target = buttons[Math.max(0, focused)];
        target.focus({ preventScroll: true });
        target.click();
      }
      pressed = down;
    };
    frame = requestAnimationFrame(poll);
    return () => cancelAnimationFrame(frame);
  });
  /** Which pair a seat is in, when playing in pairs. */


  // Only the golden die asks anything first: the number. Nothing else is
  // ever aimed by picking someone.
  let aiming = $state<Item | null>(null);
  $effect(() => onaim(aiming));

  // An item's skill check: a needle sweeping side to side, stopped with a
  // click or Space. The bands match the board's: the middle is perfect.
  let checkNeedle = $state(0);
  let checkLeft = $state(0);
  let checkSent = false;
  const checking = $derived(myTurn && board?.turn.step === 'check' ? board.turn : null);
  const checkSession = $derived.by(() => {
    if (!checking) return null;
    const started = board?.log.filter(entry => entry.kind === 'item' && entry.item === 'check_ready' && entry.by === mine?.seat).slice(-1)[0];
    return JSON.stringify([room?.match_id, board?.round, board?.turn_seat, checking.item, checking.number, started?.seq ?? 0]);
  });
  const stopCheck = () => {
    if (!checking || checkSent || board?.candy_van) return;
    checkSent = true;
    send({ type: 'check', stop: checkNeedle });
  };
  $effect(() => {
    // Room snapshots replace their objects, but are not new skill checks.
    // Only a new logical check starts a new animation clock or clears Stop.
    if (!checkSession) return;
    const session = untrack(() => checking);
    if (!session) return;
    const { period, fuse } = session;
    checkSent = false;
    let previous=performance.now(),seconds=0;
    let frame = 0;
    let padDown = false;
    const tick = () => {
      const now=performance.now();
      if(!board?.candy_van)seconds+=(now-previous)/1000;
      previous=now;
      const phase = (seconds % period) / (period / 2);
      if (!checkSent) checkNeedle = phase < 1 ? phase : 2 - phase;
      checkLeft = Math.max(0, fuse - seconds);
      // The controller's action button stops it too.
      const pressed = !isGameplayTyping()&&!!gamepad()?.buttons[settings().pad.action]?.pressed;
      if (!board?.candy_van && pressed && !padDown) stopCheck();
      padDown = pressed;
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    const onKey = (event: KeyboardEvent) => {
      if (board?.candy_van || !bound('action', event.code) || event.repeat) return;
      if(event.isComposing||isGameplayTyping(event.target))return;
      event.preventDefault();
      stopCheck();
    };
    window.addEventListener('keydown', onKey);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('keydown', onKey);
    };
  });
  // The cannon's fuse: the same sweeping needle, stopped to fire. The middle
  // flies exactly the charge; the further out, the more the shot strays.
  let fuseNeedle = $state(0);
  let fuseSent = false;
  const loading = $derived(myTurn && board?.turn.step === 'cannon' ? board.turn : null);
  const loadingKey = $derived(loading ? JSON.stringify([room?.match_id, board?.round, board?.turn_seat]) : null);
  const fireCannon = () => {
    if (!loading || fuseSent || board?.candy_van) return;
    fuseSent = true;
    send({ type: 'fire_cannon', power: charge, stop: fuseNeedle });
  };
  $effect(() => {
    if (!loadingKey) return;
    fuseSent = false;
    let previous = performance.now(), seconds = 0, frame = 0, padDown = false;
    const tick = () => {
      const now = performance.now();
      if (!board?.candy_van) seconds += (now - previous) / 1000;
      previous = now;
      const phase = (seconds % 0.9) / 0.45;
      if (!fuseSent) fuseNeedle = phase < 1 ? phase : 2 - phase;
      const pressed = !isGameplayTyping() && !!gamepad()?.buttons[settings().pad.action]?.pressed;
      if (!board?.candy_van && pressed && !padDown) fireCannon();
      padDown = pressed;
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    const onKey = (event: KeyboardEvent) => {
      if (board?.candy_van || !bound('action', event.code) || event.repeat) return;
      if (event.isComposing || isGameplayTyping(event.target)) return;
      event.preventDefault();
      fireCannon();
    };
    window.addEventListener('keydown', onKey);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('keydown', onKey);
    };
  });
  const CHECK_PROMPTS: Partial<Record<Item, string>> = {
    medkit: 'A perfect stop heals you to full; good, two hearts; weak, one.',
    shield: 'Good or better raises the shield; perfect pays 3 coins too.',
    compass: 'Perfect: 1 space before the chest. Good: 3 spaces. Weak: 6 spaces. Miss: stay here.',
    double_dice: 'Good or better gives you two dice; perfect pays 3 coins too.',
    golden_die: 'Perfect rolls your number; good is one off, weak two off.',
    heist: 'Good or better cracks the safe and takes candy off the leader.',
  };

  const SPACE_COLORS: Record<string, string> = {
    start: '#ffd23f',
    coin: '#4cc9f0',
    loss: '#ff5f6d',
    item: '#9b5de5',
    event: '#62c370',
    shop: '#ffa62b',
    hazard: '#7f1d3a',
    teleport: '#22d3ee',
    prize_spot: '#f8f0d0',
    cannon: '#f97316',
    tide: '#14b8a6',
    ice: '#bae6fd',
  };
  const SPACE_LABELS: Record<string, string> = {
    start: 'onto the start',
    coin: 'onto the sidewalk',
    loss: 'onto a rough space',
    item: 'onto an item space',
    event: 'onto a toll space',
    shop: 'past the shop',
    hazard: 'through the spikes',
    teleport: 'onto the teleport',
    prize_spot: 'toward the candy',
    cannon: 'into the cannon',
    tide: 'onto a tide space',
    ice: 'onto the ice',
  };
  const BRANCH_LETTERS = ['A', 'B', 'C'];

  const inArena = $derived(phase === 'playing' || phase === 'countdown' || phase === 'practice');
  const scoreOf = (seat: number) => hud?.scores?.find((s) => s.seat === seat) ?? null;
  const combat = $derived(hud?.combat ?? null);
  const hudAccent = $derived(byId.get(mine?.character ?? '')?.accent ?? '#facc15');
  const objective = $derived(hud?.objective ?? 'Outlast the field');

  let looking = $state(false);
  let lookAt: { x: number; y: number } | null = null;
  function lookStart(event: PointerEvent) {
    looking = true;
    lookAt = { x: event.clientX, y: event.clientY };
    (event.target as HTMLElement).setPointerCapture(event.pointerId);
  }
  function lookMove(event: PointerEvent) {
    if (!looking || !lookAt) return;
    onlook(event.clientX - lookAt.x, event.clientY - lookAt.y);
    lookAt = { x: event.clientX, y: event.clientY };
  }
  function lookEnd() {
    looking = false;
    lookAt = null;
  }

  function pump(side: -1 | 1) {
    send({ type: 'press', hand: side < 0 ? 'left' : 'right' });
    onstick({ x: side, z: 0 });
    setTimeout(() => onstick({ x: 0, z: 0 }), 55);
  }

  const onBoard = $derived(!!board && (phase === 'board' || !cityMode&&(phase === 'vote' || phase === 'game_over')));
  const mapBox = $derived.by(() => {
    if (!board) return null;
    const xs = board.spaces.map((space) => space.x);
    const zs = board.spaces.map((space) => space.z);
    const minX = Math.min(...xs) - 2;
    const minZ = Math.min(...zs) - 2;
    return { minX, minZ, w: Math.max(...xs) + 2 - minX, h: Math.max(...zs) + 2 - minZ };
  });
  const spacesById = $derived(new Map(board?.spaces.map((space) => [space.id, space]) ?? []));
  const spaceAt = (id: number) => spacesById.get(id) ?? null;

  const placements = $derived(room?.phase.name === 'results' ? room.phase.placements : []);

  // After each bout of a Tawkathon, the people in it are asked to rate it.
  // One tap on a star sends the rating and pays a little; a note can follow
  // it, and pays the rest of a note's reward.
  const ask = $derived(room?.feedback ?? null);
  let rating = $state(0);
  let note = $state('');
  let feedbackPending=$state(false),feedbackError=$state(''),feedbackNotice=$state('');
  let handledFeedback:FeedbackResult|null=null,feedbackTimer:ReturnType<typeof setTimeout>|undefined,noticeTimer:ReturnType<typeof setTimeout>|undefined;
  const beginFeedback=()=>{feedbackPending=true;feedbackError='';if(feedbackTimer)clearTimeout(feedbackTimer);feedbackTimer=setTimeout(()=>{feedbackPending=false;feedbackError='No save confirmation yet. Your text is still here; please try again.';},18000);};
  onDestroy(()=>{if(feedbackTimer)clearTimeout(feedbackTimer);if(noticeTimer)clearTimeout(noticeTimer);});
  $effect(()=>{
    if(!feedbackResult||feedbackResult===handledFeedback)return;
    handledFeedback=feedbackResult;
    if(feedbackResult.number!==askedAbout)return;
    feedbackPending=false;if(feedbackTimer)clearTimeout(feedbackTimer);
    if(feedbackResult.saved){
      feedbackError='';feedbackNotice=feedbackResult.with_note?'Note saved. Thank you!':'Rating saved. Thank you!';
      if(feedbackResult.rewarded)feedbackNotice+=feedbackResult.with_note?' +90 points':' +10 points';
      if(feedbackResult.with_note)skipped=[...skipped,feedbackResult.number];
      if(noticeTimer)clearTimeout(noticeTimer);noticeTimer=setTimeout(()=>{feedbackNotice='';},3500);
    }else feedbackError=feedbackResult.message??'Your feedback was not saved. Please try again.';
  });
  const stars = $derived(ask?.rated ?? rating);
  let skipped = $state<number[]>([]);
  let askedAbout = 0;
  $effect(() => {
    if (ask && ask.number !== askedAbout) {
      askedAbout = ask.number;
      rating = 0;
      note = '';feedbackPending=false;feedbackError='';if(feedbackTimer)clearTimeout(feedbackTimer);
    }
  });
  const rate = (star: number) => {
    if (ask?.rated||feedbackPending) return;
    beginFeedback();rating = star;
    send({ type: 'feedback', rating: star, note: '' });
  };
  const sendNote = () => {
    if (!note.trim()||feedbackPending) return;
    beginFeedback();
    send({ type: 'feedback', rating: stars, note: note.trim() });
  };
  const headline = $derived.by(() => {
    const winners = (placements[0] ?? []).map((seat) => bySeat(seat)?.name ?? 'Someone');
    if (!winners.length) return 'Nobody stayed on';
    if (winners.length === 1) return `${winners[0]} wins!`;
    return `${winners.slice(0, -1).join(', ')} and ${winners[winners.length - 1]} tie!`;
  });
  const ordinal = (n: number) => ['1st', '2nd', '3rd', '4th'][n] ?? `${n + 1}th`;
  const clock = (seconds: number) => `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;

  const recording = $derived(room?.recording ?? null);
  let now = $state(Date.now() / 1000);
  $effect(() => {
    if (!recording) return;
    const timer = setInterval(() => (now = Date.now() / 1000), 1000);
    return () => clearInterval(timer);
  });
  const recordedFor = $derived(recording ? clock(Math.max(0, Math.floor(now - recording.since))) : '');
  const canRecord = $derived(!!mine && !spectating && !guest);
  $effect(() => {
    if (phase !== 'practice' || !mine?.in_round || mine.ready) return;
    const ready = (event: KeyboardEvent) => {
      if (event.code !== 'Enter' || event.repeat) return;
      if(event.isComposing||isGameplayTyping(event.target))return;
      event.preventDefault();
      send({ type: 'ready' });
    };
    window.addEventListener('keydown', ready);
    return () => window.removeEventListener('keydown', ready);
  });

  const panel = 'rounded-2xl border border-white/15 bg-slate-900/80 text-white shadow-xl backdrop-blur';
</script>

<!-- While something is in hand in first person, the page's panels give way to
     the game's own overlay, so nothing sits under the mouse while you aim. -->
<svelte:window onkeydown={helpKey}/>
<div class="pointer-events-none absolute inset-0 select-none font-sans" class:in-hand={!!lobView || travelling || !!replayStatus} style="--hud-accent:{hudAccent}" bind:this={hudRoot}>
  {#if hookNotice && !replayStatus}
    <div class="hud-keep hook-result hook-result--{hookNotice.tone}" role="status" aria-live="polite">
      <span class="hook-result__heading">{hookNotice.heading}</span>
      <strong>{hookNotice.title}</strong>
      <span class="hook-result__detail">{hookNotice.detail}</span>
    </div>
  {/if}
  {#if (lobView || travelling) && !touch && !pointerLocked}
    <div class="hud-keep hud-prompt absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-20 items-center gap-2 px-4 py-2 text-xs font-bold uppercase tracking-wider text-white">
      <MousePointerClick class="size-4" /> Click to take aim
    </div>
  {/if}
  {#if travelling && touch}
    <div class="hud-keep pointer-events-auto absolute inset-y-24 right-0 left-1/3" role="application" aria-label="Drag to look while walking" onpointerdown={lookStart} onpointermove={lookMove} onpointerup={lookEnd} onpointercancel={lookEnd}></div>
    <button class="hud-keep pointer-events-auto absolute bottom-12 right-6 grid size-20 place-items-center rounded-full border-2 border-cyan-200 bg-cyan-950/70 text-xs font-black uppercase text-white" aria-label="Hold to vacuum pedestrians and pickups"
      onpointerdown={event=>{event.currentTarget.setPointerCapture(event.pointerId);onfire(true)}} onpointerup={()=>onfire(false)} onpointercancel={()=>onfire(false)}>Suction</button>
  {/if}
  {#if replayStatus}
    <div class="hud-keep replay-banner absolute top-4 left-1/2 -translate-x-1/2">
      <span class="replay-banner__tag">REPLAY</span>
      <strong>{replayStatus.label}</strong>
      <span>{Math.round(replayStatus.speed*100)}% SPEED</span>
      <div class="replay-progress"><i style="width:{replayStatus.progress*100}%"></i></div>
    </div>
    <button class="hud-keep pointer-events-auto absolute bottom-5 right-5 border border-white/25 bg-slate-950/70 px-4 py-2 text-xs font-bold uppercase tracking-widest text-white" onclick={onskipreplay}>Skip · Space</button>
  {/if}
  {#if inArena && !onBoard}
    <header class="match-hud absolute left-0 top-0 max-w-[min(36rem,calc(100%-7rem))] text-white">
      <div class="match-hud__title">
        <span class="match-hud__status {status === 'online' ? 'bg-emerald-400' : status === 'connecting' ? 'bg-amber-400' : 'bg-rose-500'}"></span>
        <span class="truncate">{board?.finale?.title ?? room?.minigame_name ?? 'Bout'}</span>
        {#if phase!=='playing'||showInstructions}<span class="match-hud__objective max-md:hidden">{objective}</span>{/if}
      </div>
      <ol class="match-hud__roster" aria-label="Players">
        {#if boutTeams.length}
          {#each boutTeams as team (team.id)}
            <li class="team-rail" class:team-rail--mine={myTeam===team.id} style={`--team-color:${team.color}`}>
              <span class="team-rail__name">{team.label}{#if myTeam===team.id}<b>YOUR TEAM</b>{/if}</span>
              {#each (room?.members ?? []).filter((member) => team.seats.includes(member.seat)) as member (member.user_id)}
                {@const character = byId.get(member.character)}
                {@const score = scoreOf(member.seat)}
                <span class:team-rail__member--out={score && !score.up} class:team-rail__member--me={member.user_id === me} class="team-rail__member">
                  <span class="match-hud__swatch" style="background:{team.color}; border-color:{team.color}"></span>
                  <span class="max-w-24 truncate">{member.name}</span>
                  {#if !score?.up}<Skull class="size-3" />{/if}
                </span>
              {/each}
            </li>
          {/each}
        {:else}
          {#each room?.members ?? [] as member (member.user_id)}
            {@const character = byId.get(member.character)}
            {@const score = scoreOf(member.seat)}
            <li
              class:match-hud__player--me={member.user_id === me}
              class:match-hud__player--out={score && !score.up}
              class="match-hud__player"
              style={visibleTeamFor(boutTeams,member.seat)?`box-shadow: inset 3px 0 0 ${visibleTeamFor(boutTeams,member.seat)!.color}`:undefined}
            >
              <span class="match-hud__swatch" style="background:{character?.body}; border-color:{character?.accent}"></span>
              <span class="max-w-28 truncate font-bold">{member.name}</span>
              {#if member.is_bot}<span class="text-[9px] font-black uppercase tracking-wider text-white/45">bot</span>{/if}
              {#if score}
                <output class="ml-auto tabular-nums text-amber-200">{score.hits}</output>
                {#if score.downs !== undefined}<Skull class="size-3 text-white/35" />{/if}
              {:else}
                <output class="ml-auto tabular-nums text-amber-200">{member.wins}</output>
              {/if}
            </li>
          {/each}
        {/if}
      </ol>
    </header>
  {:else if phase === 'lobby' && room}
    <header class="lobby-hud absolute left-0 top-0 text-white">
      <div class="lobby-hud__title">
        <span class="match-hud__status {status === 'online' ? 'bg-emerald-400' : status === 'connecting' ? 'bg-amber-400' : 'bg-rose-500'}"></span>
        <!-- Set up for a Tawkathon, the lobby is for the board, not the bout picked last. -->
        <span class="truncate">{partyMode ? `Tawkathon · ${(room.maps ?? []).find(([map]) => map === room.map)?.[1] ?? ''}` : room.minigame_name}</span>
        <span class="lobby-hud__state">LOBBY</span>
      </div>
      <div class="lobby-hud__players">
        <strong>{room.members.length}/{room.max_seats}</strong>
        {#each room.members as member (member.user_id)}
          {@const character = byId.get(member.character)}
          <span class="lobby-hud__player" title={member.name} aria-label={member.name} style="background:{character?.body}; border-color:{character?.accent}"></span>
        {/each}
        <span class="text-[10px] font-bold uppercase tracking-[0.12em] text-white/45">players ready</span>
      </div>
    </header>
  {:else if onBoard && board && room}
    <BoardStandings {board} members={listed} {roster} {me} teams={boardTeams} />
  {:else if cityMode&&room?.city&&(phase==='vote'||phase==='game_over')}
    <header class="match-hud absolute left-0 top-0 text-white" aria-label="City Heist score"><div class="match-hud__title"><Gamepad2 class="size-4 text-cyan-200"/>City Heist</div><div class="flex items-center gap-2 px-3 py-2 text-sm font-black tabular-nums"><Shield class="size-4 text-cyan-300"/>{room.city.scores[0]}<span class="mx-1 text-white/30">/</span>{room.city.scores[1]}<Banknote class="size-4 text-amber-200"/></div></header>
  {:else if (phase !== 'lobby'  && phase !== 'table' && phase !== 'map_finale' && phase !== 'city_heist' && !(tableGame && phase === 'game_over')) || !room}
  <div class="pointer-events-auto absolute left-3 top-3 w-60 {panel} p-3">
    <div class="flex items-center gap-2">
      <span class="size-2.5 rounded-full {status === 'online' ? 'bg-emerald-400' : status === 'connecting' ? 'bg-amber-400' : 'bg-rose-500'}"></span>
      <span class="text-sm font-bold tracking-wide">{board?.finale?.title ?? (onBoard ? 'Tawkathon' : (room?.minigame_name ?? 'Bout'))}</span>
    </div>
    <div class="mt-2 flex flex-col gap-1">
      {#each listed as member (member.user_id)}
        {@const character = byId.get(member.character)}
        {@const turnNumber = board ? board.order.indexOf(member.seat) + 1 : 0}
        {@const team=visibleTeamFor(onBoard?boardTeams:boutTeams,member.seat)}
        <div
          class="flex items-center gap-2 rounded-lg px-1.5 py-1 text-sm {member.connected ? '' : 'opacity-45'} {placements[0]?.includes(member.seat) ? 'bg-amber-300/20' : ''} {board && member.seat === board.turn_seat ? 'bg-white/15 ring-1 ring-white/40' : ''}"
        >
          {#if turnNumber > 0}<span class="w-3 text-center text-[11px] tabular-nums text-white/50">{turnNumber}</span>{/if}
          <span class="size-3.5 shrink-0 rounded-full border-2" style="background:{character?.body}; border-color:{character?.accent}"></span>
          {#if team}<span class="member-team" style={`--team-color:${team.color}`}>{team.label}</span>{/if}
          <span class="truncate {member.user_id === me ? 'font-bold' : ''}">{member.name}</span>
          {#if member.user_id === room?.host_id}<Crown class="size-3.5 shrink-0 text-amber-300" aria-label="Host" />{/if}
          {#if member.is_bot}<span class="rounded bg-white/15 px-1 text-[10px] font-semibold uppercase">Bot</span>{/if}
          {#if board}
            {@const token = board.tokens.find((t) => t.seat === member.seat)}
            {#if token}
              {#if token.grudge !== null && token.grudge !== undefined}
                {@const owed = bySeat(token.grudge)}
                <span
                  class="flex items-center gap-0.5 text-[11px] font-semibold text-rose-300"
                  title="{member.name} owes {owed?.name ?? 'someone'} one"
                >
                  <Swords class="size-3" />{owed?.name?.slice(0, 6) ?? ''}
                </span>
              {/if}
              {#if token.upgrades?.length}
                <span class="flex items-center gap-0.5 text-cyan-200" title="Vacuum: {token.upgrades.map((upgrade) => UPGRADES[upgrade].label).join(', ')}">
                  {#each token.upgrades as upgrade (upgrade)}
                    {@const Icon = UPGRADE_ICONS[upgrade]}
                    <Icon class="size-3" />
                  {/each}
                </span>
              {/if}
              <span class="ml-auto flex items-center gap-1 tabular-nums text-pink-300"><Candy class="size-3.5" />{token.prizes}</span>
              <span class="flex items-center gap-1 tabular-nums text-amber-200" title={token.coins === null ? 'Hidden by the fog' : undefined}><Coins class="size-3.5" />{token.coins ?? '?'}</span>
              <span class="flex items-center gap-0.5 tabular-nums text-rose-300"><Heart class="size-3.5" />{token.health}</span>
            {/if}
          {:else if firstPerson && (inArena || phase === 'results') && scoreOf(member.seat)}
            {@const score = scoreOf(member.seat)}
            <span class="ml-auto flex items-center gap-1 tabular-nums text-amber-200"><Target class="size-3.5" />{score?.hits ?? 0}</span>
            <span class="flex items-center gap-1 tabular-nums text-white/50"><Skull class="size-3.5" />{score?.downs ?? 0}</span>
          {:else}
            <span class="ml-auto flex items-center gap-1 tabular-nums text-amber-200"><Trophy class="size-3.5" />{member.wins}</span>
          {/if}
          {#if member.is_bot && isHost && phase === 'lobby'}
            <button class="rounded p-0.5 text-white/60 hover:bg-white/10 hover:text-white" aria-label="Remove {member.name}" onclick={() => send({ type: 'remove_bot', seat: member.seat })}>
              <X class="size-3.5" />
            </button>
          {/if}
        </div>
      {/each}
      {#if board && board.round === 1}
        <div class="mt-1 flex items-start gap-1.5 border-t border-white/10 pt-2 text-[11px] leading-snug text-white/70">
          <Candy class="mt-0.5 size-3.5 shrink-0 text-pink-300" />
          <span><strong class="block text-white">Collect the candy</strong>Most candy after {board.rounds} rounds wins. Vacuum in the floating candy to buy it for {myPrizeCost} coins.{#if board.underdog === mine?.seat}
            Yours is cheap this round.{/if}</span>
        </div>
      {/if}
      {#if !room}
        <div class="flex items-center gap-2 py-1 text-sm text-white/70">
          <LoaderCircle class="size-4 animate-spin" />
          {hasPass ? 'Joining the room' : 'Setting up the room'}
        </div>
      {/if}
    </div>
  </div>
  {/if}

  {#if !filming}
  <div class="hud-tools hud-keep pointer-events-auto absolute right-3 top-3 flex items-center gap-2">
    {#if room?.sandbox}
      {#if isHost}
        <button class="hud-control" style="width:auto;display:flex;gap:.3rem;padding:0 .65rem" aria-label="Exit sandbox" title="Return to character selection" onclick={() => send({type:'dev',tool:{action:'exit'}})}>
          <X class="size-4" /><span class="text-xs font-bold">Exit<span class="max-sm:hidden"> sandbox</span></span>
        </button>
      {/if}
      <button class="hud-control" aria-label="Manual events" title="Manual sandbox events" aria-expanded={showSandboxTools} onclick={() => (showSandboxTools = !showSandboxTools)}>
        <Gamepad2 class="size-5" />
      </button>
    {/if}
    {#if recording}
      <button
        class="{inArena || onBoard ? 'hud-control' : panel + ' px-3 py-2 hover:bg-slate-800/90'} flex items-center gap-2 text-sm font-bold"
        title={canRecord ? `Recording by ${recording.by}. Stop recording` : `Recording by ${recording.by}`}
        aria-label="Stop recording"
        disabled={!canRecord}
        onclick={() => send({ type: 'record', on: false })}
      >
        <span class="rec-dot size-2.5 rounded-full {recording.live ? 'bg-red-500' : 'bg-red-500/40'}"></span>
        {recording.live ? recordedFor : 'Starting'}
      </button>
    {:else if canRecord}
      <button
        class="{inArena || onBoard ? 'hud-control' : panel + ' p-2.5 hover:bg-slate-800/90'}"
        title="Record this game to the channel's files"
        aria-label="Record this game"
        onclick={() => send({ type: 'record', on: true })}
      >
        <CircleDot class="size-5 text-red-400" />
      </button>
    {/if}
    {#if inArena}<button class="hud-control" aria-label="Controls and objective" title="H · Controls and objective" onclick={()=>showInstructions=!showInstructions}><CircleHelp class="size-5"/></button>{/if}
    <button class="{inArena || onBoard ? 'hud-control' : panel + ' p-2.5 hover:bg-slate-800/90'}" aria-label="Settings" title="Settings" onclick={() => (showSettings = true)}>
      <Settings2 class="size-5" />
    </button>
    <button class="{inArena || onBoard ? 'hud-control' : panel + ' p-2.5 hover:bg-slate-800/90'}" aria-label={muted ? 'Turn sound on' : 'Turn sound off'} onclick={toggleMute}>
      {#if muted}<VolumeX class="size-5" />{:else}<Volume2 class="size-5" />{/if}
    </button>
    {#if onpopout}
      <button class="{inArena||onBoard?'hud-control':panel+' p-2.5 hover:bg-slate-800/90'}" aria-label={poppedOut?'Return to app':'Pop out game'} title={poppedOut?'Return to app':'Pop out game'} onclick={onpopout}>
        {#if poppedOut}<ArrowDownLeft class="size-5"/>{:else}<ExternalLink class="size-5"/>{/if}
      </button>
    {/if}
    <button
      class="{inArena || onBoard ? 'hud-control' : panel + ' p-2.5 hover:bg-slate-800/90'}"
      aria-label={fullscreen ? 'Leave fullscreen' : 'Play fullscreen'}
      title={fullscreen ? 'Leave fullscreen' : 'Play fullscreen'}
      onclick={toggleFullscreen}
    >
      {#if fullscreen}<Minimize2 class="size-5" />{:else}<Maximize2 class="size-5" />{/if}
    </button>
  </div>
  {/if}

  {#if timeLeft !== null}
    <div class="absolute left-1/2 top-3 -translate-x-1/2 text-center">
      <output class="{inArena ? 'hud-clock' : panel + ' px-5 py-1.5'} text-2xl font-black tabular-nums {timeLeft <= 10 ? 'hud-clock--danger text-rose-300' : ''}">{clock(timeLeft)}</output>
      {#if hud?.alert}<div class="mt-1 text-xs font-bold uppercase tracking-wider text-rose-200 drop-shadow">{hud.alert}</div>{/if}
    </div>
  {/if}

  {#if board && !board.candy_van && room?.phase.name === 'board'}
    <div class="absolute left-1/2 top-3 flex -translate-x-1/2 flex-col items-center gap-1">
      <div class="board-round">
        <span class="board-round__label">ROUND</span>
        <strong>{String(board.round).padStart(2, '0')}<span> / {String(board.rounds).padStart(2, '0')}</span></strong>
        <span class="board-round__turn">{myTurn ? 'YOUR TURN' : `${turnHolder}'s turn`}</span>
      </div>
      {#if event}
        <div class="{panel} whitespace-nowrap px-3 py-1 text-xs font-semibold text-amber-200" title={event.text}>{event.title}</div>
      {/if}
      {#if typeof board.tide === 'boolean'}
        <div class="{panel} whitespace-nowrap px-3 py-1 text-xs font-semibold text-sky-200" title={board.tide ? 'Stopping on a tide space costs coins while the water is in. It goes out next round.' : 'Tide spaces pay out while the water is out. Anyone still on one when it comes in next round pays for it.'}>
          {board.tide ? 'The tide is in' : 'The tide is out'}
        </div>
      {/if}
      {#if typeof board.gates_open_from === 'number'}
        <div class="board-gate-note {panel} whitespace-nowrap px-3 py-1 text-xs font-semibold text-sky-100" title="The rooftop shortcut opens in the last rounds. Once it opens, it is the quickest way across the city.">
          {board.gates_open ? 'The rooftop shortcut is open' : `The rooftop shortcut opens in round ${board.gates_open_from}`}
        </div>
      {/if}
      {#if board.storm !== null}
        <div class="{panel} whitespace-nowrap px-3 py-1 text-xs font-semibold text-slate-300" title="It moves on at the end of the round. Anyone it catches is blown back.">
          Storm on the board
        </div>
      {/if}
    </div>
    {#if event && bannerRound === board.round}
      <div class="party-pop absolute inset-x-0 top-1/3 flex flex-col items-center gap-2 text-center drop-shadow-[0_6px_0_rgba(15,23,42,0.55)]">
        <div class="text-6xl font-black text-amber-300">{event.title}</div>
        <div class="{panel} px-4 py-2 text-base font-semibold text-white">{event.text}</div>
      </div>
    {/if}

    {#if canShout && !myTurn}
      <div class="pointer-events-auto absolute bottom-3 left-3 flex flex-wrap gap-1.5">
        {#each TAUNTS as shout (shout.taunt)}
          <button
            class="rounded-lg bg-slate-900/70 px-2.5 py-1 text-xs font-semibold text-white/80 hover:bg-white/20 disabled:opacity-40"
            disabled={Date.now() - shoutedAt < 4000}
            onclick={() => {
              send({ type: 'taunt', taunt: shout.taunt });
              shoutedAt = Date.now();
            }}
          >
            {shout.label}
          </button>
        {/each}
      </div>
    {/if}

    {#if board.candy_van}
      <div class="absolute bottom-4 left-1/2 -translate-x-1/2 border border-pink-200/30 bg-slate-950/85 px-4 py-2 text-center text-xs font-bold text-pink-100">Candy van visit · board paused</div>
    {/if}
    {#if myTurn && !board.candy_van && !board.raids?.some(raid=>raid.seat===mine?.seat) && board.turn.step === 'done' && (!board.sucked || room?.sandbox)}
      <!-- Landed: a moment to take the vacuum out before the turn moves on. -->
      <div class="board-action pointer-events-auto absolute bottom-3 left-1/2 -translate-x-1/2 flex items-center gap-3 p-3" data-pad-menu>
        <span class="text-sm text-white/80">Anything worth vacuuming from here?</span>
        <button class="flex items-center gap-1.5 rounded-xl bg-cyan-500/30 px-3 py-2 text-sm font-bold text-cyan-50 hover:bg-cyan-500/40" onclick={() => send({ type: 'suck' })}>
          <Zap class="size-4" /> Vacuum <span class="text-xs font-normal opacity-70">V</span>
        </button>
      </div>
    {/if}

    {#if myTurn && !board.candy_van && !board.raids?.some(raid=>raid.seat===mine?.seat) && !['done','walking','driving'].includes(board.turn.step)}
      <div class="board-action pointer-events-auto absolute bottom-3 left-1/2 w-[min(42rem,calc(100%-1.5rem))] -translate-x-1/2 p-4" data-pad-menu>
        {#if board.turn.step === 'position' && board.positioning && !board.positioning.active}
          <PositionHud position={board.positioning} {send}/>
        {:else if board.turn.step === 'lob'}
          {@const throwing = lobView}
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="text-sm font-bold">{ITEMS[board.turn.item].label} in hand. Throw it!</div>
              <div class="mt-0.5 text-xs text-white/70">
                Move the mouse to aim, A and D to turn. Hold the mouse or Space and let go when the power is right.
              </div>
            </div>
            {#if throwing}
              <div class="flex shrink-0 items-center gap-3">
                <!-- The wind, as it blows across the way you face: pointing up, it carries the throw on. -->
                <div class="flex flex-col items-center text-[10px] font-semibold text-white/60" title="Wind">
                  <svg viewBox="-12 -12 24 24" class="size-8" style="transform: rotate({(-throwing.windAngle * 180) / Math.PI}deg)">
                    <path d="M0 -9 L5 2 L1.5 1 L1.5 9 L-1.5 9 L-1.5 1 L-5 2 Z" fill="#7dd3fc" opacity={0.35 + Math.min(1, throwing.wind / 3.5) * 0.65} />
                  </svg>
                  {throwing.wind.toFixed(1)}
                </div>
                <div class="text-center" title="Fuse">
                  <div class="text-lg font-black tabular-nums {throwing.fuseLeft < 3 ? 'text-rose-400' : 'text-amber-200'}">{throwing.fuseLeft.toFixed(1)}</div>
                  <div class="text-[10px] font-semibold text-white/60">fuse</div>
                </div>
              </div>
            {/if}
          </div>
          <div class="mt-3 h-4 overflow-hidden rounded-full bg-white/10" aria-label="Power">
            <div
              class="h-full rounded-full {throwing?.charging ? 'bg-gradient-to-r from-amber-300 to-rose-500' : 'bg-white/20'}"
              style="width:{(throwing?.charging ? throwing.power : 0) * 100}%"
            ></div>
          </div>
        {:else if board.turn.step === 'check'}
          {@const item = board.turn.item}
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="flex items-center gap-1.5 text-sm font-bold"><ItemIcon {item} /> {ITEMS[item].label}: stop the needle in the middle</div>
              <div class="mt-0.5 text-xs text-white/70">{CHECK_PROMPTS[item] ?? ''} Click Stop or press Space.</div>
            </div>
            <div class="text-center" title="Time left">
              <div class="text-lg font-black tabular-nums {checkLeft < 3 ? 'text-rose-400' : 'text-amber-200'}">{checkLeft.toFixed(1)}</div>
              <div class="text-[10px] font-semibold text-white/60">left</div>
            </div>
          </div>
          <div class="mt-3 flex items-center gap-3">
            <div class="relative h-5 flex-1 overflow-hidden rounded-full bg-white/10" aria-label="Skill check">
              <div class="absolute inset-y-0 left-[7.5%] right-[7.5%] bg-slate-400/30"></div>
              <div class="absolute inset-y-0 left-[25%] right-[25%] bg-orange-400/50"></div>
              <div class="absolute inset-y-0 left-[42.5%] right-[42.5%] bg-amber-300"></div>
              <div class="absolute inset-y-[-2px] w-1 -translate-x-1/2 rounded bg-white shadow" style="left:{checkNeedle * 100}%"></div>
            </div>
            <button class="rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300" onclick={stopCheck}>
              Stop <span class="text-xs font-normal opacity-70">Space</span>
            </button>
          </div>
        {:else if board.turn.step === 'swing'}
          {@const swinging = lobView}
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="text-sm font-bold">Bat in hand. Swing it!</div>
              <div class="mt-0.5 text-xs text-white/70">
                Turn with the mouse or A and D to face them. Click or press Space to stop the needle in the middle.
              </div>
            </div>
            {#if swinging}
              <div class="text-center" title="Time left">
                <div class="text-lg font-black tabular-nums {swinging.fuseLeft < 3 ? 'text-rose-400' : 'text-amber-200'}">{swinging.fuseLeft.toFixed(1)}</div>
                <div class="text-[10px] font-semibold text-white/60">left</div>
              </div>
            {/if}
          </div>
          <div class="relative mt-3 h-5 overflow-hidden rounded-full bg-white/10" aria-label="Swing timing">
            <!-- Weak, good and perfect, matching the board's scoring. -->
            <div class="absolute inset-y-0 left-[7.5%] right-[7.5%] bg-slate-400/30"></div>
            <div class="absolute inset-y-0 left-[25%] right-[25%] bg-orange-400/50"></div>
            <div class="absolute inset-y-0 left-[42.5%] right-[42.5%] bg-amber-300"></div>
            <div class="absolute inset-y-[-2px] w-1 -translate-x-1/2 rounded bg-white shadow" style="left:{(swinging?.power ?? 0) * 100}%"></div>
          </div>
        {:else if board.turn.step === 'cannon'}
          {@const max = board.turn.max}
          <div class="text-sm font-bold">Into the cannon. How hard?</div>
          <div class="mt-1 text-xs text-white/70">
            Pick the charge, then fire with the fuse needle in the middle to fly exactly that far. The further out, the more it strays.
          </div>
          <div class="mt-2 flex flex-wrap items-center gap-2">
            <input
              type="range"
              min="1"
              max={max}
              bind:value={charge}
              class="h-2 w-48 cursor-pointer appearance-none rounded-full bg-white/15 accent-orange-400"
            />
            <span class="w-24 text-sm font-bold tabular-nums text-orange-300">{charge} spaces</span>
          </div>
          <div class="mt-3 flex items-center gap-3">
            <div class="relative h-5 flex-1 overflow-hidden rounded-full bg-white/10" aria-label="Cannon fuse timing">
              <div class="absolute inset-y-0 left-[7.5%] right-[7.5%] bg-slate-400/30"></div>
              <div class="absolute inset-y-0 left-[25%] right-[25%] bg-orange-400/50"></div>
              <div class="absolute inset-y-0 left-[42.5%] right-[42.5%] bg-amber-300"></div>
              <div class="absolute inset-y-[-2px] w-1 -translate-x-1/2 rounded bg-white shadow" style="left:{fuseNeedle * 100}%"></div>
            </div>
            <button
              class="flex items-center gap-1.5 rounded-xl bg-orange-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-orange-300"
              onclick={fireCannon}
            >
              Fire <span class="text-xs font-normal opacity-70">Space</span>
            </button>
          </div>
        {:else if board.turn.step === 'shop'}
          <div class="text-sm font-bold">The shop is open. Buy one thing, or walk on.</div>
          <div class="mt-2 flex flex-wrap gap-2">
            {#each board.turn.offers as offer (offer.item)}
              {@const tooDear = (myToken?.coins ?? 0) < offer.price}
              <button
                class="flex items-center gap-2 rounded-xl px-3 py-2 text-sm font-semibold
                  {tooDear ? 'cursor-not-allowed bg-white/5 text-white/40' : 'bg-white/10 hover:bg-white/20'}"
                disabled={tooDear}
                title={ITEMS[offer.item].help}
                onclick={() => send({ type: 'shop', item: offer.item })}
              >
                <ItemIcon item={offer.item} />
                {ITEMS[offer.item].label}
                <span class="flex items-center gap-1 tabular-nums {tooDear ? 'text-rose-300/80' : 'text-amber-200'}">
                  <Coins class="size-3.5" />{offer.price}
                </span>
              </button>
            {/each}
            <button
              class="ml-auto rounded-xl px-3 py-2 text-sm font-semibold text-white/70 hover:text-white"
              onclick={() => send({ type: 'shop', item: null })}
            >
              Walk on
            </button>
          </div>
          {#if board.turn.upgrades?.length}
            <div class="mt-3 text-xs font-semibold uppercase tracking-wide text-cyan-200/80">Vacuum upgrades, kept for the game</div>
            <div class="mt-1.5 flex flex-wrap gap-2">
              {#each board.turn.upgrades as offer (offer.upgrade)}
                {@const tooDear = (myToken?.coins ?? 0) < offer.price}
                {@const Icon = UPGRADE_ICONS[offer.upgrade]}
                <button
                  class="flex items-center gap-2 rounded-xl px-3 py-2 text-sm font-semibold
                    {tooDear ? 'cursor-not-allowed bg-white/5 text-white/40' : 'bg-cyan-500/20 text-cyan-50 hover:bg-cyan-500/30'}"
                  disabled={tooDear}
                  title={UPGRADES[offer.upgrade].help}
                  onclick={() => send({ type: 'shop', item: null, upgrade: offer.upgrade })}
                >
                  <Icon class="size-4" />
                  {UPGRADES[offer.upgrade].label}
                  <span class="flex items-center gap-1 tabular-nums {tooDear ? 'text-rose-300/80' : 'text-amber-200'}">
                    <Coins class="size-3.5" />{offer.price}
                  </span>
                </button>
              {/each}
            </div>
          {/if}
          {#if myToken?.upgrades?.length}
            <div class="mt-2 text-xs text-white/60">
              Your vacuum has: {myToken.upgrades.map((upgrade) => UPGRADES[upgrade].label).join(', ')}
            </div>
          {/if}
        {:else if board.turn.step === 'branch'}
          <div class="text-sm font-bold">Choose your route · no U-turns. Follow the markers on the board.</div>
          <div class="mt-2 flex flex-wrap gap-2">
            {#each board.turn.options as option, index (option.node)}
              <button
                class="flex items-center gap-2 rounded-xl bg-white/10 px-4 py-2 text-sm font-semibold hover:bg-white/20"
                onclick={() => send({ type: 'choose_branch', node: option.node })}
              >
                <span class="flex size-6 items-center justify-center rounded-full bg-white text-sm font-black text-slate-900">
                  {BRANCH_LETTERS[index] ?? '?'}
                </span>
                <span class="size-3 rounded-full" style="background:{SPACE_COLORS[option.kind]}"></span>
                {routeLabel(option.node)}
                <span class="text-white/60">{SPACE_LABELS[option.kind]}</span>
              </button>
            {/each}
          </div>
        {:else}
          <div class="flex flex-wrap items-center gap-2">
            {#if board.turn.step === 'items'}
              {#each myToken?.items ?? [] as item, index (index)}
                {@const blocked = (item === 'double_dice' || item === 'golden_die') && diceSet}
                <button
                  class="flex items-center gap-1.5 rounded-xl px-3 py-2 text-sm font-semibold
                    {aiming === item ? 'bg-rose-500 text-white' : blocked ? 'cursor-not-allowed bg-white/5 text-white/40' : 'bg-white/10 hover:bg-white/20'}"
                  title={blocked ? 'You already changed your dice this turn.' : ITEMS[item].help}
                  disabled={blocked}
                  onclick={() => {
                    if (item === 'golden_die') {
                      aiming = aiming === item ? null : item;
                    } else {
                      send({ type: 'use_item', item, target: null });
                      aiming = null;
                    }
                  }}
                >
                  <ItemIcon item={item} />{ITEMS[item].label}
                </button>
              {/each}
              {#if myToken?.snowball}
                <button
                  class="flex items-center gap-1.5 rounded-xl bg-sky-400/30 px-3 py-2 text-sm font-semibold hover:bg-sky-400/40"
                  title={ITEMS.snowball.help}
                  onclick={() => send({ type: 'throw_snowball' })}
                >
                  <ItemIcon item="snowball" />Throw snowball
                </button>
              {/if}
              {#if !(myToken?.items ?? []).length && !myToken?.carrying && !myToken?.snowball}
                <span class="text-sm text-white/60">No items yet. Win a bout or land on a purple space to earn one.</span>
              {/if}
              {#if myToken?.carrying}
                <button
                  class="flex items-center gap-1.5 rounded-xl bg-amber-700/60 px-3 py-2 text-sm font-semibold hover:bg-amber-700/80"
                  title={ITEMS.crate.help}
                  onclick={() => send({ type: 'throw_crate' })}
                >
                  <ItemIcon item="crate" />Throw crate
                </button>
              {/if}
              <button
                class="flex items-center gap-1.5 rounded-xl px-3 py-2 text-sm font-semibold {board.sucked && !room?.sandbox ? 'cursor-not-allowed bg-white/5 text-white/40' : 'bg-cyan-500/20 text-cyan-100 hover:bg-cyan-500/30'}"
                title="Hold right mouse to take out the vacuum and suck; release to stow it. Hold F to blow. Aim at pedestrians, pickups or the candy."
                disabled={board.sucked && !room?.sandbox}
                onclick={() => send({ type: 'suck' })}
              >
                <Zap class="size-4" /> Vacuum <span class="text-xs font-normal opacity-70">V</span>
              </button>
            {/if}
            <button
              class="ml-auto flex items-center gap-1.5 rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300"
              onclick={() => {
                aiming = null;
                send({ type: 'roll' });
              }}
            >
              <Dices class="size-4" /> Roll
            </button>
          </div>
          {#if diceNote}
            <div class="mt-2 text-xs font-semibold text-amber-200">{diceNote}</div>
          {/if}
          {#if aiming === 'golden_die'}
            <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-white/10 pt-3">
              <span class="text-sm text-white/80">{ITEMS.golden_die.help}</span>
              {#each [1, 2, 3, 4, 5, 6] as face (face)}
                <button
                  class="flex size-9 items-center justify-center rounded-xl bg-white/10 text-sm font-black hover:bg-amber-300 hover:text-slate-900"
                  onclick={() => {
                    send({ type: 'use_item', item: 'golden_die', target: null, number: face });
                    aiming = null;
                  }}>{face}</button
                >
              {/each}
              <button class="rounded-xl px-3 py-1.5 text-sm text-white/70 hover:text-white" onclick={() => (aiming = null)}>Cancel</button>
            </div>
          {/if}
        {/if}
      </div>
    {/if}
  {/if}

  {#if firstPerson && inArena && mine?.in_round && !spectating}
    {#if combat?.up}
      <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
        <div class="relative size-8">
          <span class="absolute left-1/2 top-1/2 size-1 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white shadow"></span>
          {#each [[0, -14], [0, 14], [-14, 0], [14, 0]] as [dx, dy] (dx + ':' + dy)}
            <span
              class="absolute left-1/2 top-1/2 rounded-full bg-white/85 {dx === 0 ? 'h-2.5 w-0.5' : 'h-0.5 w-2.5'}"
              style="transform: translate(calc(-50% + {dx}px), calc(-50% + {dy}px))"
            ></span>
          {/each}
          {#if hud?.hitMarker}
            <span class="absolute inset-0 flex items-center justify-center text-2xl font-black text-rose-300">x</span>
          {/if}
        </div>
      </div>
    {:else if combat}
      <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 {panel} px-6 py-4 text-center">
        <div class="text-lg font-black text-rose-300">{combat.outTitle}</div>
        <div class="mt-1 text-sm text-white/80">
          {combat.back === null ? 'Watch your team finish the round' : `Back in ${Math.max(1, Math.ceil(combat.back))}`}
        </div>
      </div>
    {/if}

    {#if combat}
      <aside class="combat-hud absolute bottom-4 left-4 text-white">
        {#if combat.meter === 'score'}
          <div class="combat-hud__label"><Target class="size-4" /> SCORE</div>
          <output class="combat-hud__value text-emerald-200">{combat.hits}</output>
          <div class="combat-hud__hint">{combat.scoreHint ?? 'Find the lit target'}</div>
        {:else if combat.meter === 'health'}
          <div class="combat-hud__label"><Heart class="size-4 text-rose-300" /> HEALTH</div>
          <div class="flex items-end gap-3">
            <output class="combat-hud__value">{Math.max(0, combat.hp)}</output>
            <div class="mb-2 h-2 w-32 overflow-hidden bg-white/15 max-md:w-20">
              <div
                class="h-full transition-[width] {combat.hp > combat.hpMax / 3 ? 'bg-emerald-400' : 'bg-rose-400'}"
                style="width: {Math.max(0, (combat.hp / Math.max(1, combat.hpMax)) * 100)}%"
              ></div>
            </div>
          </div>
        {:else}
          <div class="combat-hud__label {combat.holding ? 'text-amber-200' : 'text-white/60'}">
            <CircleDot class="size-5" />
            {combat.holding ? 'BALL READY' : 'FIND A BALL'}
          </div>
        {/if}
        {#if combat.meter !== 'score'}
          <div class="combat-hud__stats">
            <span class="flex items-center gap-1.5 font-bold text-amber-200">
              <Target class="size-4" />{combat.hits} {combat.hitsLabel}
            </span>
            <span class="flex items-center gap-1.5 text-white/60"><Skull class="size-4" />{combat.downs}</span>
          </div>
        {/if}
      </aside>
    {/if}

    {#if touch}
      <div
        class="pointer-events-auto absolute bottom-0 right-0 top-0 w-1/2 touch-none"
        role="presentation"
        onpointerdown={lookStart}
        onpointermove={lookMove}
        onpointerup={lookEnd}
        onpointercancel={lookEnd}
      ></div>
      {#if hud?.crouch}
        <button
          class="pointer-events-auto absolute bottom-10 right-32 flex size-14 items-center justify-center rounded-full border-4 border-white/25 bg-slate-900/70 text-white active:bg-slate-700"
          aria-label="Crouch"
          onpointerdown={() => oncrouch(true)}
          onpointerup={() => oncrouch(false)}
          onpointercancel={() => oncrouch(false)}
        >
          <ChevronsDown class="size-7" />
        </button>
      {/if}
      <button
        class="pointer-events-auto absolute bottom-8 right-8 flex size-20 items-center justify-center rounded-full border-4 border-white/30 bg-rose-500/80 text-white active:bg-rose-400"
        aria-label={hud?.action ?? 'Fire'}
        onpointerdown={() => onfire(true)}
        onpointerup={() => onfire(false)}
        onpointercancel={() => onfire(false)}
      >
        <Zap class="size-8" />
      </button>
    {:else if !pointerLocked && phase === 'playing'}
      <div class="hud-prompt absolute left-1/2 top-1/2 flex -translate-x-1/2 translate-y-16 items-center gap-2 px-4 py-2 text-sm font-semibold text-white">
        <MousePointerClick class="size-4" />
        {hud?.hint ?? 'Click to take aim.'}
      </div>
    {/if}
  {/if}

  {#if board && mapBox && onBoard && !board.candy_van}
    <div class="board-radar pointer-events-auto absolute right-3 w-40 p-2 max-md:hidden">
      <svg viewBox="{mapBox.minX} {mapBox.minZ} {mapBox.w} {mapBox.h}" class="w-full" style="aspect-ratio: {mapBox.w} / {mapBox.h}" aria-label="Board map">
        {#each board.spaces as space (space.id)}
          {#each space.next as nextId (nextId)}
            {@const to = spaceAt(nextId)}
            {#if to && space.link !== nextId}
              <line x1={space.x} y1={space.z} x2={to.x} y2={to.z} stroke="rgba(255,255,255,0.28)" stroke-width="0.6" />
            {/if}
          {/each}
        {/each}
        {#each board.spaces as space (space.id)}
          <circle
            cx={space.x}
            cy={space.z}
            r={space.id === board.prize_at ? 1.6 : 1}
            fill={space.id === board.prize_at ? '#ff5fa2' : SPACE_COLORS[space.kind]}
            opacity={space.id === board.prize_at ? 1 : 0.8}
          />
        {/each}
        {#each board.tokens as token (token.seat)}
          {@const at = spaceAt(token.space)}
          {#if at}
            <circle
              cx={at.x}
              cy={at.z}
              r={token.seat === board.turn_seat ? 2.2 : 1.7}
              fill={byId.get(bySeat(token.seat)?.character ?? '')?.body ?? '#ffffff'}
              stroke={token.seat === board.turn_seat ? '#ffffff' : 'rgba(15,23,42,0.7)'}
              stroke-width="0.6"
            />
          {/if}
        {/each}
      </svg>
      <button class="mt-1.5 flex w-full items-center justify-center gap-1.5 border-t border-white/20 px-2 py-1.5 text-[10px] font-bold uppercase tracking-wider hover:bg-white/10" onclick={toggleOverview}>
        {#if overview}<Crosshair class="size-3.5" /> Follow the turn{:else}<Maximize2 class="size-3.5" /> Whole board{/if}
      </button>
    </div>
  {/if}

  {#if vote && (board||cityMode)}
    <div class="pointer-events-auto absolute bottom-3 left-1/2 w-[min(44rem,calc(100%-1.5rem))] -translate-x-1/2 {panel} p-4" data-pad-menu>
      <div class="flex items-baseline justify-between gap-3">
        <div class="text-lg font-black">{cityMode?'Next minigame':`Round ${board?.round} bout`}</div>
        <div class="text-sm tabular-nums text-white/70">{canVote && !myVote ? 'Pick one' : 'Waiting for votes'} · {Math.ceil(vote.seconds)}</div>
      </div>
      {#if sides}
        <div class="mt-2 flex flex-wrap items-center gap-2 text-sm">
          {#each sides as side, index (side.id)}
            {#if index > 0}<span class="text-white/40">vs</span>{/if}
            <span class="flex items-center gap-1.5 rounded-lg px-2 py-1 font-semibold" style="background:{side.color}22; color:{side.color}">
              <Users class="size-3.5" /><b>{side.label}</b> · {side.names.join(' and ')}
            </span>
          {/each}
        </div>
      {/if}
      {#if canVote && board}
        <div class="mt-2 flex flex-wrap items-center gap-2 text-sm">
          <span class="font-semibold text-white/80">Back yourself</span>
          {#each [0, 2, 5, board.max_bet] as stake (stake)}
            <button
              class="rounded-lg px-2.5 py-1 text-xs font-bold tabular-nums
                {myBet === stake ? 'bg-amber-400 text-slate-900' : 'bg-white/10 hover:bg-white/20'}"
              disabled={stake > (myToken?.coins ?? 0) + myBet}
              onclick={() => send({ type: 'bet', coins: stake })}
            >
              {stake === 0 ? 'Nothing' : stake}
            </button>
          {/each}
          {#if pot > 0}
            <span class="flex items-center gap-1 text-amber-200">
              <Coins class="size-3.5" />{pot} in the pot, to whoever wins
            </span>
          {/if}
        </div>
      {/if}
      <!-- The podiums over the island are the ballot; these are for the keyboard. -->
      <div class="mt-3 flex flex-wrap items-center gap-2 text-sm">
        <span class="text-white/60">{canVote ? cityMode?'Choose the next minigame:':'Click a podium, or:' : 'Votes so far:'}</span>
        {#each vote.options as key (key)}
          {@const voters = vote.votes.filter((v) => v.key === key)}
          <button
            class="flex items-center gap-1.5 rounded-lg border px-2.5 py-1 font-semibold disabled:cursor-default
              {myVote === key ? 'border-amber-300 bg-amber-400/20' : 'border-white/10 bg-white/5 enabled:hover:bg-white/10'}"
            disabled={!canVote}
            aria-pressed={myVote === key}
            onclick={() => send({ type: 'vote', key })}
          >
            {bouts.get(key)?.name ?? key}
            {#if voters.length}<span class="rounded-full bg-white/15 px-1.5 text-[11px] tabular-nums">{voters.length}</span>{/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if room?.phase.name==='game_over'&&cityMode&&room.city}
    <div class="absolute left-1/2 top-1/2 w-[min(30rem,calc(100%-2rem))] -translate-x-1/2 -translate-y-1/2 rounded-xl border-t-4 border-cyan-300 bg-slate-950/95 p-6 text-center shadow-2xl">
      <span class="text-xs font-black uppercase tracking-widest text-cyan-200">City Heist · Match complete</span>
      <h2 class="my-3 text-3xl font-black text-white">{room.city.scores[0]>=room.city.scores[1]?'Police win':'The crew wins'}</h2>
      <p class="text-lg font-bold text-amber-200">POLICE {room.city.scores[0]} — {room.city.scores[1]} CREW</p>
      <p class="mt-3 text-sm text-white/70">Returning to setup in {Math.ceil(room.phase.seconds)}s</p>
    </div>
  {/if}
  {#if room?.phase.name === 'game_over' && !tableGame&&!cityMode}
    {@const standings = room.phase.standings}
    {@const bonuses = board?.bonuses ?? []}
    {@const elapsed = FINALE_SECONDS - room.phase.seconds}
    {@const revealed = bonuses.length ? Math.min(bonuses.length, Math.floor(elapsed / BONUS_SECONDS) + 1) : 0}
    {@const podium = !bonuses.length || elapsed >= bonuses.length * BONUS_SECONDS + 1}
    <div class="absolute left-1/2 top-1/2 w-[min(34rem,calc(100%-1.5rem))] -translate-x-1/2 -translate-y-1/2 {panel} p-5 text-center">
      {#if !podium}
        <div class="text-2xl font-black text-amber-300">Bonus trophies</div>
        <div class="mt-1 text-sm text-white/70">Each one is worth candy.</div>
        <div class="mt-4 flex flex-col gap-2">
          {#each bonuses.slice(0, revealed) as award (award.bonus)}
            <div class="party-pop flex items-center gap-3 rounded-xl bg-white/10 px-4 py-3 text-left">
              <Trophy class="size-6 shrink-0 text-amber-300" />
              <div class="flex-1">
                <div class="font-bold">{BONUSES[award.bonus].title}</div>
                <div class="text-xs text-white/70">{BONUSES[award.bonus].text(award.count)}</div>
              </div>
              <div class="flex flex-col items-end gap-0.5 text-sm font-semibold">
                {#each award.seats as seat (seat)}
                  <span class="flex items-center gap-1.5">
                    <span class="size-3 rounded-full" style="background:{byId.get(bySeat(seat)?.character ?? '')?.body}"></span>
                    {bySeat(seat)?.name ?? 'Someone'}
                    <span class="flex items-center gap-0.5 text-pink-300"><Plus class="size-3" /><Candy class="size-3.5" /></span>
                  </span>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="party-pop text-3xl font-black text-amber-300">
          {(standings[0] ?? []).map((seat) => bySeat(seat)?.name ?? 'Someone').join(' and ')} {cityMode?'wins City Heist!':'wins the Tawkathon!'}
        </div>
        <div class="mt-5 flex items-end justify-center gap-2">
          {#each [1, 0, 2] as place (place)}
            {@const group = standings[place] ?? []}
            {#if group.length}
              <div class="party-pop flex w-32 flex-col items-center gap-1.5" style="animation-delay:{place === 0 ? 0.5 : place === 1 ? 0.25 : 0}s">
                {#each group as seat (seat)}
                  {@const token = board?.tokens.find((t) => t.seat === seat)}
                  <span class="size-7 rounded-full border-4" style="background:{byId.get(bySeat(seat)?.character ?? '')?.body}; border-color:{byId.get(bySeat(seat)?.character ?? '')?.accent}"></span>
                  <span class="w-full truncate text-sm font-bold">{bySeat(seat)?.name ?? 'Left the room'}</span>
                  <span class="flex items-center gap-2 text-xs">
                    <span class="flex items-center gap-0.5 tabular-nums text-pink-300"><Candy class="size-3" />{token?.prizes ?? 0}</span>
                    <span class="flex items-center gap-0.5 tabular-nums text-amber-200"><Coins class="size-3" />{token?.coins ?? 0}</span>
                  </span>
                {/each}
                <div
                  class="flex w-full items-start justify-center rounded-t-xl pt-2 text-2xl font-black text-slate-900
                    {place === 0 ? 'h-28 bg-amber-300' : place === 1 ? 'h-20 bg-slate-300' : 'h-14 bg-orange-300'}"
                >
                  {ordinal(place)}
                </div>
              </div>
            {/if}
          {/each}
        </div>
        {#if (board?.highlights ?? []).length}
          <div class="mt-4 flex flex-col gap-1 border-t border-white/10 pt-3 text-left text-xs">
            {#each board?.highlights ?? [] as award (award.highlight)}
              <div class="flex items-center gap-2">
                <span class="size-2.5 shrink-0 rounded-full" style="background:{byId.get(bySeat(award.seat)?.character ?? '')?.body}"></span>
                <span class="font-semibold">{bySeat(award.seat)?.name ?? 'Someone'}</span>
                <span class="text-white/70">{HIGHLIGHTS[award.highlight](award.value)}</span>
              </div>
            {/each}
          </div>
        {/if}
        {#if standings.length > 3}
          <div class="mt-3 flex flex-col gap-1 text-left text-sm">
            {#each standings.slice(3) as group, index (index)}
              {#each group as seat (seat)}
                {@const token = board?.tokens.find((t) => t.seat === seat)}
                <div class="flex items-center gap-2">
                  <span class="w-9 font-bold text-white/70">{ordinal(index + 3)}</span>
                  <span class="size-3 rounded-full" style="background:{byId.get(bySeat(seat)?.character ?? '')?.body}"></span>
                  <span class="flex-1 truncate">{bySeat(seat)?.name ?? 'Left the room'}</span>
                  <span class="flex items-center gap-1 tabular-nums text-pink-300"><Candy class="size-3.5" />{token?.prizes ?? 0}</span>
                  <span class="flex items-center gap-1 tabular-nums text-amber-200"><Coins class="size-3.5" />{token?.coins ?? 0}</span>
                </div>
              {/each}
            {/each}
          </div>
        {/if}
      {/if}
      <div class="mt-4 text-xs text-white/60">Back to the lobby in {Math.ceil(room.phase.seconds)}</div>
    </div>
  {/if}

  {#if status === 'offline' && room}
    <div class="absolute left-1/2 top-16 -translate-x-1/2 {panel} flex items-center gap-2 px-4 py-2 text-sm">
      <LoaderCircle class="size-4 animate-spin" /> Reconnecting
    </div>
  {/if}

  {#if errorMessage}
    <div class="absolute left-1/2 top-28 -translate-x-1/2 rounded-xl bg-rose-600/90 px-4 py-2 text-sm font-semibold text-white shadow-lg">{errorMessage}</div>
  {/if}

  {#if sides && (phase === 'countdown' || phase === 'playing')}
    <div class="absolute left-1/2 top-16 flex -translate-x-1/2 items-center gap-2 text-sm font-bold">
      {#each sides as side, index (side.id)}
        {#if index > 0}<span class="text-white/50">vs</span>{/if}
        <span
          class="{panel} flex items-center gap-1.5 px-3 py-1"
          style="color:{side.color}{myTeam === side.id ? '; outline: 2px solid ' + side.color : ''}"
        >
          <Users class="size-3.5" /><strong>{side.label}</strong>{#if myTeam===side.id}<b class="your-team-tag">YOUR TEAM</b>{/if}
        </span>
      {/each}
    </div>
  {/if}

  {#if room?.phase.name === 'practice'}
    <!-- The practice go: it does not count, and ends when everyone is ready. -->
    <div class="practice-strip pointer-events-auto absolute left-1/2 top-3 -translate-x-1/2">
      <span class="text-[10px] font-black uppercase tracking-[0.14em] text-emerald-200">Warmup</span>
      <span class="text-xs font-bold tabular-nums text-white/70">{Math.ceil(room.phase.seconds)}s</span>
      {#if mine?.in_round}
        <button
          class="px-2 py-1 text-[11px] font-black uppercase tracking-wider {mine.ready ? 'text-white/50' : 'bg-emerald-400 text-slate-900 hover:bg-emerald-300'}"
          disabled={mine.ready}
          onclick={() => send({ type: 'ready' })}
        >
          {mine.ready ? 'Ready' : touch ? "I'm ready" : 'Enter · Ready'}
        </button>
      {/if}
    </div>
  {/if}

  {#if (room?.phase.name === 'countdown' || room?.phase.name === 'practice') && mine?.in_round && !briefHidden}
    <!-- How to play, while the bout counts in: the keys as this player has them bound. -->
    {@const kind = bouts.get(room.minigame)?.controls ?? 'move'}
    {@const keys = settings().keys}
    {@const cap = (code: string | undefined) => (code ? keyName(code) : '—')}
    <div class="game-brief absolute left-1/2 top-16 w-[min(30rem,calc(100%-2rem))] -translate-x-1/2 p-4">
      <div class="text-[11px] font-bold uppercase tracking-[0.16em] text-white/50">{room.opening_minigame?'Opening minigame · decide turn order':'How to play'}</div>
      <div class="mt-0.5 text-lg font-black">{room.minigame_name}</div>
      <div class="text-sm text-amber-200">{objective}</div>
      <div class="mt-3 flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-white/80">
        <span class="flex items-center gap-1.5">
          <span class="flex gap-0.5">
            {#each [keys.up[0], keys.left[0], keys.down[0], keys.right[0]] as code, index (index)}
              <kbd class="rounded border border-white/25 bg-white/10 px-1.5 py-0.5 font-mono text-[11px] font-bold">{cap(code)}</kbd>
            {/each}
          </span>
          Move
        </span>
        {#if kind === 'move_action'}
          <span class="flex items-center gap-1.5"><kbd class="rounded border border-white/25 bg-white/10 px-1.5 py-0.5 font-mono text-[11px] font-bold">{cap(keys.action[0])}</kbd> or click · Action</span>
        {:else if kind === 'move_aim'}
          <span class="flex items-center gap-1.5"><kbd class="rounded border border-white/25 bg-white/10 px-1.5 py-0.5 text-[11px] font-bold">Mouse</kbd> Aim</span>
          <span class="flex items-center gap-1.5"><kbd class="rounded border border-white/25 bg-white/10 px-1.5 py-0.5 text-[11px] font-bold">Click</kbd> Fire</span>
        {:else if kind === 'first_person'}
          <span class="flex items-center gap-1.5"><kbd class="rounded border border-white/25 bg-white/10 px-1.5 py-0.5 text-[11px] font-bold">Mouse</kbd> Look</span>
          <span class="flex items-center gap-1.5"><kbd class="rounded border border-white/25 bg-white/10 px-1.5 py-0.5 text-[11px] font-bold">Click</kbd> Shoot</span>
          <span class="flex items-center gap-1.5"><kbd class="rounded border border-white/25 bg-white/10 px-1.5 py-0.5 font-mono text-[11px] font-bold">{cap(keys.jump[0])}</kbd> Jump</span>
          <span class="flex items-center gap-1.5"><kbd class="rounded border border-white/25 bg-white/10 px-1.5 py-0.5 font-mono text-[11px] font-bold">{cap(keys.crouch[0])}</kbd> Crouch</span>
        {/if}
      </div>
      {#if hud?.hint}
        <div class="mt-2 text-xs text-white/60">{hud.hint}</div>
      {/if}
      {#if gamepad()}
        <div class="mt-1 text-[11px] text-white/45">Controller: left stick to move{kind === 'move_action' ? ', A to act' : ''}{kind === 'first_person' ? ', right stick to look' : ''}.</div>
      {/if}
    </div>
  {/if}

  {#if room?.phase.name === 'countdown'}
    {#key room.phase.seconds}
      <div class="party-pop absolute inset-0 flex items-center justify-center text-[9rem] font-black text-white drop-shadow-[0_6px_0_rgba(15,23,42,0.55)]">
        {Math.ceil(room.phase.seconds)}
      </div>
    {/key}
  {:else if showGo}
    <div class="party-pop absolute inset-0 flex items-center justify-center text-[8rem] font-black text-amber-300 drop-shadow-[0_6px_0_rgba(15,23,42,0.55)]">Go!</div>
  {/if}

  {#if room?.phase.name === 'results' && !replayStatus}
    <div class="absolute left-1/2 top-3 w-72 -translate-x-1/2 {panel} p-4 text-center max-md:top-auto max-md:bottom-3">
      <div class="text-3xl font-black text-amber-300">{headline}</div>
      {#if room.opening_minigame}<div class="mt-1 text-xs text-amber-200">Starting coins &amp; first turn order</div>{/if}
      {#if board?.finale}<div class="mt-1 text-[10px] font-bold uppercase tracking-[0.13em] text-cyan-200">{board.finale.title}</div>{/if}
      <div class="mt-3 flex flex-col gap-1 text-left text-sm">
        {#each placements as group, place}
          {#each group as seat (seat)}
            {@const member = bySeat(seat)}
            {@const reward=room.minigame_rewards?.find(reward=>reward.seat===seat)}
            <div class="flex items-center gap-2">
              <span class="w-9 font-bold text-white/70">{ordinal(reward?reward.place-1:place)}</span>
              <span class="size-3 rounded-full" style="background:{byId.get(member?.character ?? '')?.body}"></span>
              <span class="truncate">{member?.name ?? 'Left the room'}</span>
              {#if reward}<span class="ml-auto shrink-0 text-xs text-amber-200">{reward.coins>0?`+${reward.coins} coins`:'Item draw'}</span>{/if}
            </div>
            {#if reward}<div class="ml-11 mb-1 text-[10px] text-white/50">Item lottery · {Math.round(reward.tier_one_chance*100)}% tier 1</div>{/if}
          {/each}
        {/each}
      </div>
      <div class="mt-3 text-xs text-white/60">{cityMode?'Next heist':board ? 'Back to the board' : 'Back to the lobby'} in {Math.ceil(room.phase.seconds)}</div>
    </div>
  {/if}

  {#if ask && !skipped.includes(ask.number) && !spectating}
    <div class="pointer-events-auto absolute left-3 top-1/2 z-20 w-72 -translate-y-1/2 {panel} p-4 max-md:top-auto max-md:bottom-28 max-md:translate-y-0" data-pad-menu>
      <div class="text-[11px] font-bold uppercase tracking-[0.16em] text-white/50">How was it?</div>
      <div class="mt-0.5 text-lg font-black">{ask.name}</div>
      <div class="mt-2 flex gap-1" role="radiogroup" aria-label="Rating">
        {#each [1, 2, 3, 4, 5] as star (star)}
          <button
            class="rounded-lg p-1 transition hover:bg-white/10"
            role="radio"
            aria-checked={stars === star}
            aria-label="{star} of 5"
            disabled={!!ask.rated||feedbackPending}
            onclick={() => rate(star)}
          >
            <Star class="size-6 {star <= stars ? 'fill-amber-300 text-amber-300' : 'text-white/40'}" />
          </button>
        {/each}
      </div>
      {#if !ask.rated}
        <div class="mt-2 flex items-center justify-between gap-2">
          <span class="text-xs text-white/50">{feedbackPending?'Saving rating…':'Tap a star · +10 points'}</span>
          <button class="rounded-lg px-3 py-1.5 text-sm text-white/60 hover:bg-white/10 hover:text-white" onclick={() => (skipped = [...skipped, ask.number])}>
            Skip
          </button>
        </div>
      {:else}
        <div class="mt-1 text-xs font-semibold text-amber-200">{feedbackResult?.number===ask.number&&feedbackResult.rewarded?'Rating saved · +10 points':'Rating saved. Thank you!'}</div>
        <!-- Keys typed here are words, not the game's controls. -->
        <textarea
          class="mt-2 h-16 w-full resize-none rounded-lg border border-white/10 bg-white/5 p-2 text-sm placeholder:text-white/40 focus:border-amber-300/60 focus:outline-none"
          maxlength="500"
          placeholder="Want to tell us more? (optional)"
          bind:value={note}
          onkeydown={(event) => event.stopPropagation()}
        ></textarea>
        <div class="mt-2 flex items-center justify-between gap-2">
          <button class="rounded-lg px-3 py-1.5 text-sm text-white/60 hover:bg-white/10 hover:text-white" onclick={() => (skipped = [...skipped, ask.number])}>
            Done
          </button>
          <button
            class="rounded-lg bg-amber-300 px-3 py-1.5 text-sm font-bold text-slate-950 transition hover:bg-amber-200 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={!note.trim()||feedbackPending}
            onclick={sendNote}
          >
            {feedbackPending?'Saving note…':'Send note · +90 points'}
          </button>
        </div>
      {/if}
      {#if feedbackError}<p class="mt-2 text-xs text-rose-200" role="alert">{feedbackError}</p>{/if}
    </div>
  {/if}
  {#if feedbackNotice}<div class="hud-keep absolute left-1/2 top-24 z-50 -translate-x-1/2 rounded-lg border border-emerald-300/40 bg-slate-950/95 px-4 py-2 text-xs font-bold text-emerald-200" role="status">{feedbackNotice}</div>{/if}

  {#if showSettings}
    <SettingsPanel onclose={() => (showSettings = false)} />
  {/if}

  {#if room?.sandbox && showSandboxTools}
    <!-- The sandbox's tools: items on tap, coins, hearts and jumping about. -->
    <div class="pointer-events-auto absolute right-3 top-16 z-20 w-64 {panel} p-3 text-sm">
      <div class="flex items-center justify-between">
        <span class="text-[11px] font-bold uppercase tracking-[0.16em] text-fuchsia-300">Manual events</span>
        <button class="rounded-lg px-2 py-1 text-xs text-white/60 hover:bg-white/10 hover:text-white" onclick={() => send({ type: 'dev', tool: { action: 'exit' } })}>
          Leave
        </button>
      </div>
      <div class="mt-2 text-xs text-white/60">Give yourself an item (the bag holds 5):</div>
      <div class="mt-1 grid grid-cols-4 gap-1">
        {#each Object.keys(ITEMS).filter((key) => key !== 'crate' && key !== 'snowball') as key (key)}
          {@const item = key as Item}
          <button
            class="flex flex-col items-center gap-0.5 rounded-lg bg-white/5 p-1.5 text-[10px] leading-tight hover:bg-white/15"
            title={ITEMS[item].help}
            onclick={() => send({ type: 'dev', tool: { action: 'give', item } })}
          >
            <ItemIcon {item} />
            <span class="truncate">{ITEMS[item].label}</span>
          </button>
        {/each}
      </div>
      <div class="mt-2 flex flex-wrap gap-1">
        <button class="rounded-lg bg-cyan-500/20 px-2 py-1 text-xs font-semibold text-cyan-100 hover:bg-cyan-500/30" onclick={() => {send({type:'dev',tool:{action:'finale'}});showSandboxTools=false;}}>Play map finale</button>
        {#if isHost && room.dev_tools}
          <button class="rounded-lg bg-lime-500/20 px-2 py-1 text-xs font-semibold text-lime-100 hover:bg-lime-500/30" onclick={()=>{send({type:'dev',tool:{action:'crawler'}});showSandboxTools=false;}}>{board?.crawler?.controller===mine?.seat?'Return to player':'Control zombie'}</button>
          {#if board?.map==='frozen'}<button class="rounded-lg bg-pink-500/20 px-2 py-1 text-xs font-semibold text-pink-100 hover:bg-pink-500/30" onclick={()=>{send({type:'dev',tool:{action:'candy_van'}});showSandboxTools=false;}}>Summon candy van</button>{/if}
        {/if}
        <button class="rounded-lg bg-white/10 px-2 py-1 text-xs font-semibold hover:bg-white/20" onclick={() => send({ type: 'dev', tool: { action: 'coins', coins: 50 } })}>+50 coins</button>
        <button class="rounded-lg bg-white/10 px-2 py-1 text-xs font-semibold hover:bg-white/20" onclick={() => send({ type: 'dev', tool: { action: 'heal' } })}>Full hearts</button>
        <button class="rounded-lg bg-white/10 px-2 py-1 text-xs font-semibold hover:bg-white/20" onclick={() => send({ type: 'dev', tool: { action: 'prize' } })}>+1 star</button>
        <button class="rounded-lg bg-white/10 px-2 py-1 text-xs font-semibold hover:bg-white/20" title="The zombie catches you and knocks you out" onclick={() => send({ type: 'dev', tool: { action: 'zombie_kill' } })}>Zombie KO me</button>
        <button class="rounded-lg bg-white/10 px-2 py-1 text-xs font-semibold hover:bg-white/20" title="The zombie catches the next player and knocks them out" onclick={() => send({ type: 'dev', tool: { action: 'zombie_kill', seat: ((board?.tokens.find((t) => t.seat !== board?.turn_seat)?.seat) ?? 1) } })}>Zombie KO a bot</button>
        <button
          class="rounded-lg px-2 py-1 text-xs font-semibold {room.mature ? 'bg-rose-500 text-white' : 'bg-white/10 hover:bg-white/20'}"
          title="Cartoon blood and knockout effects"
          onclick={() => send({ type: 'choose_mature', on: !room.mature })}
        >
          Mature {room.mature ? 'on' : 'off'}
        </button>
        <button
          class="rounded-lg px-2 py-1 text-xs font-semibold {teleporting ? 'bg-fuchsia-500 text-white' : 'bg-white/10 hover:bg-white/20'}"
          onclick={() => onteleport(!teleporting)}
        >
          {teleporting ? 'Click a space…' : 'Jump to a space'}
        </button>

      </div>
      <div class="mt-2 text-[11px] leading-snug text-white/50">Drag anyone, bots too, onto a space to move them there.</div>

    </div>
  {/if}

  {#if room && phase === 'lobby'}
    <div class="lobby-panel pointer-events-auto absolute bottom-3 left-1/2 w-[min(48rem,calc(100%-1.5rem))] -translate-x-1/2 {panel} p-4" data-pad-menu>
      <!-- Everyone in the room, lined up over the panel. -->
      <ul class="mb-3 flex flex-wrap justify-center gap-2" aria-label="Players">
        {#each room.members as member (member.user_id)}
          {@const character = byId.get(member.character)}
          <li class="flex items-center gap-2 rounded-full border border-white/15 bg-slate-900/80 py-1 pr-3 pl-1 text-sm font-semibold text-white shadow-lg backdrop-blur">
            <span class="size-7 rounded-full border-4" style="background:{character?.body}; border-color:{character?.accent}"></span>
            <span class="max-w-28 truncate">{member.name}</span>
            {#if member.user_id === room.host_id}<Crown class="size-4 text-amber-300" aria-label="Host" />{/if}
            {#if member.is_bot}<Bot class="size-4 text-white/60" aria-label="Bot" />{/if}
            {#if member.user_id === me}<span class="rounded-full bg-amber-300/20 px-1.5 text-[10px] font-bold uppercase text-amber-200">You</span>{/if}
          </li>
        {/each}
      </ul>
      {#if filming}
        <div class="text-center text-sm font-semibold text-white/80">Waiting for the next game</div>
      {:else if spectating || !mine}
        <div class="flex items-center gap-2 text-sm text-white/80">
          <Eye class="size-4" /> The room is full. You are watching and get a seat when one opens.
        </div>
      {:else if needsAvatar}
        <div class="flex flex-col items-center gap-3 py-4 text-center text-white">
          <div class="text-lg font-black">Set up your Tawkathon avatar</div>
          <p class="max-w-sm text-sm text-white/70">Pick a colour and an outfit before your first game. It follows you into every game and onto your profile card.</p>
          <button class="rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300" onclick={() => wardrobeApi?.open?.()}>Set up avatar</button>
        </div>
      {:else}
        <div class="flex items-center gap-3 rounded-xl bg-white/5 p-2">
          <div class="h-20 w-16 shrink-0"><CharacterPreview character={myLook} compact hoverReaction /></div>
          <div class="min-w-0 flex-1">
            <div class="text-sm font-semibold text-white">Your avatar</div>
            <div class="text-xs text-white/60">{guest ? 'Sign in to choose and save your look.' : 'Saved in Tawks settings · the same in every game'}</div>
          </div>
          {#if !guest && wardrobeApi?.open}
            <button class="rounded-xl bg-white/10 px-3 py-2 text-sm font-semibold text-white hover:bg-white/20" onclick={() => wardrobeApi?.open?.()}>Edit avatar</button>
          {/if}
        </div>
        <div class="mt-3 flex items-center gap-2 border-t border-white/10 pt-3">
          {#if isHost}
            <div class="flex rounded-xl bg-white/10 p-1 text-sm font-semibold" role="radiogroup" aria-label="What to play">
              {#each [['party', 'Tawkathon'], ['city', 'City Heist'], ['single', 'Single game']] as const as [mode, label] (label)}
                <button
                  class="rounded-lg px-3 py-1 {(mode==='city'?cityMode:!cityMode&&partyMode===(mode==='party')) ? 'bg-amber-400 text-slate-900' : 'hover:bg-white/10'}"
                  role="radio"
                  aria-checked={mode==='city'?cityMode:!cityMode&&partyMode===(mode==='party')}
                  onclick={() => send(mode==='city'?{type:'choose_city',...cityConfig}:{type:'choose_mode',party:mode==='party'})}
                >
                  {label}
                </button>
              {/each}
            </div>
          {:else}
            <span class="text-sm font-semibold">{cityMode?'City Heist':partyMode ? 'A Tawkathon' : 'A single game'}</span>
          {/if}
          <span class="text-xs text-white/50">{cityMode?'A shared nighttime city. Rob the bank or stop the crew.':partyMode ? 'The board below, with a bout voted on each round.' : 'One bout or card game on its own.'}</span>
        </div>
        {#if cityMode}
          <section class="mt-3 grid gap-3 rounded-xl border border-cyan-300/20 bg-slate-950/70 p-3 text-sm" aria-label="City Heist setup">
            <div class="flex flex-wrap items-center gap-3"><strong>Police</strong>{#each Array.from({length:Math.max(1,room.members.length-1)},(_,i)=>i+1) as police}<button class="rounded-lg px-3 py-1 {cityConfig.police===police?'bg-cyan-300 text-slate-950':'bg-white/10'}" disabled={!isHost} onclick={()=>send({type:'choose_city',...cityConfig,police})}>{police}</button>{/each}<span class="text-xs text-white/50">Everyone else joins the robber crew</span></div>
            <div class="flex flex-wrap items-center gap-3"><strong>Nights</strong>{#each [1,3,5] as jobs}<button class="rounded-lg px-3 py-1 {cityConfig.jobs===jobs?'bg-amber-300 text-slate-950':'bg-white/10'}" disabled={!isHost} onclick={()=>send({type:'choose_city',...cityConfig,jobs})}>{jobs}</button>{/each}</div>
            {#if isHost}<div class="flex flex-wrap items-center gap-3"><strong>My role</strong>{#each [[false,'Robber'],[true,'Police']] as const as [host_police,label]}<button class="rounded-lg px-3 py-1 {cityConfig.host_police===host_police?'bg-amber-300 text-slate-950':'bg-white/10'}" onclick={()=>send({type:'choose_city',...cityConfig,host_police})}>{label}</button>{/each}</div>
            {#if people<=1}<label class="flex items-center gap-2"><input type="checkbox" bind:checked={cityWatch}/> Watch all AI · take control whenever you like</label>{/if}{/if}
            <p class="text-xs leading-relaxed text-white/60">Six minutes per night, then a minigame. Deliver bags or stop jobs to score. Night one includes a role briefing; solo play adds three bots. Cops patrol by car and pursue on foot.</p>
          </section>
        {/if}
        {#if partyMode}
          <div class="mt-3 flex flex-wrap items-center gap-3 border-t border-white/10 pt-3">
            <span class="text-sm font-semibold">Rounds</span>
            {#if isHost}
              <div class="flex gap-1 rounded-lg bg-white/10 p-1" role="radiogroup" aria-label="Number of rounds">
                {#each [10,15,20,25,30] as rounds (rounds)}
                  <button class="rounded-md px-3 py-1 text-sm font-bold {(room.rounds??10)===rounds?'bg-amber-400 text-slate-950':'hover:bg-white/10'}" role="radio" aria-checked={(room.rounds??10)===rounds} onclick={()=>send({type:'choose_rounds',rounds})}>{rounds}</button>
                {/each}
              </div>
            {:else}<strong class="text-amber-200">{room.rounds??10}</strong>{/if}
            <span class="text-xs text-white/50">Start at random spots. An opening minigame sets turn order and pays 25–10 starting coins. Later winners get 10 coins; everyone draws an item.</span>
          </div>
          <div class="mt-3 flex items-center gap-3 border-t border-white/10 pt-3">
            {#if isHost}
              <button
                class="relative h-6 w-11 shrink-0 rounded-full transition {room.mature ? 'bg-rose-500' : 'bg-white/15'}"
                role="switch"
                aria-checked={!!room.mature}
                aria-label="Mature"
                onclick={() => send({ type: 'choose_mature', on: !room.mature })}
              >
                <span class="absolute top-1 size-4 rounded-full bg-white transition-all {room.mature ? 'left-6' : 'left-1'}"></span>
              </button>
            {/if}
            <div class="text-sm">
              <span class="font-semibold">Mature {room.mature ? 'on' : 'off'}</span>
              <span class="text-xs text-white/50"> · cartoon blood and knockout effects.</span>
            </div>
          </div>
          <div class="mt-3 flex items-center gap-3 border-t border-white/10 pt-3">
            {#if isHost}
              <button
                class="relative h-6 w-11 shrink-0 rounded-full transition {room.pairs_on ? 'bg-cyan-500' : 'bg-white/15'}"
                role="switch"
                aria-checked={!!room.pairs_on}
                aria-label="Pairs"
                onclick={() => send({ type: 'choose_pairs', on: !room.pairs_on })}
              >
                <span class="absolute top-1 size-4 rounded-full bg-white transition-all {room.pairs_on ? 'left-6' : 'left-1'}"></span>
              </button>
            {/if}
            <div class="text-sm">
              <span class="font-semibold">Pairs {room.pairs_on ? 'on' : 'off'}</span>
              <span class="text-xs text-white/50">
                · dealt into pairs at the start (four or more players): partners share each bout's place and never hurt each other.
              </span>
            </div>
          </div>
        {/if}
        {#if isHost && !partyMode && !cityMode && allGames.length > 1 && gameChoice}
          <div class="mt-3 border-t border-white/10 pt-3">
            <div class="mb-1 text-[11px] font-bold uppercase tracking-[0.16em] text-white/50">Tonight's bout</div>
            <div class="flex items-center gap-2 rounded-2xl border border-white/10 bg-gradient-to-r from-cyan-400/15 via-indigo-500/15 to-fuchsia-500/15 p-2">
              <button class="grid size-10 shrink-0 place-items-center rounded-xl bg-slate-950/35 hover:bg-white/10" aria-label="Previous bout" onclick={() => cycleGame(-1)}><ChevronLeft class="size-5" /></button>
              <button class="min-w-0 flex-1 rounded-xl px-2 py-1 text-left hover:bg-white/10" onclick={() => send({ type: 'choose_game', key: gameChoice[0] })}>
                <div class="flex items-center gap-2 text-base font-black"><Gamepad2 class="size-4 text-amber-300" />{gameChoice[1]}</div>
                <div class="mt-0.5 truncate text-xs text-white/65">{gameCopy(gameChoice[0])}</div>
              </button>
              <button class="grid size-10 shrink-0 place-items-center rounded-xl bg-slate-950/35 hover:bg-white/10" aria-label="Next bout" onclick={() => cycleGame(1)}><ChevronRight class="size-5" /></button>
            </div>
            <div class="mt-1.5 text-center text-xs text-white/50">{tableGame ? 'A card game at one table. People make it; bots only fill chairs.' : 'Use the arrows to browse.'}</div>
          </div>
        {/if}
        {#if partyMode && (room.maps ?? []).length > 1}
          <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-white/10 pt-3">
            <span class="text-sm text-white/70">Board</span>
            {#each room.maps as [map, label] (map)}
              <button
                class="rounded-xl px-3 py-1.5 text-sm font-semibold
                  {room.map === map ? 'bg-amber-400 text-slate-900' : 'bg-white/10 enabled:hover:bg-white/20 disabled:opacity-50'}"
                disabled={!isHost}
                onclick={() => send({ type: 'choose_map', map })}
              >
                {label}
              </button>
            {/each}
          </div>
        {/if}
        {#if isHost && (room.skills ?? []).length > 1}
          <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-white/10 pt-3">
            <span class="text-sm text-white/70">{cityMode?'Minigame bots':'Bots'}</span>
            {#each room.skills as [level] (level)}
              <button
                class="rounded-xl px-3 py-1.5 text-sm font-semibold capitalize
                  {room.bot_skill_name === level ? 'bg-amber-400 text-slate-900' : 'bg-white/10 hover:bg-white/20'}"
                onclick={() => send({ type: 'choose_skill', level })}
              >
                {level}
              </button>
            {/each}
          </div>
        {/if}
        <div class="mt-3 flex flex-wrap items-center gap-2">
          {#if isHost && tableGame}
            <button
              class="flex items-center gap-1.5 rounded-xl bg-white/10 px-3 py-2 text-sm font-semibold hover:bg-white/20 disabled:opacity-40"
              disabled={seatsFull}
              onclick={() => send({ type: 'add_bot' })}
            >
              <Plus class="size-4" /><Bot class="size-4" /> Add a bot
            </button>
            <button
              class="ml-auto flex items-center gap-1.5 rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300 disabled:opacity-40"
              disabled={room.members.length < tableMin}
              onclick={() => send({ type: 'start' })}
            >
              <Play class="size-4" /> Deal the cards
            </button>
            {#if room.members.length < tableMin}
              <div class="w-full text-xs text-white/60">{room.members.length} of {tableMin} seats filled. Bots can fill in, but they play simply.</div>
            {/if}
          {:else if isHost}
            <button
              class="flex items-center gap-1.5 rounded-xl bg-white/10 px-3 py-2 text-sm font-semibold hover:bg-white/20 disabled:opacity-40"
              disabled={seatsFull}
              onclick={() => send({ type: 'add_bot' })}
            >
              <Plus class="size-4" /><Bot class="size-4" /> Add a bot
            </button>
            {#if room.dev_tools&&!cityMode}
              <button
                class="flex items-center gap-1.5 rounded-xl bg-fuchsia-500/20 px-3 py-2 text-sm font-semibold text-fuchsia-200 hover:bg-fuchsia-500/30"
                title="Dev stack only: the board with no bouts, bots that stand still, and every item on tap."
                onclick={() => send({ type: 'start_sandbox' })}
              >
                <Zap class="size-4" /> Sandbox
              </button>
            {/if}
            <button
              class="ml-auto flex items-center gap-1.5 rounded-xl bg-amber-400 px-4 py-2 text-sm font-bold text-slate-900 hover:bg-amber-300 disabled:opacity-40"
              disabled={!cityMode&&room.members.length < 2}
              onclick={() => send(cityMode?{type:'start_city',watch:cityWatch}:{ type: partyMode ? 'start_party' : 'start' })}
            >
              {#if cityMode}<Play class="size-4" /> {cityWatch?'Watch AI heist':'Start City Heist'}{:else if partyMode}<Dices class="size-4" /> Start a Tawkathon{:else}<Play class="size-4" /> Start this bout{/if}
            </button>
            {#if !cityMode&&room.members.length < 2}
              <div class="w-full text-xs text-white/60">Invite someone into the lobby or add a bot to start.</div>
            {/if}
          {:else}
            <div class="text-sm text-white/70">Waiting for {host?.name ?? 'the host'} to start the round.</div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if inArena && mine?.in_round && !spectating && (hud?.meter || (showInstructions&&firstPerson&&combat))}
    <footer class="match-objective absolute left-1/2 -translate-x-1/2 {hud?.touch === 'pump' && touch ? 'bottom-28' : 'bottom-4'}">
      {#if hud?.meter}
        <span>{hud.meter.label}</span>
        <div class="objective-meter"><i style="width:{hud.meter.progress * 100}%"></i></div>
        {hud.meter.text}
      {:else}
        <span>OBJECTIVE</span>{objective}
      {/if}
    </footer>
  {/if}

  {#if touch&&!spectating&&mine&&(phase==='map_finale'||board?.crawler?.controller===mine.seat||board?.positioning?.seat===mine.seat&&!board.positioning.active||board?.raids?.some(raid=>raid.seat===mine.seat)||board?.driving?.some(driver=>driver.seat===mine.seat))}
    <div class="pointer-events-auto absolute bottom-24 left-5 z-50"><Joystick onchange={onstick}/></div>
  {/if}
  {#if mine?.in_round && inArena}
    {#if hud?.touch === 'pump' && touch}
      <div class="pointer-events-auto absolute bottom-5 left-4 right-4 flex gap-3">
        <button class="flex h-20 flex-1 items-center justify-center rounded-2xl border-2 border-white/25 bg-indigo-500/85 text-lg font-black text-white active:bg-indigo-400" aria-label="Pump left" onpointerdown={() => pump(-1)}>LEFT</button>
        <button class="flex h-20 flex-1 items-center justify-center rounded-2xl border-2 border-white/25 bg-fuchsia-500/85 text-lg font-black text-white active:bg-fuchsia-400" aria-label="Pump right" onpointerdown={() => pump(1)}>RIGHT</button>
      </div>
    {:else if touch}
      {#if hud?.touch !== 'none'}
        <div class="pointer-events-auto absolute bottom-6 left-6"><Joystick onchange={onstick} /></div>
      {/if}
    {:else if !firstPerson && hud?.hint&&(phase!=='playing'||showInstructions)}
      <div class="absolute {hud.meter ? 'bottom-14' : 'bottom-4'} left-1/2 -translate-x-1/2 rounded-full bg-slate-900/60 px-4 py-1.5 text-xs font-semibold text-white/85">
        {hud.hint}
      </div>
    {/if}
  {/if}
</div>

<style>
.lobby-panel{max-height:calc(100% - 5rem);overflow-y:auto;scrollbar-width:thin;}

  .hook-result {
    position:absolute;left:50%;top:22%;transform:translateX(-50%);
    display:flex;flex-direction:column;align-items:center;gap:0.3rem;
    width:min(32rem,calc(100% - 2rem));padding:1rem 1.5rem;
    border-top:2px solid var(--hook-color);border-bottom:1px solid color-mix(in srgb,var(--hook-color),transparent 65%);
    background:linear-gradient(90deg,transparent,rgba(2,6,23,0.84) 16%,rgba(2,6,23,0.84) 84%,transparent);
    color:white;text-align:center;text-shadow:0 2px 6px #020617;
    animation:hook-arrival 180ms ease-out;
  }
  .hook-result--gold {--hook-color:#fcd34d;}
  .hook-result--cyan {--hook-color:#67e8f9;}
  .hook-result--muted {--hook-color:#cbd5e1;}
  .hook-result__heading {font-size:0.65rem;font-weight:900;letter-spacing:0.2em;color:var(--hook-color);}
  .hook-result strong {font-size:clamp(1.35rem,4vw,2.25rem);font-weight:950;font-style:italic;line-height:1.15;}
  .hook-result__detail {font-size:0.8rem;color:#e2e8f0;}
  @keyframes hook-arrival {from{opacity:0;translate:0 0.35rem;}to{opacity:1;translate:0 0;}}
  .replay-banner{display:flex;align-items:center;gap:.7rem;flex-wrap:wrap;padding:.5rem .8rem;min-width:15rem;max-width:calc(100% - 2rem);background:linear-gradient(90deg,#040c18ed,#040c1866);border-top:2px solid #fcd34d;color:white;filter:drop-shadow(0 3px 8px #0005)}
  .replay-banner__tag{font-size:.6rem;letter-spacing:.18em;color:#fcd34d;font-weight:950}.replay-banner strong{font-size:.7rem}.replay-banner>span:last-of-type{margin-left:auto;font-size:.5rem;font-variant-numeric:tabular-nums;letter-spacing:.08em;color:#ffffff99}.replay-progress{width:100%;height:2px;background:#ffffff20}.replay-progress i{display:block;height:100%;background:#fcd34d}

  .practice-strip {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.4rem 0.75rem;
    border-top: 2px solid #34d399;
    color: white;
    background: linear-gradient(180deg, rgb(4 12 24 / 0.85), rgb(4 12 24 / 0.35));
    white-space: nowrap;
  }
  .game-brief {
    border-left: 3px solid var(--hud-accent);
    color: white;
    background: linear-gradient(100deg, rgb(4 12 24 / 0.92), rgb(4 12 24 / 0.55));
    filter: drop-shadow(0 4px 12px rgb(0 0 0 / 0.2));
  }
  .board-round {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 10rem;
    padding: 0.5rem 1.6rem 0.65rem;
    border-top: 2px solid #fcd34d;
    color: white;
    background: linear-gradient(180deg, rgb(4 12 24 / 0.85), rgb(4 12 24 / 0.05));
    clip-path: polygon(0.6rem 0, calc(100% - 0.6rem) 0, 100% 100%, 0 100%);
    text-shadow: 0 2px 4px rgb(0 0 0 / 0.4);
  }
  .board-round__label { color: #fde68a; font-size: 0.5rem; font-weight: 900; letter-spacing: 0.25em; }
  .board-round strong { font-size: 1.65rem; font-weight: 950; font-style: italic; line-height: 1.2; font-variant-numeric: tabular-nums; }
  .board-round strong span { color: rgb(255 255 255 / 0.4); font-size: 0.85rem; }
  .board-round__turn { margin-top: 0.25rem; font-size: 0.6rem; font-weight: 850; text-transform: uppercase; letter-spacing: 0.06em; }
  .board-action {
    border-top: 2px solid #fcd34d;
    color: white;
    background: linear-gradient(180deg, rgb(4 12 24 / 0.94), rgb(4 12 24 / 0.75));
    box-shadow: 0 10px 25px rgb(0 0 0 / 0.2);
    clip-path: polygon(0.6rem 0, calc(100% - 0.6rem) 0, 100% 0.6rem, 100% 100%, 0 100%, 0 0.6rem);
  }
  .board-action :global(button) { border-radius: 0.2rem; }
  @media(max-width:767px){.board-radar{display:none}}
  @media(max-width:640px){.board-gate-note{display:none}}
  .board-radar {
    top:50%;transform:translateY(-50%);
    color: white;
    border-bottom: 1px solid rgb(255 255 255 / 0.2);
    background: radial-gradient(ellipse, rgb(4 12 24 / 0.6), rgb(4 12 24 / 0.1) 70%, transparent);
    filter: drop-shadow(0 2px 4px rgb(0 0 0 / 0.4));
  }
  /* Where a controller has moved to in a menu. */
  [data-pad-menu] :global(button:focus) {
    outline: 3px solid #fcd34d;
    outline-offset: 2px;
  }
  .in-hand > :global(*:not(.hud-keep)) {
    display: none !important;
  }
  .rec-dot {
    animation: rec-pulse 1.6s ease-in-out infinite;
  }

  @keyframes rec-pulse {
    50% { opacity: 0.35; }
  }

  .lobby-hud {
    filter: drop-shadow(0 7px 12px rgb(2 6 23 / 0.28));
  }
  .lobby-hud__title {
    display: flex;
    min-width: 15rem;
    align-items: center;
    gap: 0.55rem;
    padding: 0.55rem 2rem 0.55rem 0.85rem;
    border-bottom: 1px solid var(--hud-accent);
    background: linear-gradient(100deg, rgb(8 15 29 / 0.94), rgb(15 23 42 / 0.78));
    clip-path: polygon(0 0, 100% 0, calc(100% - 1rem) 100%, 0 100%);
    font-size: 0.76rem;
    font-weight: 900;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }
  .lobby-hud__state {
    margin-left: auto;
    color: var(--hud-accent);
    font-size: 0.55rem;
    letter-spacing: 0.16em;
  }
  .lobby-hud__players {
    display: flex;
    width: fit-content;
    align-items: center;
    gap: 0.45rem;
    padding: 0.4rem 1.35rem 0.4rem 0.85rem;
    background: linear-gradient(90deg, rgb(8 15 29 / 0.86), rgb(15 23 42 / 0.62), transparent);
    font-size: 0.7rem;
  }
  .lobby-hud__player {
    width: 0.65rem;
    height: 0.65rem;
    border: 2px solid;
    transform: rotate(45deg);
  }
  .lobby-panel {
    max-height: calc(100% - 5.5rem);
    overflow-y: auto;
  }
  .character-select { display: grid; grid-template-columns: minmax(0, 0.8fr) minmax(0, 1.2fr); align-items: center; gap: 1rem; }
  @media (max-width: 640px) { .character-select { grid-template-columns: minmax(0, 0.65fr) minmax(0, 1.35fr); gap: 0.35rem; } }
  .match-hud {
    filter: drop-shadow(0 8px 14px rgb(2 6 23 / 0.28));
  }
  .match-hud__title {
    display: flex;
    min-width: 16rem;
    align-items: center;
    gap: 0.55rem;
    padding: 0.55rem 2.2rem 0.55rem 0.9rem;
    background: linear-gradient(100deg, rgb(8 15 29 / 0.94), rgb(15 23 42 / 0.82));
    border-bottom: 1px solid color-mix(in srgb, var(--hud-accent) 75%, transparent);
    clip-path: polygon(0 0, 100% 0, calc(100% - 1.2rem) 100%, 0 100%);
    font-size: 0.78rem;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .match-hud__status {
    width: 0.5rem;
    height: 0.5rem;
    flex: none;
    box-shadow: 0 0 0.7rem currentColor;
  }
  .match-hud__objective {
    overflow: hidden;
    margin-left: 0.25rem;
    color: rgb(255 255 255 / 0.48);
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .match-hud__roster {
    display: grid;
    width: min(28rem, calc(100vw - 8rem));
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
    background: rgb(255 255 255 / 0.08);
  }
  .match-hud__player {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.45rem;
    padding: 0.35rem 0.7rem;
    background: linear-gradient(90deg, rgb(8 15 29 / 0.86), rgb(15 23 42 / 0.72));
    border-bottom: 1px solid transparent;
    font-size: 0.7rem;
  }
  .match-hud__player--me {
    border-bottom-color: var(--hud-accent);
    background: linear-gradient(90deg, color-mix(in srgb, var(--hud-accent) 18%, rgb(8 15 29 / 0.9)), rgb(15 23 42 / 0.72));
  }
  .match-hud__player--out {
    filter: grayscale(0.8);
    opacity: 0.42;
  }
  .match-hud__swatch {
    width: 0.58rem;
    height: 0.58rem;
    flex: none;
    border: 2px solid;
    transform: rotate(45deg);
  }
  .team-rail {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.5rem;
    grid-column: 1 / -1;
    min-height: 2rem;
    padding: 0.3rem 0.7rem;
    background: linear-gradient(90deg, rgb(8 15 29 / 0.92), rgb(15 23 42 / 0.72));
    font-size: 0.68rem;
  }
  .team-rail {border-left:3px solid var(--team-color);flex-wrap:wrap;}
  .team-rail--mine {background:linear-gradient(90deg,color-mix(in srgb,var(--team-color) 18%,#080f1b),#0f172ab8);box-shadow:inset 0 1px var(--team-color),inset 0 -1px color-mix(in srgb,var(--team-color) 35%,transparent);}
  .team-rail__name {
    min-width: 4.6rem;
    flex: none;
    color: var(--team-color);
    font-size: 0.58rem;
    font-weight: 950;
    letter-spacing: 0.13em;
  }
  .team-rail__member {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.35rem;
    font-weight: 750;
  }
  .team-rail__name>b{display:block;color:#fff;font-size:.42rem;letter-spacing:.06em;margin-top:.2rem}.team-rail__member--me{color:#fff;text-decoration:underline;text-decoration-color:var(--team-color);text-underline-offset:.22rem}.member-team{display:inline-flex;padding:.12rem .3rem;border:1px solid var(--team-color);color:var(--team-color);border-radius:.2rem;font-size:.48rem;white-space:nowrap}.your-team-tag{font-size:.5rem;letter-spacing:.05em;}
  .team-rail__member--out { filter: grayscale(1); opacity: 0.35; text-decoration: line-through; }
  .hud-control {
    display: grid;
    width: 2.65rem;
    height: 2.65rem;
    place-items: center;
    color: white;
    border: 1px solid rgb(255 255 255 / 0.16);
    background: linear-gradient(145deg, rgb(15 23 42 / 0.92), rgb(8 15 29 / 0.76));
    clip-path: polygon(0.45rem 0, 100% 0, 100% calc(100% - 0.45rem), calc(100% - 0.45rem) 100%, 0 100%, 0 0.45rem);
    filter: drop-shadow(0 4px 8px rgb(2 6 23 / 0.3));
  }
  .hud-control:hover {
    color: var(--hud-accent);
    border-color: color-mix(in srgb, var(--hud-accent) 55%, transparent);
  }
  .hud-clock {
    display: block;
    min-width: 6.5rem;
    padding: 0.28rem 1.35rem 0.42rem;
    color: white;
    border-top: 2px solid var(--hud-accent);
    background: linear-gradient(180deg, rgb(8 15 29 / 0.94), rgb(15 23 42 / 0.76));
    clip-path: polygon(0.7rem 0, calc(100% - 0.7rem) 0, 100% 100%, 0 100%);
    filter: drop-shadow(0 6px 8px rgb(2 6 23 / 0.3));
  }
  .hud-clock--danger {
    border-top-color: #fb7185;
    animation: hud-alert 0.8s ease-in-out infinite alternate;
  }
  .combat-hud {
    min-width: 13rem;
    padding: 0.55rem 1.8rem 0.6rem 0.8rem;
    border-left: 3px solid var(--hud-accent);
    background: linear-gradient(90deg, rgb(8 15 29 / 0.9), rgb(8 15 29 / 0.55) 72%, transparent);
    filter: drop-shadow(0 5px 10px rgb(2 6 23 / 0.3));
  }
  .combat-hud__label {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    color: rgb(255 255 255 / 0.62);
    font-size: 0.62rem;
    font-weight: 900;
    letter-spacing: 0.16em;
  }
  .combat-hud__value {
    display: block;
    font-size: 2.45rem;
    font-weight: 950;
    line-height: 1;
    letter-spacing: -0.06em;
    text-shadow: 0 2px 10px rgb(0 0 0 / 0.35);
  }
  .combat-hud__hint {
    margin-top: 0.15rem;
    color: rgb(255 255 255 / 0.62);
    font-size: 0.66rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .combat-hud__stats {
    display: flex;
    gap: 1rem;
    margin-top: 0.2rem;
    font-size: 0.7rem;
  }
  .match-objective {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.35rem 1.2rem;
    color: rgb(255 255 255 / 0.72);
    border-bottom: 1px solid rgb(255 255 255 / 0.15);
    background: linear-gradient(90deg, transparent, rgb(8 15 29 / 0.78) 15%, rgb(8 15 29 / 0.78) 85%, transparent);
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    white-space: nowrap;
  }
  .match-objective span {
    color: var(--hud-accent);
    font-weight: 950;
    letter-spacing: 0.14em;
  }
  .objective-meter {
    width: 7rem;
    height: 0.32rem;
    overflow: hidden;
    background: rgb(255 255 255 / 0.14);
  }
  .objective-meter i {
    display: block;
    height: 100%;
    background: #22d3ee;
    box-shadow: 0 0 0.6rem #22d3ee;
    transition: width 100ms linear;
  }
  .hud-prompt {
    border-top: 1px solid var(--hud-accent);
    background: linear-gradient(90deg, transparent, rgb(8 15 29 / 0.88) 12%, rgb(8 15 29 / 0.88) 88%, transparent);
    white-space: nowrap;
  }
  .party-pop {
    animation: party-pop 0.6s cubic-bezier(0.2, 1.6, 0.4, 1) both;
  }
  @keyframes party-pop {
    from {
      transform: scale(0.3);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }
  @keyframes hud-alert {
    to { filter: drop-shadow(0 0 0.6rem rgb(251 113 133 / 0.58)); }
  }
  @media (max-width: 640px) {
    .hud-tools { display: grid; grid-template-columns: repeat(2, 1.8rem); gap: 0.25rem; top: 0.65rem; right: 0.65rem; }
    .hud-tools :global(button) { width: 1.8rem; height: 1.8rem; }
    .hud-tools :global(svg) { width: 0.9rem; height: 0.9rem; }
    .board-round { min-width: 5.5rem; padding: 0.4rem 0.55rem; }
    .board-round strong { font-size: 1.3rem; }
    .board-round strong span { font-size: 0.65rem; }
    .board-round__turn { max-width: 6rem; font-size: 0.5rem; text-align: center; }
    .match-hud__title {
      min-width: 10rem;
      padding-left: 0.65rem;
      font-size: 0.68rem;
    }
    .match-hud__roster {
      width: min(19rem, calc(100vw - 7rem));
      grid-template-columns: 1fr;
    }
    .match-hud__player:nth-child(n + 4) {
      display: none;
    }
    .combat-hud {
      bottom: 0.65rem;
      left: 0.65rem;
      min-width: 10rem;
    }
    .combat-hud__value { font-size: 2rem; }
    .hud-clock {
      min-width: 5rem;
      font-size: 1.15rem;
    }
    .match-objective {
      bottom: auto;
      top: 7.25rem;
      max-width: calc(100vw - 1rem);
      padding-inline: 0.7rem;
      font-size: 0.55rem;
    }
    .objective-meter { width: 4.5rem; }
    .lobby-hud__title { min-width: 12rem; }
    .lobby-hud__players { max-width: calc(100vw - 6.5rem); }
  }
  @media (max-height: 760px) {
    .lobby-panel {
      bottom: 0.5rem;
      padding: 0.65rem;
    }
    :global(.lobby-panel button) { padding-block: 0.35rem; }
    :global(.lobby-panel .mt-3) { margin-top: 0.45rem; }
  }
</style>

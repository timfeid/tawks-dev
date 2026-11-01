<script lang="ts">
  import { getContext, onMount } from 'svelte';
  import * as THREE from 'three';
  import type { BoutFrame, BoutHost, BoutHud, BoutModule, BoutPhase, BoutRenderer } from '@tawks/bout-sdk';
  import type { WorkflowResource } from '@tawks/workflow-types';
  import type { WorkflowStateProvider } from '@tawks/workflow-ui/workflow';
  import { submitServerAction } from '@tawks/workflow-ui/components/workflow';
  import { SceneManager } from './battlefield/engine/SceneManager';
  import { PartyConnection, type ConnectionStatus } from './party/connection';
  import type { ReplayStatus } from './party/replay';
  import type { CharacterDef, ClientMsg, CovenView, GameEvent, Item, PartyPass, RoomView } from './party/protocol';
  import { BAT, PULL, pullKey, VACUUM, bombAt, bombFlight } from './party/protocol';
  import { inputDevice, padButton } from './party/input-device';
  import { ThrowHud } from './party/board/throw-hud';
  import { VoteStage } from './party/board/vote-stage';
  import { bound, gamepad, keyName, settings } from './party/settings';
  import { ITEMS } from './party/board/items';
  import { createPartyStage, WATER_Y } from './party/stage';
  import { buildCharacter } from './party/characters';
  import { loadBout } from './party/bouts/registry';
  import { ShooterControls } from './party/shooter/controls';
  import { PartyBoard } from './party/board/scene';
  import { CameraFlow } from './party/camera-flow';
  import { FrameBudget } from './party/frame-budget';
  import { isGameplayTyping } from './party/typing';
  import { CovenTable } from './party/coven/table';
  import CovenHud from './party/coven/coven-hud.svelte';
  import { LandlordTable } from './party/landlord/table';
  import LandlordHud from './party/landlord/landlord-hud.svelte';
  import type { LandlordView } from './party/landlord/rules';
  import { PartyInput, type StickValue } from './party/input';
  import { PartySfx } from './party/sfx';
  import RoomHud from './party/room-hud.svelte';
  import RaidHud from './party/raid-hud.svelte';
  import DrivingHud from './party/driving-hud.svelte';
  import ZombieControlHud from './party/zombie-control-hud.svelte';
  import CandyVanHud from './party/candy-van-hud.svelte';
  import FinaleHud from './party/finale-hud.svelte';
  import CityHud from './party/city/hud.svelte';
  import{CityHeistScene}from'./party/city/scene';
  import { observePartyViewport } from './party/viewport';

  let {
    workflow,
    watchPass = null,
    parked=false,
    poppedOut=false,
    onpopout,
  }: {
    workflow?: WorkflowResource;
    /** A recorder's watch-only pass, in place of joining through a workflow. */
    watchPass?: PartyPass | null;
    parked?:boolean;
    poppedOut?:boolean;
    onpopout?:()=>void;
  } = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');
  const filming = $derived(!!watchPass);

  const pass = $derived(watchPass ?? (workflow?.responses?.party as PartyPass | null) ?? null);

  let canvas=$state<HTMLCanvasElement>(null!);
  let root=$state<HTMLDivElement>(null!);
  let candyBearing=$state(0);
  let cityHeld=$state(false);
  let citySuction=$state(false);
  let cityYaw=$state(0);
  let candyJoining=false;
  let captureCandy:(()=>void)|null=null;
  // Raw: each message brings a whole new room, and the frame loop reads it
  // every frame, which through a deep proxy cost a fifth of the main thread.
  let room = $state.raw<RoomView | null>(null);
  let roster = $state<CharacterDef[]>([]);
  let me = $state<string | null>(null);
  let spectating = $state(false);
  let guest = $state(false);
  let status = $state<ConnectionStatus>('connecting');
  let feedbackResult=$state<import('./party/protocol').FeedbackResult|null>(null);
  let errorMessage = $state<string | null>(null);
  let timeLeft = $state<number | null>(null);
  let showGo = $state(false);
  let muted = $state(false);
  let touch = $state(false);
  let overview = $state(false);
  /** A tip for whatever on the board is under the pointer. */
  let tip = $state<{ x: number; y: number; title: string; body: string } | null>(null);
  let aimItem = $state<Item | null>(null);
  /** Throwing a bomb in first person, for the HUD: the power swing, the fuse,
   * and the wind as an angle from the way you face (0 is behind you, pushing). */
  let lobView = $state<{ power: number; charging: boolean; fuseLeft: number; wind: number; windAngle: number } | null>(null);
  /** The sandbox's free camera, and a click on a space jumping you there. */
  let teleporting = $state(false);
  let pointerLocked = $state(false);
  /** The browser hands back a lock the page released without a click, so a lock alone does not mean the player has started the bout. */
  let clickedIntoBout = $state(false);
  const briefing = $derived(room?.phase.name === 'countdown' || room?.phase.name === 'practice');
  const briefHidden = $derived(pointerLocked && clickedIntoBout);
  let hud = $state<BoutHud | null>(null);
  let replayStatus=$state<ReplayStatus|null>(null);
  let mySeatNow = $derived(room?.members.find((m) => m.user_id === me)?.seat ?? null);
  let drainingMe = $derived(
    mySeatNow !== null && room?.board?.turn_seat !== mySeatNow && !!room?.board?.vacuum?.siphoning.includes(mySeatNow),
  );
  let bracingNow = $derived(mySeatNow !== null && !!room?.board?.vacuum?.braced.includes(mySeatNow));
  let covenView = $state<CovenView | null>(null);
  let landlordView = $state<LandlordView | null>(null);
  let covenHit = $state<{ seat: number; index?: number; at: number } | null>(null);
  let nativeFullscreen = $state(false);
  let expanded = $state(false);
  const fullscreen = $derived(nativeFullscreen || expanded);
  const walkingOwn = $derived(room?.board?.walk && room.board.walk.elapsed >= room.board.walk.delay && room.board.walk.seat === mySeatNow && !spectating ? room.board.walk : null);
  const firstPerson = $derived(['practice','countdown','playing'].includes(room?.phase.name??'') && room?.bouts.find((bout) => bout.key === room?.minigame)?.controls === 'first_person');

  const myDrivingNow=$derived(room?.phase.name==='board'?room.board?.driving?.find(driver=>driver.seat===mySeatNow):null);
  let driveHorn=$state(false);
  function setDriveHorn(held:boolean){
    driveHorn=held;
    if(held&&myDrivingNow){const stick=input?.read()??{x:0,z:0};connection?.send({type:'drive_input',throttle:-stick.z,steer:stick.x,horn:true});}
  }
  let connection: PartyConnection | null = null;
  let input: PartyInput | null = null;
  let sfx: PartySfx | null = null;
  let refreshing = false;
  let errorTimer: ReturnType<typeof setTimeout> | null = null;
  let goTimer: ReturnType<typeof setTimeout> | null = null;
  let restoredPick = false;
  let recamera: (() => void) | null = null;
  /** A dash or shot asked for in a top-down bout, sent on the next frame. */
  let actionPressed = false;
  let look: (dx: number, dy: number) => void = () => {};
  let trigger: (down: boolean) => void = () => {};
  let duck: (down: boolean) => void = () => {};
  let resizeRoot: (() => void) | null = null;

  async function toggleFullscreen() {
    if (nativeFullscreen) await document.exitFullscreen();
    else if (expanded) expanded = false;
    else {
      try {
        await root.requestFullscreen();
      } catch {
        // Browsers without the Fullscreen API can still fill the app viewport.
        expanded = true;
      }
    }
  }

  $effect(() => {
    if (!expanded||parked) return;
    const previous = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = previous;
    };
  });

  $effect(() => {
    void fullscreen;
    resizeRoot?.();
  });

  $effect(() => {
    if (!briefing) clickedIntoBout = false;
  });

  function toggleOverview() {
    overview = !overview;
    recamera?.();
  }

  function refreshPass() {
    if (refreshing || filming || !workflow) return;
    refreshing = true;
    void submitServerAction(provider, workflow.instance_id, { lobby_id: (workflow.responses?.lobby_id as string) ?? null }, 'games.party.join');
    setTimeout(() => (refreshing = false), 3000);
  }

  function send(msg: ClientMsg) {
    if(msg.type==='join_candy_visit'){candyJoining=true;captureCandy?.();}
    connection?.send(msg);
  }

  function showError(message: string) {
    errorMessage = message;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => (errorMessage = null), 3500);
  }

  function phaseChanged(previous: RoomView | null, next: RoomView) {
    const before = previous?.phase;
    const after = next.phase;
    if (after.name === 'countdown' && (before?.name !== 'countdown' || before.seconds !== after.seconds)) {
      sfx?.count(false);
    }
    if (after.name === 'playing' && before?.name === 'countdown') {
      sfx?.count(true);
      showGo = true;
      if (goTimer) clearTimeout(goTimer);
      goTimer = setTimeout(() => (showGo = false), 800);
    }
    if (after.name === 'game_over' && before?.name !== 'game_over') sfx?.over();
    // A chime when it comes round to you on the board.
    const mySeat = next.members.find((m) => m.user_id === me)?.seat;
    const turn = next.board?.turn_seat;
    if (after.name === 'board' && mySeat !== undefined && turn === mySeat && (previous?.board?.turn_seat !== turn || before?.name !== 'board')) {
      sfx?.turn();
    }
    if (after.name === 'results' && before?.name !== 'results') {
      const seat = next.members.find((m) => m.user_id === me)?.seat;
      if (seat !== undefined && after.placements[0]?.includes(seat)) sfx?.win();
    }
    // Bring back the character this person picked last time, if it is free.
    const mine = next.members.find((m) => m.user_id === me);
    if (guest && !restoredPick && mine && after.name === 'lobby') {
      restoredPick = true;
      try {
        const favorite = localStorage.getItem('party-character');
        if (favorite && favorite !== mine.character && !next.members.some((m) => m.character === favorite)) {
          send({ type: 'pick', character: favorite });
        }
      } catch {
        // No stored favorite.
      }
    }
  }

  $effect(() => {
    if (pass && connection) connection.setPass(pass.ws_url, pass.token);
  });

  onMount(() => {
    const playPad=()=>parked||isGameplayTyping()?null:gamepad();
    const viewport = observePartyViewport(root, () => fullscreen);
    resizeRoot = viewport.resize;
    const onFullscreenChange = () => (nativeFullscreen = document.fullscreenElement === root);
    const onEscape = (event: KeyboardEvent) => {
      if(parked)return;
      if (event.key === 'Escape') expanded = false;
      if(event.code==='Space' && connection && (connection.replay.active||connection.replay.pending)){event.preventDefault();connection.replay.skip();}
    };
    document.addEventListener('fullscreenchange', onFullscreenChange);
    document.addEventListener('keydown', onEscape);
    touch = window.matchMedia('(pointer: coarse)').matches;
    const mobile = touch || window.innerWidth < 760;
    const manager = new SceneManager({ canvas, useBloom: false, useSmaa: false, directRendering:true,maxPixelRatio:mobile?1:1.25 });
    const frameBudget=new FrameBudget(Math.min(window.devicePixelRatio,mobile?1:1.25));
    let lastShadowAt=0;
    let vanPadHeld=false;
    let cityVehiclePadHeld=false;
    let padActionHeld=false;
    // Hints name a controller's buttons when it is the controller in use.
    const onPad=()=>inputDevice()==='pad';
    const padKey=(button:'action'|'jump'|'shoulder'|'blow')=>padButton(settings().pad[button]);
    const { scene, camera, renderer } = manager;
    const cameraFlow = new CameraFlow({duration:0.48});
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    manager.setPixelRatio(frameBudget.ratio);
    renderer.shadowMap.autoUpdate=false;
    renderer.shadowMap.needsUpdate=true;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    camera.fov = 40;
    camera.far = 400;

    const stage = createPartyStage(scene, { mobile });
    const sunlight = stage.sun.intensity;
    const daylightPosition=stage.sun.position.clone();
    const daylightTarget=stage.sun.target.position.clone();
    const dayBackground=scene.background;const nightBackground=new THREE.Color('#030814');
    const hemisphere=scene.children.filter((o):o is THREE.HemisphereLight=>(o as THREE.HemisphereLight).isHemisphereLight);
    const hemiDay=hemisphere.map(light=>light.intensity);
    let shadowSpan = 0;
    /** The board is far wider than an arena, so the sun covers more of it. */
    const coverWithSun = (span: number) => {
      if (span === shadowSpan) return;
      shadowSpan = span;
      Object.assign(stage.sun.shadow.camera, { left: -span, right: span, top: span, bottom: -span, far: span * 3 });
      // A wider shadow needs a bigger map and a kinder bias, or the whole
      // island stair-steps.
      const detail = mobile ? 1024 : 2048;
      if (stage.sun.shadow.mapSize.x !== detail) {
        stage.sun.shadow.mapSize.set(detail, detail);
        stage.sun.shadow.map?.dispose();
        stage.sun.shadow.map = null as unknown as THREE.WebGLRenderTarget;
      }
      stage.sun.shadow.normalBias = span > 20 ? 0.09 : 0.03;
      stage.sun.shadow.camera.updateProjectionMatrix();
    };
    // First-person bouts hang their view models off the camera.
    scene.add(camera);
    const throwHud = new ThrowHud(camera,()=>({width:canvas.clientWidth,height:canvas.clientHeight}));
    // The vote for the next bout, as podiums floating over the island.
    const voteStage = new VoteStage(scene);
    let hoveredVote: string | null = null;
    let voting = false;
    const CONTROL_TAGS: Record<string, string> = { move: 'Move', move_action: 'Move and act', move_aim: 'Move and aim', first_person: 'First person' };
    const onVoteMove = (event: PointerEvent) => {
      if (!voting) return;
      onPointerMove(event);
      aimRay.setFromCamera(pointer, camera);
      hoveredVote = voteStage.pick(aimRay);
      canvas.style.cursor = hoveredVote ? 'pointer' : '';
    };
    const onVoteClick = (event: MouseEvent) => {
      if (!voting) return;
      onPointerMove(event);
      aimRay.setFromCamera(pointer, camera);
      const key = voteStage.pick(aimRay);
      if (key) connection?.send({ type: 'vote', key });
    };
    canvas.addEventListener('pointermove', onVoteMove);
    canvas.addEventListener('click', onVoteClick);
    // A board builds once for its map; another map, in a preview or the next
    // Tawkathon, gets a fresh one.
    let partyBoard = new PartyBoard(scene, { detail: mobile ? 0.45 : 1 });
    let cityScene:CityHeistScene|null=null;
    let boardMap: string | null = null;
    let boardIsPreview = false;
    const covenTable = new CovenTable(scene);
    const landlordTable = new LandlordTable(scene);
    const controls = new ShooterControls(0.0022, (locked) => {
      pointerLocked = locked;
      if (!locked) clickedIntoBout = false;
    });
    const onBoutClick = () => {
      if (briefing) clickedIntoBout = true;
    };
    canvas.addEventListener('pointerdown', onBoutClick);
    controls.attach(canvas);
    captureCandy=()=>controls.capture(!touch,'candy-offer');
    input = new PartyInput();
    sfx = new PartySfx();
    muted = sfx.muted;
    partyBoard.sounds = sfx;
    let rosterMap = new Map<string, CharacterDef>();

    const host: BoutHost = {
      scene,
      camera,
      canvas,
      mobile,
      waterY: WATER_Y,
      sounds: sfx,
      character: (id) => rosterMap.get(id),
      rider: (character) => buildCharacter(character as CharacterDef),
      send: (message) => connection?.send(message),
    };
    let bout: { key: string; module: BoutModule; renderer: BoutRenderer } | null = null;
    let loading = '';
    let stopped = false;
    let lastView: BoutFrame<unknown, unknown> | null = null;
    const swapBout = (key: string) => {
      if ((bout?.key ?? '') === key) return;
      bout?.renderer.dispose();
      bout = null;
      hud = null;
      recamera?.();
      if (!key) {
        loading = '';
        return;
      }
      if (loading === key) return;
      loading = key;
      loadBout(key)?.then((module) => {
        if (stopped || loading !== key) return;
        loading = '';
        bout = { key, module, renderer: module.create(host) };
        recamera?.();
      });
    };

    // Where the pointer is over the arena, for bouts that aim with the mouse.
    const pointer = new THREE.Vector2();
    let aimYaw = 0;
    const groundPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -0.9);
    const aimRay = new THREE.Raycaster();
    const aimPoint = new THREE.Vector3();
    const hoverRay = new THREE.Raycaster();
    let orbitYaw = 0;
    let orbitPitch = 0;
    let orbitSeat: number | null = null;
    let orbitView = '';
    let orbitDragging = false;
    let pendingFacing: number | null = null;
    let lastFacingSent = 0;
    let zoomTarget = 1;
    let zoom = 1;
    const canOrbit = () => onBoard && (room?.phase.name === 'board'||room?.phase.name==='map_finale'||room?.phase.name==='city_heist') && !!room.board && !room.board.walk &&
      !lobbing && !room?.board?.positioning && !room?.board?.driving?.length && !overview && !teleporting && !replayStatus &&
      (!partyBoard.shot() || room.board.turn.step === 'shop');
    const onOrbitDown = (event: PointerEvent) => {
      if (event.button !== 1 || !canOrbit()) return;
      event.preventDefault();
      orbitDragging = true;
      orbitSeat = room!.phase.name==='map_finale'?(mySeatNow??room!.board!.turn_seat):room!.board!.turn_seat;
      if(room?.phase.name!=='city_heist')orbitYaw = boardYaw;
      canvas.setPointerCapture(event.pointerId);
      controls.capture(true, `orbit:${orbitView}`);
      tip = null;
    };
    const onOrbitUp = (event: PointerEvent) => {
      if (event.button !== 1) return;
      orbitDragging = false;
      if (canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
    };
    const onOrbitCancel = () => { orbitDragging = false; };
    const onOrbitUnlock = () => { if (document.pointerLockElement !== canvas) orbitDragging = false; };
    const onOrbitAux = (event: MouseEvent) => { if (event.button === 1 && onBoard) event.preventDefault(); };
    const onOrbitWheel = (event: WheelEvent) => {
      if (!onBoard) return;
      event.preventDefault();
      if (!canOrbit()) return;
      const pixels = event.deltaY * (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? canvas.clientHeight : 1);
      zoomTarget = THREE.MathUtils.clamp(zoomTarget * Math.exp(THREE.MathUtils.clamp(pixels, -200, 200) * 0.0018), 0.22, 2.2);
      orbitSeat = room!.phase.name==='map_finale'?(mySeatNow??room!.board!.turn_seat):room!.board!.turn_seat;
    };
    canvas.addEventListener('pointerdown', onOrbitDown);
    canvas.addEventListener('pointercancel', onOrbitCancel);
    canvas.addEventListener('auxclick', onOrbitAux);
    canvas.addEventListener('wheel', onOrbitWheel, {passive:false});
    window.addEventListener('pointerup', onOrbitUp);
    window.addEventListener('blur', onOrbitCancel);
    document.addEventListener('pointerlockchange', onOrbitUnlock);
    const onPointerMove = (event: MouseEvent) => {
      const box = canvas.getBoundingClientRect();
      pointer.set(((event.clientX - box.left) / box.width) * 2 - 1, -((event.clientY - box.top) / box.height) * 2 + 1);
      // No tips while aiming: the mouse is busy.
      if (!onBoard || lobbing || walkingOwn || replayStatus) {
        tip = null;
        return;
      }
      const board = room?.board;
      if (event.type === 'mousemove' && orbitDragging && canOrbit() && board) {
        const { sensitivity, invertY } = settings().camera;
        orbitYaw -= THREE.MathUtils.clamp(event.movementX, -80, 80) * 0.004 * sensitivity;
        orbitPitch = THREE.MathUtils.clamp(orbitPitch + THREE.MathUtils.clamp(event.movementY, -80, 80) * 0.14 * sensitivity * (invertY ? -1 : 1), -15, 25);
        orbitSeat = room?.phase.name==='map_finale'?(mySeatNow??board.turn_seat):board.turn_seat;
        tip = null;
        return;
      }
      hoverRay.setFromCamera(pointer, camera);
      const found = partyBoard.hover(hoverRay);
      tip = found ? { ...found, x: event.clientX - box.left, y: event.clientY - box.top } : null;
    };
    canvas.addEventListener('mousemove', onPointerMove);
    canvas.addEventListener('mouseleave', () => (tip = null));
    let controlsKind = 'move';
    const askAction = () => {
      if(parked||isGameplayTyping()||!bout||!['playing','countdown','practice'].includes(room?.phase.name??''))return;
      if (controlsKind === 'move_action' || controlsKind === 'move_aim') actionPressed = true;
    };
    const onDriveKey=(event:KeyboardEvent)=>{if(myDrivingNow&&!typing(event)&&bound('action',event.code)){event.preventDefault();if(!event.repeat)setDriveHorn(event.type==='keydown');}};
    const releaseDriveHorn=()=>{driveHorn=false;};
    window.addEventListener('keydown',onDriveKey);window.addEventListener('keyup',onDriveKey);window.addEventListener('blur',releaseDriveHorn);
    const onCandyKey=(event:KeyboardEvent)=>{
      if((event.code!=='KeyB'&&!bound('interact',event.code))||event.repeat||typing(event)||room?.phase.name!=='board')return;
      const van=room.board?.candy_van;if(!van)return;
      if(van.phase==='offer'&&van.can_join&&event.code==='KeyB'){event.preventDefault();send({type:'join_candy_visit'});}
      else if(van.phase==='visit'&&van.can_buy){event.preventDefault();send({type:'buy_candy'});}
    };
    window.addEventListener('keydown',onCandyKey);
    const onCityKey=(event:KeyboardEvent)=>{
      if(room?.phase.name!=='city_heist'||typing(event))return;
      if(bound('interact',event.code)){event.preventDefault();cityHeld=event.type==='keydown';}
      if(event.type==='keydown'&&!event.repeat&&event.code==='KeyF'){event.preventDefault();send({type:'city_vehicle'});}
      if(event.type==='keydown'&&!event.repeat&&['Digit1','Digit2'].includes(event.code)){event.preventDefault();send({type:'city_tool',tool:event.code==='Digit1'?'smoke':'scanner'});}
    };
    const clearCity=()=>{cityHeld=false;citySuction=false;};
    window.addEventListener('keydown',onCityKey);window.addEventListener('keyup',onCityKey);window.addEventListener('blur',clearCity);

    const onActionKey = (event: KeyboardEvent) => {
      if (bound('action', event.code)) askAction();
    };
    canvas.addEventListener('mousedown', askAction);
    window.addEventListener('keydown', onActionKey);
    look = (dx, dy) => controls.look(dx, dy);
    trigger = (down) => {
      controls.pullTrigger(down);
      if (down) askAction();
    };
    duck = (down) => controls.setCrouch(down);

    const cameraOffset = new THREE.Vector3();
    const frameHalf = new THREE.Vector2(13, 10);
    /** The board's usual shot: how much of it is in view, and how steeply it is looked down on. */
    const BOARD_WIDE = 21;
    const BOARD_TALL = 15;
    const BOARD_PITCH = 36;
    const BOARD_PITCH_PORTRAIT = 48;
    /** Where the board camera is swung to, and the height it looks at. */
    let boardYaw = 0;
    let boardLook = 0;
    const follow = new THREE.Vector3();
    const target = new THREE.Vector3();
    let lookHeight = -0.8;
    /** How far the camera is into a tunnel's own shot, from 0 to 1, and where that shot is. */
    let caveMix = 0;
    const caveEye = new THREE.Vector3();
    const caveLook = new THREE.Vector3();
    const lookAt = new THREE.Vector3();
    const skyFog = new THREE.Color('#bfeaff');
    const caveFog = new THREE.Color('#0b0806');
    let onBoard = false;
    let atTable = false;
    const onTableClick = (event: MouseEvent) => {
      if (!atTable) return;
      onPointerMove(event);
      aimRay.setFromCamera(pointer, camera);
      const found = room?.minigame === 'landlord' ? landlordTable.pick(aimRay) : covenTable.pick(aimRay);
      if (found) covenHit = { ...found, at: performance.now() };
    };
    canvas.addEventListener('click', onTableClick);
    // Aiming a bomb: a click on a space in reach puts the target there.
    const onBoardClick = (event: MouseEvent) => {
      if (flyDragged || lobbing) return;
      if (teleporting && room?.sandbox && onBoard) {
        onPointerMove(event);
        aimRay.setFromCamera(pointer, camera);
        const space = partyBoard.pickSpace(aimRay);
        if (space !== null) {
          connection?.send({ type: 'dev', tool: { action: 'teleport', space } });
          teleporting = false;
        }
        return;
      }
    };
    canvas.addEventListener('click', onBoardClick);

    // Throwing a bomb, through your own eyes: the mouse turns you and tips the
    // throw up or down, A and D turn you further round, and holding the mouse
    // or Space swings the power, thrown when you let go.
    const POWER_SWING = 1.5;
    let lobbing = false;
    let lobItemLabel = 'Bomb';
    /** Which shoulder the throw camera looks over: 1 the right, -1 the left. Swapped
     * with a right click or Q, and remembered. `shoulderAt` glides after it. */
    let shoulder = 1;
    try {
      shoulder = localStorage.getItem('party-shoulder') === 'left' ? -1 : 1;
    } catch {
      // Remembering the side is a convenience only.
    }
    let shoulderAt = shoulder;
    const swapShoulder = () => {
      shoulder = -shoulder;
      try {
        localStorage.setItem('party-shoulder', shoulder < 0 ? 'left' : 'right');
      } catch {
        // As above.
      }
    };
    let walkBlowing=false;
    let rightVacuumHeld=false,rightVacuumOwned=false;
    let rightVacuumRequestedAt=0;
    const onWalkBlow=(event:PointerEvent)=>{
      if(parked||event.button!==2||isGameplayTyping(event.target))return;
      if(room?.phase.name==='city_heist'){event.preventDefault();rightVacuumHeld=true;return;}
      if(room?.phase.name!=='board')return;
      const board=room.board,seat=room.members.find(m=>m.user_id===me)?.seat;
      if(!board||board.candy_van||board.raids?.some(r=>r.seat===seat)||board.positioning?.seat===seat||board.driving?.some(d=>d.seat===seat))return;
      if(walkingOwn){event.preventDefault();rightVacuumHeld=true;return;}
      if(board.turn_seat!==seat)return;
      if(board.turn.step==='vacuum'||((board.turn.step==='items'||board.turn.step==='done')&&(!board.sucked||room.sandbox))){
        event.preventDefault();rightVacuumHeld=true;rightVacuumOwned=true;rightVacuumRequestedAt=performance.now();
        if(board.turn.step!=='vacuum')connection?.send({type:'suck'});else suckWith(true,false);
      }
    };
    const onWalkBlowUp=(event:PointerEvent)=>{if(event.button===2){rightVacuumHeld=false;walkBlowing=false;
      if(rightVacuumOwned&&room?.board?.turn.step==='vacuum'){suckWith(false,false);connection?.send({type:'stow_vacuum'});rightVacuumOwned=false;}
    }};
    const onWalkBlowKey=(event:KeyboardEvent)=>{if(walkingOwn && !typing(event) && bound('blow',event.code)){event.preventDefault();walkBlowing=event.type==='keydown';}};
    const clearWalkBlow=()=>{walkBlowing=false;rightVacuumHeld=false;};
    const unlockWalkBlow=()=>{if(document.pointerLockElement!==canvas)clearWalkBlow();};
    canvas.addEventListener('pointerdown',onWalkBlow);
    window.addEventListener('pointerup',onWalkBlowUp);
    window.addEventListener('keydown',onWalkBlowKey);
    window.addEventListener('keyup',onWalkBlowKey);
    window.addEventListener('blur',clearWalkBlow);
    document.addEventListener('pointerlockchange',unlockWalkBlow);
    const onLobMenu = (event: MouseEvent) => {
      if (lobbing || walkingOwn) event.preventDefault();
    };
    let lobBase = 0;
    let lobYaw = 0;
    let lobPitch = 0.7;
    let lobLookPitch=-.12;
    let lobStarted = 0;
    let chargeFrom: number | null = null;
    const lobTurn = new Set<string>();
    let padFiring = false;
    let padSwapping = false;
    let padBlowing = false;
    const swing = (now: number) => {
      const phase = (((now - (chargeFrom ?? now)) / 1000) % POWER_SWING) / (POWER_SWING / 2);
      return phase < 1 ? phase : 2 - phase;
    };
    // For driving the game from a test page: the board, and the aim while something is in hand.
    const testHook = {
      // A getter: a new board is made for each map.
      get board() {
        return partyBoard;
      },
      get room(){return room;},
      get seat(){return mySeatNow;},
      canvas,
      renderer,
      get audio(){return sfx;},
      scene,
      camera,
      yaw: () => lobYaw,
      aim: (yaw: number) => {
        lobBase += yaw - lobYaw;
        lobYaw = yaw;
      },
    };
    const hookHost = window as unknown as { __party?: unknown };
    if (import.meta.env.DEV) hookHost.__party = testHook;
    const onLobAim = (event: MouseEvent) => {
      if (!lobbing || !controls.locked) return;
      const { sensitivity, invertY } = settings().camera;
      lobYaw -= event.movementX * 0.0022 * sensitivity;
      lobLookPitch=THREE.MathUtils.clamp(lobLookPitch-event.movementY*.0022*sensitivity*(invertY?-1:1),-1.4,1.4);
      lobPitch=THREE.MathUtils.clamp(.7+lobLookPitch,.08,1.4);
    };
    const onLobTouchAim = (event: PointerEvent) => {
      if (!lobbing || event.pointerType !== 'touch') return;
      const rect = canvas.getBoundingClientRect();
      const nx = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      const ny = ((event.clientY - rect.top) / rect.height) * 2 - 1;
      const {sensitivity,invertY}=settings().camera;
      lobYaw = lobBase - nx * 1.3 * sensitivity;
      lobLookPitch = THREE.MathUtils.clamp(-ny * .7 * sensitivity * (invertY ? -1 : 1), -1.4, 1.4);
      lobPitch = THREE.MathUtils.clamp(.7 + lobLookPitch, .08, 1.4);
    };
    // A bat is swung, not thrown: its needle swings on its own, and a click
    // or Space stops it.
    const NEEDLE_SWING = 0.9;
    /** How long the camera stays over your shoulder after a swing or a throw,
     * to watch it land; a throw holds on longer while it is still in the air. */
    const SWING_HOLD_MS = 1200;
    const CHASE_HOLD_MS = 6000;
    let batting = false;
    let swungAt = 0;
    /** The camera as the throw left it, for gliding back to the board from. */
    let glide: { position: THREE.Vector3; quaternion: THREE.Quaternion; fov: number; t: number } | null = null;
    // A camera, so that looking at something points its -z there, as the real one does.
    const chaseLook = new THREE.PerspectiveCamera();
    const needle = (now: number) => {
      const phase = (((now - lobStarted) / 1000) % NEEDLE_SWING) / (NEEDLE_SWING / 2);
      return phase < 1 ? phase : 2 - phase;
    };
    const swingNow = () => {
      if (!lobbing || !batting) return;
      batting = false;
      swungAt = performance.now();
      throwHud.hide();
      partyBoard.setArc(null);
      partyBoard.setBatReach(null);
      connection?.send({ type: 'swing', yaw: lobYaw, stop: needle(performance.now()) });
    };
    // The vacuum: held on to suck or blow, let go to stop, its aim sent as it moves.
    let vacuuming = false;
    let sucking = false;
    let blowing = false;
    let vacuumSent = 0;
    let vacuumSentYaw = 0;
    const sendVacuum = (force = false) => {
      if(room?.phase.name!=='board'||room.board?.turn.step!=='vacuum'||room.board.turn_seat!==mySeatNow)return;
      const now = performance.now();
      if (!force && now - vacuumSent < 100 && Math.abs(lobYaw - vacuumSentYaw) < 0.02) return;
      vacuumSent = now;
      vacuumSentYaw = lobYaw;
      connection?.send({ type: 'vacuum', yaw: lobYaw, on: sucking, blow: blowing });
    };
    const suckWith = (on: boolean, blow = false) => {
      if (!vacuuming) return;
      if (on) {
        if (sucking && blowing === blow) return;
        blowing = blow;
      } else if (!sucking || blowing !== blow) return;
      sucking = on;
      sendVacuum(true);
    };
    const throwNow = () => {
      if(parked||room?.board?.candy_van)return;
      if (vacuuming) {
        suckWith(false, blowing);
        return;
      }
      if (!lobbing || chargeFrom === null) return;
      const power = swing(performance.now());
      chargeFrom = null;
      swungAt = performance.now();
      throwHud.hide();
      partyBoard.setArc(null);
      connection?.send({ type: 'lob', yaw: lobYaw, pitch: lobPitch, power });
    };
    const onLobDown = (event: PointerEvent) => {
      if (!lobbing||room?.board?.candy_van||isGameplayTyping(event.target)) return;
      if (event.pointerType !== 'touch' && !controls.locked) {
        // Resuming after Escape captures the mouse without using the item.
        controls.lock();
        return;
      }
      if (lobbing && event.button === 2) {
        if (vacuuming) suckWith(true,false);
        else swapShoulder();
        return;
      }
      if (event.button !== 0) return;
      if (vacuuming) suckWith(true);
      else if (batting) swingNow();
      else if (chargeFrom === null) chargeFrom = performance.now();
    };
    const onLobKey = (event: KeyboardEvent) => {
      if (!lobbing || room?.board?.candy_van || typing(event)) return;
      if (bound('action', event.code)) {
        event.preventDefault();
        if (vacuuming) {
          if (!event.repeat) suckWith(event.type === 'keydown');
          return;
        }
        if (batting) {
          if (event.type === 'keydown' && !event.repeat) swingNow();
          return;
        }
        if (event.type === 'keydown' && !event.repeat && chargeFrom === null) chargeFrom = performance.now();
        if (event.type === 'keyup') throwNow();
      } else if (vacuuming && bound('blow', event.code)) {
        event.preventDefault();
        if (!event.repeat) suckWith(event.type === 'keydown', true);
      } else if (bound('shoulder', event.code)) {
        if (event.type === 'keydown' && !event.repeat) swapShoulder();
      } else if (bound('left', event.code) || bound('right', event.code)) {
        if (event.type === 'keydown') lobTurn.add(event.code);
        else lobTurn.delete(event.code);
      }
    };
    document.addEventListener('mousemove', onLobAim);
    canvas.addEventListener('pointermove', onLobTouchAim);
    canvas.addEventListener('pointerdown', onLobDown);
    canvas.addEventListener('contextmenu', onLobMenu);
    window.addEventListener('pointerup', throwNow);
    window.addEventListener('keydown', onLobKey);
    const cancelLobInput = () => {
      chargeFrom = null;
      lobTurn.clear();
      suckWith(false, blowing);
    };
    const onLobUnlock = () => {
      if (document.pointerLockElement !== canvas) cancelLobInput();
    };
    document.addEventListener('pointerlockchange', onLobUnlock);
    window.addEventListener('blur', cancelLobInput);
    const onTypingFocus=(event:FocusEvent)=>{
      if(!isGameplayTyping(event.target))return;
      actionPressed=false;chargeFrom=null;lobTurn.clear();walkBlowing=false;driveHorn=false;
      padFiring=false;padBlowing=false;padSwapping=false;controls.clearHeld();
      rightVacuumHeld=false;cityHeld=false;citySuction=false;
      if(rightVacuumOwned&&room?.board?.turn.step==='vacuum'){connection?.send({type:'stow_vacuum'});rightVacuumOwned=false;}
      suckWith(false,blowing);controls.release();
    };
    document.addEventListener('focusin',onTypingFocus);

    // The vacuum key: out before you roll or once you land, and away again.
    const onVacuumKey = (event: KeyboardEvent) => {
      if (event.repeat || typing(event) || !bound('vacuum', event.code)) return;
      const board = room?.board;
      const mine = room?.members.find((m) => m.user_id === me);
      if (!board || board.turn_seat !== mine?.seat) return;
      if (board.turn.step === 'vacuum') connection?.send({ type: 'stow_vacuum' });
      else if ((board.turn.step === 'items' || board.turn.step === 'done') && (!board.sucked || room?.sandbox)) connection?.send({ type: 'suck' });
    };
    window.addEventListener('keydown', onVacuumKey);
    // Someone else's vacuum draining you: hold the action key to brace.
    const onBraceKey = (event: KeyboardEvent) => {
      if (lobbing || typing(event) || event.repeat || !bound('action', event.code)) return;
      if (event.type === 'keyup') {
        connection?.send({ type: 'brace', on: false });
      } else if (drainingMe) {
        event.preventDefault();
        connection?.send({ type: 'brace', on: true });
      }
    };
    window.addEventListener('keydown', onBraceKey);
    window.addEventListener('keyup', onBraceKey);
    window.addEventListener('keyup', onLobKey);
    /** Each frame of a throw: the camera at your eyes, the start of the arc, and the HUD. */
    const lobFrame = (dt: number, seat: number, wind: [number, number] | null, fuse: number, mode: 'throw' | 'swing' | 'vacuum' = 'throw') => {
      const now = performance.now();
      if (!lobbing) {
        // Start facing the way the path goes on.
        const at = partyBoard.focus(seat);
        const ahead = partyBoard.ahead(seat, 4);
        lobBase = ahead.distanceTo(at) > 0.5 ? Math.atan2(ahead.x - at.x, ahead.z - at.z) : 0;
        lobYaw = room?.board?.positioning?.active?controls.yaw+Math.PI:lobBase;
        lobLookPitch=room?.board?.positioning?.active?controls.pitch:-.12;
        lobPitch = THREE.MathUtils.clamp(.7+lobLookPitch,.08,1.4);
        lobStarted = now;
        chargeFrom = null;
        lobTurn.clear();
        lobbing = true;
        batting = mode === 'swing';
        vacuuming = mode === 'vacuum';
        sucking = false;
        blowing = false;
      }
      // Thrown or swung: the camera is following it now, so leave it be.
      if (swungAt > lobStarted) return;
      const holding = (action: 'left' | 'right') => settings().keys[action].some((code) => lobTurn.has(code));
      let turn = (holding('left') ? 1 : 0) - (holding('right') ? 1 : 0);
      // A controller: the right stick aims, the action button winds up and
      // throws or swings, and the shoulder button swaps shoulders.
      const pad = playPad();
      if (pad) {
        const { deadzone, action, shoulder: swap, blow } = settings().pad;
        const [, , rx = 0, ry = 0] = pad.axes;
        if (Math.abs(rx) > deadzone) turn -= rx * 1.4;
        if (Math.abs(ry) > deadzone) {lobLookPitch = THREE.MathUtils.clamp(lobLookPitch - ry * dt * 1.6 * (settings().camera.invertY ? -1 : 1), -1.4, 1.4);lobPitch = THREE.MathUtils.clamp(.7 + lobLookPitch, .08, 1.4);}
        const firing = !!pad.buttons[action]?.pressed;
        const swapping = !!pad.buttons[swap]?.pressed;
        const gusting = !!pad.buttons[blow]?.pressed;
        if (vacuuming) {
          if (firing !== padFiring) suckWith(firing);
          if (gusting !== padBlowing) suckWith(gusting, true);
          padBlowing = gusting;
        } else if (firing && !padFiring) {
          if (batting) swingNow();
          else if (chargeFrom === null) chargeFrom = performance.now();
        } else if (!firing && padFiring && !batting) throwNow();
        if (swapping && !padSwapping) swapShoulder();
        padFiring = firing;
        padSwapping = swapping;
      }
      lobBase += turn * dt * 1.8;
      lobYaw += turn * dt * 1.8;
      const at = partyBoard.focus(seat);
      const facing = new THREE.Vector3(Math.sin(lobYaw), 0, Math.cos(lobYaw));
      // Over the shoulder, a little to the right and well above, on a wide
      // lens: you see yourself, the ground ahead, and where it will land.
      const right = new THREE.Vector3(-facing.z, 0, facing.x);
      shoulderAt += (shoulder - shoulderAt) * (1 - Math.exp(-dt * 8));
      camera.position.copy(at).add(new THREE.Vector3(0,1.45,0));
      camera.rotation.set(lobLookPitch,lobYaw+Math.PI,0,'YXZ');
      partyBoard.clearSightline(camera.position, at.clone().add(new THREE.Vector3(0, 1.2, 0)));
      if (mode === 'vacuum') {
        partyBoard.setArc(null);
        const vacuum = room?.board?.vacuum ?? null;
        partyBoard.setVacuum(seat, vacuum, lobYaw,lobLookPitch);
        sendVacuum();
        const left = vacuum?.left ?? fuse;
        lobView = { power: left / fuse, charging: true, fuseLeft: left, wind: 0, windAngle: 0 };
        const blowKey = keyName(settings().keys.blow[0] ?? 'KeyF');
        const controls = onPad()
          ? `HOLD ${padKey('action')}  Suction     HOLD ${padKey('blow')}  Blow     RIGHT STICK  Aim`
          : `HOLD CLICK / SPACE  Suction     ${blowKey}  Blow     RMB  Hold / auto-stow     V  Stow     ESC  Cursor`;
        // How far the chest has come, since it is the one thing worth keeping the nozzle on.
        const prizeAt = room?.board?.prize_at;
        const chest = prizeAt == null ? 0 : (vacuum?.pulling.find(([key]) => key === pullKey(PULL.chest, prizeAt))?.[1] ?? 0);
        throwHud.show(camera, {
          ...lobView,
          label:
            chest > 0
              ? `Pulling the chest in · ${Math.round(Math.min(1, chest) * 100)}%`
              : sucking
                ? blowing
                  ? 'Vacuum · blowing'
                  : 'Vacuum · sucking'
                : 'Vacuum',
          needle: false,
          fuse,
          calm: true,
          worldReticle: true,
          notice: partyBoard.vacuumFeedback()??undefined,
          hint: controls,
        });
        return;
      }
      if (!wind) {
        // Swung, and waiting on the server to say how it went.
        if (!batting) return;
        partyBoard.holdBat(seat);
        // The bat's reach, as dots round the arc on the ground in front.
        const reach: THREE.Vector3[] = [];
        for (let i = 0; i <= 16; i++) {
          const angle = lobYaw - BAT.arc + (BAT.arc * 2 * i) / 16;
          const x = at.x + Math.sin(angle) * BAT.range;
          const z = at.z + Math.cos(angle) * BAT.range;
          reach.push(new THREE.Vector3(x, partyBoard.groundAt(x, z) + 0.15, z));
        }
        for (const angle of [lobYaw - BAT.arc, lobYaw + BAT.arc]) {
          for (let i = 1; i < 6; i++) {
            const x = at.x + Math.sin(angle) * (BAT.range * i) / 6;
            const z = at.z + Math.cos(angle) * (BAT.range * i) / 6;
            reach.push(new THREE.Vector3(x, partyBoard.groundAt(x, z) + 0.15, z));
          }
        }
        partyBoard.setArc(reach);
        const inReach = partyBoard.setBatReach(seat, lobYaw);
        lobView = { power: needle(now), charging: true, fuseLeft: Math.max(0, fuse - (now - lobStarted) / 1000), wind: 0, windAngle: 0 };
        throwHud.show(camera, {
          ...lobView,
          label: inReach ? `Bat in hand · ${inReach === 1 ? 'one rival' : `${inReach} rivals`} in reach` : 'Bat in hand · nobody in reach',
          needle: true,
          fuse,
          hint: onPad()
            ? inReach ? `${padKey('action')} to swing · ${padKey('shoulder')} to swap shoulder` : `Turn with the right stick until someone stands in the arc · ${padKey('shoulder')} to swap shoulder`
            : inReach ? 'Click or Space to swing · right click to swap shoulder' : 'Turn until someone stands in the arc · right click to swap shoulder',
        });
        return;
      }
      // Thrown, and waiting on the server to say where it went.
      if (swungAt > lobStarted) return;
      // Only the first part of the arc shows, and none of the wind: the rest is judgement.
      const power = chargeFrom === null ? 0.55 : swing(now);
      const flight = bombFlight(lobPitch, power);
      const points: THREE.Vector3[] = [];
      for (let i = 1; i <= 14; i++) {
        const p = bombAt(lobYaw, lobPitch, power, [0, 0], (flight * 0.35 * i) / 14);
        points.push(new THREE.Vector3(at.x + p.x, at.y + p.y, at.z + p.z));
      }
      partyBoard.setArc(points);
      const windStrength = Math.hypot(wind[0], wind[1]);
      lobView = {
        power,
        charging: chargeFrom !== null,
        fuseLeft: Math.max(0, fuse - (now - lobStarted) / 1000),
        wind: windStrength,
        windAngle: Math.atan2(wind[0], wind[1]) - lobYaw,
      };
      throwHud.show(camera, { ...lobView, label: `${lobItemLabel} in hand`, needle: false, fuse, ...(onPad() ? { hint: `HOLD ${padKey('action')}  Charge · release to throw     ${padKey('shoulder')}  Shoulder     RIGHT STICK  Aim` } : {}) });
    };

    // Left-drag only moves sandbox tokens. Board camera movement is MMB
    // orbit plus wheel zoom; keyboard directions belong to characters.
    let flyDragged = false;
    const typing = (event:{target:EventTarget|null}) => parked||isGameplayTyping(event.target);
    let dragging: { seat: number; space: number | null } | null = null;
    const onFlyDown = (event: PointerEvent) => {
      if (lobbing || walkingOwn || room?.board?.positioning) return;
      if (room?.sandbox && onBoard && event.button === 0) {
        onPointerMove(event);aimRay.setFromCamera(pointer,camera);
        const seat=partyBoard.pickToken(aimRay);
        if(seat!==null){dragging={seat,space:null};flyDragged=true;}
      }
    };
    const onFlyMove = (event: PointerEvent) => {
      if(!dragging)return;
      onPointerMove(event);aimRay.setFromCamera(pointer,camera);
      dragging.space=partyBoard.pickSpace(aimRay);partyBoard.dragPreview(dragging.seat,dragging.space);
    };
    const onFlyUp = () => {
      if(dragging){const {seat,space}=dragging;dragging=null;partyBoard.dragPreview(seat,null);if(space!==null)connection?.send({type:'dev',tool:{action:'teleport',space,seat}});}
      setTimeout(()=>flyDragged=false,0);
    };
    canvas.addEventListener('pointerdown',onFlyDown);
    window.addEventListener('pointermove',onFlyMove);
    window.addEventListener('pointerup',onFlyUp);
    const fit = () => {
      camera.aspect = canvas.clientWidth / Math.max(1, canvas.clientHeight);
      camera.updateProjectionMatrix();
    };
    /** The shortest way round to an angle, so the camera never spins the long way. */
    const wrapAngle = (angle: number) => Math.atan2(Math.sin(angle), Math.cos(angle));
    /** Keeps a box of `wide` by `tall` in view from above at `pitchDegrees`. */
    const frameBox = (wide: number, tall: number, pitchDegrees: number) => {
      const portrait = camera.aspect < 0.8;
      const halfV = THREE.MathUtils.degToRad(camera.fov / 2);
      const halfH = Math.atan(Math.tan(halfV) * camera.aspect);
      frameHalf.set(wide, tall);
      const distance = Math.max(wide / Math.tan(halfH), tall / Math.tan(halfV)) * (portrait ? 1.0 : 1.02);
      const pitch = THREE.MathUtils.degToRad(pitchDegrees);
      cameraOffset.set(0, Math.sin(pitch) * distance, Math.cos(pitch) * distance);
    };
    const aimBoard = () => {
      const board = partyBoard.bounds();
      if (overview) target.copy(board.center);
      else {
        // Whoever's turn it is, a little toward where they are heading, at
        // the height they stand at: on a hill, aiming at sea level under them
        // pushed them to the edge of the screen.
        const seat = room?.board?.turn_seat ?? null;
        target.copy(partyBoard.focus(seat)).lerp(partyBoard.ahead(seat), 0.4);
        // Keep the shot over the island rather than sliding out to sea.
        const reachX = Math.max(0, board.halfW + 3 - frameHalf.x);
        const reachZ = Math.max(0, board.halfD + 3 - frameHalf.y);
        target.x = THREE.MathUtils.clamp(target.x, board.center.x - reachX, board.center.x + reachX);
        target.z = THREE.MathUtils.clamp(target.z, board.center.z - reachZ, board.center.z + reachZ);
      }
    };
    let snapCamera = true;
    recamera = () => {
      snapCamera = true;cameraFlow.transition();
    };
    fit();
    const resize = new ResizeObserver(fit);
    resize.observe(canvas.parentElement ?? canvas);

    connection = new PartyConnection({
      welcome: (userId, list, isGuest) => {
        me = userId;
        guest = isGuest;
        roster = list;
      },
      room: (view, watching) => {
        const previous = room;
        room = view;
        spectating = watching;
        phaseChanged(previous, view);
      },
      events: (events: GameEvent[]) => {
        const playing = bout;
        // Effects wait for the interpolated view to reach the moment they happened.
        setTimeout(() => {
          if (!playing || playing !== bout || !lastView || connection?.replay.active) return;
          for (const event of events) playing.renderer.effect?.(event, lastView);
        }, 100);
      },
      table: (game, view) => {
        if (game === 'coven') covenView = view as CovenView;
        else if (game === 'landlord') landlordView = view as LandlordView;
      },
      error: showError,
      feedbackResult:(result)=>{feedbackResult=result;},
      status: (next) => (status = next),
      passExpired: refreshPass,
    });
    if (pass) connection.setPass(pass.ws_url, pass.token);
    const passFallback = setTimeout(() => {
      if (!pass) refreshPass();
    }, 4000);

    let frame = 0;
    let last = performance.now();
    let lastSent: StickValue = { x: 0, z: 0 };
    let lastSendAt = 0;
    let rosterKey = '';
    let walkLeg=-1;
    let walkRollSequence=-1;
    const walkFollow=new THREE.Vector3();
    let walkFollowing=false;
    let lastWalkSend=0;
    let wasWalking=false;
    let wasUp = false;
    let hudKey = '';
    let wasReplay=false;
    let wasBoardReplay=false;
    let boardCategory='';
    let lastPositionSend=0;
    let wasPositioning=false;
    let raidSeat:number|null=null;
    let lastRaidSend=0;
    let lastRaidSlash=0;
    let lastDriveSend=0;
    let drivingSeat:number|null=null;
    const drivingFocus=new THREE.Vector3();
    let drivingYaw=0;
    let itemEye=false;
    let lastRaidFiring=false;
    let crawlerEye=false;
    let lastCrawlerSend=0;
    let lastCandySend=0;
    let wasParked=false;
    let hadCandyVan=false;
    let cityAlarmKey="";
    let candyPausedAt=0;
    let candyEye=false;
    const currentRoom=()=>room;

    const loop = (now: number) => {
      frame = requestAnimationFrame(loop);
      let dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      let time = now / 1000;
      if (!connection) return;
      if(parked!==wasParked){sfx?.setSuspended(parked);wasParked=parked;}
      if(parked){controls.enabled=false;controls.clearHeld();controls.capture(false);if(input)input.enabled=false;actionPressed=false;rightVacuumHeld=false;rightVacuumOwned=false;chargeFrom=null;lobTurn.clear();pendingFacing=null;return;}
      const boardPlayback=connection.replay.sampleBoard(time);
      const room=boardPlayback?.room??currentRoom();
      if(boardPlayback){dt=boardPlayback.dt;time=boardPlayback.time;}
      if(wasBoardReplay!==!!boardPlayback){
        partyBoard.dispose();partyBoard=new PartyBoard(scene,{detail:mobile?0.45:1});partyBoard.sounds=sfx;
        boardMap=null;recamera?.();
      }
      wasBoardReplay=!!boardPlayback;
      partyBoard.replaying=!!boardPlayback;

      const appearanceKey=JSON.stringify([roster,room?.members.map(member=>[member.character,member.costume])]);
      if (rosterKey !== appearanceKey) {
        rosterKey = appearanceKey;
        rosterMap = new Map(roster.map((c) => [c.id, c]));
        for(const member of room?.members??[]) {
          const base=rosterMap.get(member.character);
          if(base && member.costume)rosterMap.set(member.character,{...base,costume:member.costume});
        }
      }
      const members = room?.members ?? [];
      const mine = members.find((m) => m.user_id === me) ?? null;
      const phase = room?.phase.name ?? 'lobby';
      const tableKey = (room?.table_games ?? []).some(([key]) => key === room?.minigame) ? (room?.minigame ?? '') : '';
      const tableGame = tableKey !== '';
      // A lobby set up for a Tawkathon shows the board it will be played on.
      const previewing = phase === 'lobby' && !!room?.preview_board && !tableGame;
      const cityWorld=!!room?.city_mode&&['city_heist','lobby','vote','game_over'].includes(phase);
      const myCity=room?.city?.actors.find(actor=>actor.seat===mine?.seat)??null;
      const alarmKey=room?.city?.alarm?`${room.match_id}:${room.city.round}`:"";
      if(alarmKey&&alarmKey!==cityAlarmKey)sfx?.heistAlarm();cityAlarmKey=alarmKey;
      const boardPhase = cityWorld || phase === 'map_finale' || (phase==='results'&&!!room?.board?.finale) || phase === 'board' || phase === 'vote' || phase === 'game_over' || previewing;
      const nextBoardCategory = !boardPhase ? 'arena' : previewing ? 'preview' : phase === 'game_over' ? 'final' : 'play';
      if (nextBoardCategory !== boardCategory) {
        boardCategory=nextBoardCategory;
        onBoard = boardPhase;
        overview = boardPhase && (phase === 'game_over' || previewing);
        recamera?.();
      }
      if (tableGame !== atTable) {
        atTable = tableGame;
        recamera?.();
      }
      covenTable.visible = tableKey === 'coven';
      landlordTable.visible = tableKey === 'landlord';
      if (tableKey === 'coven') {
        covenTable.sync(covenView, members, rosterMap);
        covenTable.update(time, dt);
      } else if (tableKey === 'landlord') {
        landlordTable.sync(landlordView, members, rosterMap);
        landlordTable.update(time);
      }
      if (room?.phase.name === 'lobby') {
        if (covenView) covenView = null;
        if (landlordView) landlordView = null;
      }
      partyBoard.visible = boardPhase && !tableGame&&!cityWorld;
      if(cityScene)cityScene.visible=cityWorld;
      // A board has its own coast and islets; the arena's little islands are only for bouts.
      for (const islet of stage.islands) islet.visible = !(boardPhase && !tableGame);
      const candyVan=room?.board?.candy_van;
      const candyPaused=!!candyVan;
      const myCandyRun=room?.board?.candy_runs?.find(run=>run.seat===mine?.seat&&!spectating);
      const candyJoined=!!candyVan?.joined_seats.includes(mine?.seat??-1);
      if(candyVan&&!hadCandyVan){sfx?.shop();candyPausedAt=now;}
      if(!candyVan&&hadCandyVan){const frozen=now-candyPausedAt;lobStarted+=frozen;if(chargeFrom!==null)chargeFrom+=frozen;}
      hadCandyVan=candyPaused;
      if(!candyPaused||candyVan?.phase==='visit')candyJoining=false;
      const turnStep = room?.board?.turn;
      const lobStep =
        boardPhase && (turnStep?.step === 'lob' || turnStep?.step === 'swing' || turnStep?.step === 'vacuum') ? turnStep : null;
      const lobSeat = lobStep && mine && !boardPlayback && room?.board?.turn_seat === mine.seat ? mine.seat : null;
      // Anyone else's vacuum is drawn as they aim it; yours is drawn as you do.
      if (lobSeat === null) {
        const walk=room?.board?.walk;
        if(walk && walk.elapsed>=walk.delay){
          const pulling:[number,number][]=[...(walk.pulling??[])];
          for(const coin of walk.coins)if(coin.active&&!coin.collected&&coin.progress>0)pulling.push([pullKey(PULL.air,coin.id),coin.progress]);
          partyBoard.setVacuum(walk.seat,{yaw:walk.yaw,on:walk.vacuum_on,blowing:walk.blowing??false,cone:walk.cone,range:walk.range,rate:walk.rate,power:walk.power??0,out:walk.out??walk.elapsed,left:Math.max(0,walk.delay+walk.path.length*walk.hop_seconds-walk.elapsed),pulling,siphoning:[],braced:[]});
        } else partyBoard.setVacuum(room?.board?.vacuum ? room.board.turn_seat : null, room?.board?.vacuum ?? null);
      }
      const boardView = room?.board ?? (previewing ? room?.preview_board : null) ?? null;
      const nextOrbitView = `${room?.match_id}:${boardView?.map}`;
      if (nextOrbitView !== orbitView) {
        orbitView = nextOrbitView;orbitYaw = 0;orbitPitch = 0;orbitSeat = null;orbitDragging = false;pendingFacing = null;
      }
      if(cityWorld&&boardView){
        voting=false;voteStage.hide();
        if(!cityScene)cityScene=new CityHeistScene(scene,mobile);
        cityScene.visible=true;cityScene.sync(boardView,room?.city??null,members,rosterMap,mine?.seat??null);cityScene.update(time,dt);
      }
      if (boardView&&!cityWorld&&!room?.city_mode) {
        // A game's board left over in the lobby keeps its storm and coins, and
        // a preview has none of a game's, so a switch either way starts afresh.
        const isPreview = !room?.board;
        if (boardMap !== null && (boardMap !== boardView.map || boardIsPreview !== isPreview)) {
          partyBoard.dispose();
          partyBoard = new PartyBoard(scene, { detail: mobile ? 0.45 : 1 });
          partyBoard.sounds = sfx;
          recamera?.();
        }
        boardMap = boardView.map;
        boardIsPreview = isPreview;
        // The ballot, standing over the middle of the island while it is open.
        const ballot = room?.phase.name === 'vote' ? room.phase : null;
        voting = !!ballot && !tableGame;
        if (ballot && voting) {
          const meNow = room?.members.find((m) => m.user_id === me);
          const options = ballot.options.map((key) => {
            const spec = room?.bouts.find((b) => b.key === key);
            return {
              key,
              name: spec?.name ?? key,
              rules: spec?.rules ?? '',
              tag: `${CONTROL_TAGS[spec?.controls ?? 'move'] ?? ''}${spec?.format === 'teams' ? ' · teams' : ''}`,
            };
          });
          voteStage.show(
            partyBoard.bounds().center,
            0,
            options,
            ballot.votes,
            ballot.votes.find((v) => v.seat === meNow?.seat)?.key ?? null,
            (seat) => rosterMap.get(room?.members.find((m) => m.seat === seat)?.character ?? ''),
          );
          voteStage.update(time, hoveredVote);
        } else {
          voteStage.hide();
          if (hoveredVote) canvas.style.cursor = '';
          hoveredVote = null;
        }
        partyBoard.mature = !!room?.mature;
        partyBoard.pairs = room?.pairs ?? [];
        partyBoard.setCloseCamera(!!myCandyRun||!!boardPlayback||phase==='map_finale'||phase==='results'&&!!boardView.finale||lobSeat!==null||!!boardView.walk&&boardView.walk.seat===mine?.seat||!!boardView.raids?.some(raid=>raid.seat===mine?.seat)||!!boardView.driving?.some(driver=>driver.seat===mine?.seat));
        partyBoard.sync(boardView, members, rosterMap);
        partyBoard.update(time, dt);
      }

      const spec = room?.bouts.find((b) => b.key === room?.minigame) ?? null;
      swapBout(spec && !boardPhase ? spec.key : '');
      controlsKind = spec?.controls ?? 'move';
      const fpBout = controlsKind === 'first_person';
      // Practice plays exactly as the bout does; it only does not count.
      const practising = phase === 'practice';
      const playing = phase === 'playing' || practising;
      const inArena = playing || phase === 'countdown';
      const meInRound = !!mine?.in_round && !spectating;
      const myWalk=room?.board?.walk && (partyBoard.walkElapsed()??room.board.walk.elapsed)>=room.board.walk.delay && room.board.walk.seat===mine?.seat && !spectating && !boardPlayback && !candyPaused ? room.board.walk : null;
      if(myWalk&&walkLeg!==myWalk.id){
        const rollSequence=Number(room?.board?.log.findLast(entry=>entry.kind==='roll'&&entry.seat===mine?.seat)?.seq??-1);
        if(walkLeg<0||rollSequence!==walkRollSequence){controls.yaw=myWalk.yaw+Math.PI;controls.pitch=-.15;walkRollSequence=rollSequence;walkFollowing=false;}
        walkLeg=myWalk.id;
      }
      const finale=room?.board?.finale;
      const myRacer=phase==='map_finale'&&!spectating?finale?.racers.find(racer=>racer.seat===mine?.seat):null;
      if(finale)partyBoard.setFinaleViewer(mine?.seat??null);
      const myDriver=room?.board?.driving?.find(driver=>driver.seat===mine?.seat && !spectating && phase==='board'&&!candyPaused);
      if(!myDriver){drivingSeat=null;driveHorn=false;}
      const positioning=room?.board?.positioning;
      const myPosition=positioning && !positioning.active && positioning.seat===mine?.seat && !spectating && !boardPlayback && !candyPaused ? positioning : null;
      const myRaid=room?.board?.raids?.find(raid=>raid.seat===mine?.seat && !spectating && !boardPlayback && !candyPaused);
      const myCrawler=phase==='board'&&!candyPaused&&!spectating&&!boardPlayback&&room?.board?.crawler?.controller===mine?.seat&&!myRaid&&!myPosition&&!myDriver?room?.board?.crawler:null;
      if(myCrawler&&!crawlerEye){controls.yaw=myCrawler.yaw+Math.PI;controls.pitch=-.3;}
      crawlerEye=!!myCrawler;
      const eye=!!myRaid||!!myPosition||!!myCandyRun;
      if(eye&&!itemEye){const direction=camera.getWorldDirection(new THREE.Vector3());controls.yaw=Math.atan2(-direction.x,-direction.z);controls.pitch=Math.asin(direction.y);}
      itemEye=eye;
      if(myCandyRun&&!candyEye&&candyVan){controls.yaw=Math.atan2(candyVan.x-myCandyRun.x,candyVan.z-myCandyRun.z)+Math.PI;controls.pitch=-.08;}
      candyEye=!!myCandyRun;
      controls.enabled = !isGameplayTyping()&&((!!bout && fpBout && meInRound && inArena) || !!myWalk || eye || !!myCrawler);
      if (orbitDragging && !canOrbit()) orbitDragging = false;
      const captureView = myCandyRun?'candy-run':candyPaused&&(candyJoined||candyJoining)?'candy-offer':myCrawler ? `crawler:${myCrawler.controller_round}` : myRaid ? `raid:${myRaid.seat}:${myRaid.item}` : myPosition ? `position:${myPosition.seat}:${myPosition.item}` : myWalk ? `walk:${myWalk.id}` : lobSeat !== null ? `item:${lobStep?.step}` : orbitDragging ? `orbit:${orbitView}` : `bout:${bout?.key}:${phase}`;
      controls.capture(!touch && !isGameplayTyping() && (controls.enabled || orbitDragging || (candyPaused&&(candyJoined||candyJoining)) || (lobSeat !== null && onBoard && !spectating)), captureView);
      if (fpBout || myWalk || eye || myCrawler) controls.readGamepad(dt);
      if (input) input.enabled = !isGameplayTyping()&&((phase==='city_heist'&&!!myCity&&!myCity.automated&&!myCity.jailed&&!room?.city?.resolved&&!spectating) || (!!myRacer&&!myRacer.finished&&!myRacer.dnf) || !!myCandyRun || !!myPosition || !!myRaid || !!myDriver || !!myCrawler || (meInRound && inArena));
      const stick = input?.enabled ? input.read() : { x: 0, z: 0 };

      const playback=bout?connection.replay.sample(bout.module,bout.key,time):null;
      replayStatus=connection.replay.status;
      if(bout && wasReplay!==!!playback){bout.renderer.dispose();bout.renderer=bout.module.create(host);snapCamera=true;wasUp=false;}
      wasReplay=!!playback;
      let view: BoutFrame<unknown, unknown> | null = null;
      if (bout) {
        const sampled = playback?.snapshot ?? connection.sample(bout.module, bout.key);
        const snapshot = sampled && sampled.game === bout.key ? sampled : null;
        view = {
          key: bout.key,
          snapshot,
          setup: room?.setup ?? null,
          phase: (playback ? 'playing' : practising ? 'playing' : inArena || phase === 'results' ? phase : 'lobby') as BoutPhase,
          seats: members,
          mySeat: mine?.seat ?? null,
          winners: room?.phase.name === 'results' ? (room.phase.placements[0] ?? []) : [],
          playing: meInRound && !playback,
          stick,
          look: { yaw: controls.yaw, pitch: controls.pitch, jump: controls.jump, crouch: controls.crouch },
          time:playback?.time??time,
          dt:playback?.dt??dt,
        };
        if (fpBout) {
          const facing = bout.renderer.facing?.(view) ?? null;
          // Coming back in points you at the game, not at a wall.
          if (facing !== null && !wasUp) {
            controls.yaw = facing;
            controls.pitch = 0;
            view.look = { ...view.look, yaw: facing, pitch: 0 };
          }
          wasUp = facing !== null;
        }
        bout.renderer.frame(view);
        if(playback)for(const event of playback.events)bout.renderer.effect?.(event,view);
        const nextHud = bout.renderer.hud?.(view) ?? null;
        const nextHudKey = JSON.stringify(nextHud);
        if (nextHudKey !== hudKey) {
          hudKey = nextHudKey;
          hud = nextHud;
        }
        if (bout.renderer.aim) {
          aimRay.setFromCamera(pointer, camera);
          if (aimRay.ray.intersectPlane(groundPlane, aimPoint)) {
            aimYaw = bout.renderer.aim({ x: aimPoint.x, z: aimPoint.z }, view) ?? aimYaw;
          }
        }
        const clock = snapshot as { time?: number; limit?: number } | null;
        const nextTime = playing && clock?.limit !== undefined ? Math.max(0, Math.ceil(clock.limit - (clock.time ?? 0))) : null;
        if (nextTime !== timeLeft) timeLeft = nextTime;
      } else if (timeLeft !== null) {
        timeLeft = null;
      }
      lastView = view;
      if (scene.fog instanceof THREE.Fog) {
        // The board's whole-island view sits far back, so the haze starts further out.
        // On a board (the lobby preview and the vote too) it follows the camera,
        // which backs off further on the bigger islands.
        const reach = lobbing ? 220 : camera.position.distanceTo(target);
        const [near, far] = bout?.renderer.fog ?? (onBoard ? [Math.max(90, reach * 1.1), Math.max(280, reach * 2.6)] : [45, 120]);
        // Down a tunnel the dark closes in, and the daylight goes.
        scene.fog.near = THREE.MathUtils.lerp(near, 2, caveMix);
        scene.fog.far = THREE.MathUtils.lerp(far, 24, caveMix);
        scene.fog.color.copy(skyFog).lerp(caveFog, caveMix);
        stage.sun.intensity = sunlight * (1 - caveMix * 0.85);
        camera.far = Math.max(400, far + 40);
        camera.fov = (lobbing || myWalk || myRaid || myPosition || myDriver || myCandyRun) ? settings().camera.throwFov : THREE.MathUtils.lerp(40, 58, caveMix);
        camera.updateProjectionMatrix();
      }
      if(cityWorld){scene.background=nightBackground;stage.sun.intensity=.32;for(const light of hemisphere)light.intensity=.26;if(scene.fog instanceof THREE.Fog){scene.fog.color.set('#060d1e');scene.fog.near=15;scene.fog.far=100;}}
      else{scene.background=dayBackground;hemisphere.forEach((light,i)=>light.intensity=hemiDay[i]);}
      stage.update(time);
      const island = partyBoard.bounds();
      coverWithSun(cityWorld?26:onBoard ? Math.ceil(Math.max(island.halfW, island.halfD) + 10) : 15);
      // A close city camera needs detailed shadows around the player rather
      // than drawing the entire unseen city into the sun map every frame.
      if(cityWorld&&cityScene){const focus=cityScene.focus(mine?.seat??null);const x=Math.round(focus.x/4)*4,z=Math.round(focus.z/4)*4;stage.sun.position.set(x+daylightPosition.x,daylightPosition.y,z+daylightPosition.z);stage.sun.target.position.set(x,0,z);}
      else{stage.sun.position.copy(daylightPosition);stage.sun.target.position.copy(daylightTarget);}
      stage.sun.target.updateMatrixWorld();

      const portrait = camera.aspect < 0.8;
      const shot = playback?.focusPoint
        ? {kind:'frame' as const,half:[7,6] as [number,number],pitch:34,portraitPitch:48,target:[playback.focusPoint.x,playback.focusPoint.z] as [number,number],lookHeight:Math.max(0,playback.focusPoint.y)}
        : bout && view && !atTable ? bout.renderer.shot(view) : null;
      const shake = bout?.renderer.shake?.() ?? 0;
      if(candyPaused){throwHud.hide();lobView=null;partyBoard.hideSeat(null);}
      else if (lobStep && lobSeat !== null && onBoard) {
        lobItemLabel = lobStep.step === 'lob' ? (ITEMS[lobStep.item]?.label ?? 'Bomb') : 'Bat';
        if (lobStep.step === 'vacuum') {
          lobFrame(dt,lobSeat,null,lobStep.seconds,'vacuum');
          if(rightVacuumHeld)suckWith(true,false);
          else if(rightVacuumOwned){connection.send({type:'stow_vacuum'});rightVacuumOwned=false;}
        }
        else lobFrame(dt, lobSeat, lobStep.step === 'lob' ? lobStep.wind : null, lobStep.fuse, lobStep.step === 'swing' ? 'swing' : 'throw');
        partyBoard.hideSeat(lobSeat);
        partyBoard.setAimer(lobSeat, lobYaw);
        partyBoard.setClear(lobStep.step !== 'vacuum');
      } else if (lobbing && performance.now() - swungAt > (partyBoard.chase() ? CHASE_HOLD_MS : SWING_HOLD_MS)) {
        partyBoard.clearSightline(null);
        glide = { position: camera.position.clone(), quaternion: camera.quaternion.clone(), fov: camera.fov, t: 0 };
        lobbing = false;
        vacuuming = false;
        sucking = false;
        blowing = false;
        lobView = null;
        throwHud.hide();
        partyBoard.setArc(null);
        partyBoard.hideSeat(null);
        partyBoard.setAimer(null);
        partyBoard.setBatReach(null);
        partyBoard.holdBat(null);
        partyBoard.setClear(false);
        snapCamera = true;
      }
      if(wasWalking && !myWalk && !lobbing){throwHud.hide();partyBoard.hideSeat(null);partyBoard.setAimer(null);partyBoard.setVacuum(null,null);partyBoard.setClear(false);snapCamera=true;walkBlowing=false;}
      wasWalking=!!myWalk;
      if(rightVacuumOwned&&!rightVacuumHeld&&performance.now()-rightVacuumRequestedAt>1200)rightVacuumOwned=false;
      partyBoard.hideBody((myCandyRun??myRaid??myPosition)?.seat??lobSeat);
      if(eye)partyBoard.hideSeat((myCandyRun??myRaid??myPosition)!.seat);
      else if(!myWalk&&!lobbing)partyBoard.hideSeat(null);
      if(myWalk){
        partyBoard.hideSeat(null);
        partyBoard.setClear(false);
        const feet=partyBoard.focus(myWalk.seat);
        feet.y=partyBoard.walkFloor()??feet.y;
        if(!walkFollowing){walkFollow.copy(feet);walkFollowing=true;}else walkFollow.lerp(feet,1-Math.exp(-dt*12));
        const yaw=controls.yaw+Math.PI;
        const facing=new THREE.Vector3(Math.sin(yaw),0,Math.cos(yaw));
        const right=new THREE.Vector3(-facing.z,0,facing.x);
        shoulderAt+=(shoulder-shoulderAt)*(1-Math.exp(-dt*8));
        camera.position.copy(walkFollow).add(new THREE.Vector3(0,2.7,0)).addScaledVector(facing,-3.4).addScaledVector(right,-1.1*shoulderAt);
        camera.rotation.set(controls.pitch,controls.yaw,0,'YXZ');
        const pad=playPad();
        const blowing=walkBlowing || !!pad?.buttons[settings().pad.blow]?.pressed;
        const on=controls.holdingAction || rightVacuumHeld || blowing;
        const pulling: [number,number][]=[...(myWalk.pulling??[])];
        for(const coin of myWalk.coins)if(coin.active&&!coin.collected&&coin.progress>0)pulling.push([pullKey(PULL.air,coin.id),coin.progress]);
        partyBoard.setVacuum(myWalk.seat,{yaw,on,blowing,cone:myWalk.cone,range:myWalk.range,rate:myWalk.rate,power:myWalk.power??(on?1:0),out:myWalk.out??myWalk.elapsed,left:Math.max(0,myWalk.delay+myWalk.path.length*myWalk.hop_seconds-myWalk.elapsed),pulling,siphoning:[],braced:[]},yaw);
        partyBoard.setAimer(myWalk.seat,yaw);
        throwHud.show(camera,{label:blowing?'Vacuum · blowing':'Vacuum',power:0,charging:on,needle:false,calm:true,walking:true,worldReticle:true,cash:true,score:room?.board?.tokens.find(token=>token.seat===myWalk.seat)?.coins??0,collected:myWalk.bonus,goal:myWalk.max_bonus,time,
          notice: myWalk.waiting ? 'Waiting for traffic · keep vacuuming' : room?.board?.air_coins?.length && room.board.air_remaining===0 ? 'Pickup limit reached' : undefined,
          targetProgress:Math.max(0,...pulling.map(([,progress])=>progress)),
          fuseLeft:Math.max(0,myWalk.delay+myWalk.path.length*myWalk.hop_seconds-myWalk.elapsed),fuse:myWalk.delay+myWalk.path.length*myWalk.hop_seconds,
          wind:0,windAngle:0,hint:onPad()?`RIGHT STICK  Aim     HOLD ${padKey('action')}  Suction     HOLD ${padKey('blow')}  Blow`:'LOOK  Aim     HOLD CLICK / SPACE  Suction     HOLD RMB  Suction     ESC  Cursor'});
      } else if(cityWorld&&cityScene){
        target.copy(cityScene.focus(mine?.seat??null));frameBox(8,6,Math.max(26,46+orbitPitch));
        zoom+=(zoomTarget-zoom)*(1-Math.exp(-dt*10));cameraOffset.multiplyScalar(zoom);
        if(snapCamera)follow.copy(target);else follow.lerp(target,1-Math.exp(-dt*10));snapCamera=false;
        camera.position.copy(follow).add(cameraOffset.clone().applyAxisAngle(new THREE.Vector3(0,1,0),orbitYaw));
        camera.position.y=Math.max(camera.position.y,cityScene.environment.groundAt(camera.position.x,camera.position.z)+2);
        camera.lookAt(follow.clone().add(new THREE.Vector3(0,.9,0)));cityYaw=orbitYaw;
        cityScene.environment.clearSightline(camera.position,follow.clone().add(new THREE.Vector3(0,1,0)));
      } else if(finale && (phase==='map_finale'||phase==='results')){
        const watched=myRacer??finale.racers.find(racer=>racer.seat===mine?.seat)??finale.racers[0];
        if(watched){
          target.copy(partyBoard.focus(watched.seat));
          frameBox(9,7,Math.max(26,46+orbitPitch));
          zoom+=(zoomTarget-zoom)*(1-Math.exp(-dt*10));
          cameraOffset.multiplyScalar(zoom);
          if(snapCamera)follow.copy(target);else follow.lerp(target,1-Math.exp(-dt*9));
          snapCamera=false;
          camera.position.copy(follow).add(cameraOffset.clone().applyAxisAngle(new THREE.Vector3(0,1,0),orbitYaw));
          camera.position.y=Math.max(camera.position.y,partyBoard.groundAt(camera.position.x,camera.position.z)+3);
          camera.lookAt(follow.clone().add(new THREE.Vector3(0,.7,0)));
        }
      } else if(myDriver){
        const at=new THREE.Vector3(myDriver.x,partyBoard.groundAt(myDriver.x,myDriver.z)+.6,myDriver.z);
        if(drivingSeat!==myDriver.seat){drivingFocus.copy(at);drivingYaw=myDriver.yaw;drivingSeat=myDriver.seat;}
        drivingFocus.lerp(at,1-Math.exp(-dt*10));drivingYaw+=wrapAngle(myDriver.yaw-drivingYaw)*(1-Math.exp(-dt*6));
        camera.position.copy(drivingFocus).add(new THREE.Vector3(0,3.6,0)).add(new THREE.Vector3(Math.sin(drivingYaw),0,Math.cos(drivingYaw)).multiplyScalar(-6.5));
        camera.lookAt(drivingFocus.clone().add(new THREE.Vector3(0,.7,0)));
      } else if(myCrawler){
        const at=new THREE.Vector3(myCrawler.x,partyBoard.groundAt(myCrawler.x,myCrawler.z),myCrawler.z);
        const yaw=controls.yaw+Math.PI;
        camera.position.copy(at).add(new THREE.Vector3(0,1.8,0)).add(new THREE.Vector3(Math.sin(yaw),0,Math.cos(yaw)).multiplyScalar(-3.4));
        camera.rotation.set(controls.pitch,controls.yaw,0,'YXZ');
      } else if(myCandyRun||myRaid||myPosition){
        const actor=(myCandyRun??myRaid??myPosition)!;
        camera.position.copy(partyBoard.focus(actor.seat)).add(new THREE.Vector3(0,1.45,0));
        camera.rotation.set(controls.pitch,controls.yaw,0,'YXZ');
        partyBoard.faceSeat(actor.seat,controls.yaw+Math.PI);
      } else if (lobbing) {
        // The throw has the camera. Once it is away, the camera stays at your
        // eyes and turns to follow it down.
        const chased = swungAt > lobStarted ? partyBoard.chase() : null;
        if (chased) {
          chaseLook.position.copy(camera.position);
          chaseLook.lookAt(chased);
          camera.quaternion.slerp(chaseLook.quaternion, 1 - Math.exp(-dt * 7));
          partyBoard.clearSightline(camera.position, chased);
        }
      } else if (shot?.kind === 'eye') {
        camera.position.set(shot.x, shot.y, shot.z);
        camera.rotation.set(shot.pitch, shot.yaw, 0, 'YXZ');
        camera.position.x += (Math.random() - 0.5) * shake;
        camera.position.y += (Math.random() - 0.5) * shake;
      } else {
        if (atTable) {
          frameBox(10.5, 10.5, portrait ? 60 : 44);
          lookHeight = 0.6;
          target.set(0, 0, 0);
        } else if (onBoard && voting) {
          // Framing the stage floating over the island.
          frameBox(13, 6.5, portrait ? 34 : 16);
          lookHeight = 3.2;
          target.copy(voteStage.center);
          boardYaw += wrapAngle(0 - boardYaw) * (1 - Math.exp(-dt * 2.6));
        } else if (onBoard) {
          const board = partyBoard.bounds();
          // The board asks for its own shot while something is worth watching.
          const orbiting = orbitSeat !== null && !room?.board?.walk && !boardPlayback;
          const sceneShot = overview ? null : partyBoard.shot();
          const cinema = orbiting && room?.board?.turn.step === 'shop' ? null : sceneShot;
          if (overview) frameBox(board.halfW + 1, board.halfD + 1, portrait ? 66 : 58);
          else if (cinema) {
            const pitch = THREE.MathUtils.lerp(portrait ? BOARD_PITCH_PORTRAIT : BOARD_PITCH, cinema.pitch, cinema.weight);
            frameBox(
              THREE.MathUtils.lerp(BOARD_WIDE, cinema.wide, cinema.weight),
              THREE.MathUtils.lerp(BOARD_TALL, cinema.tall, cinema.weight),
              pitch,
            );
          } else frameBox(BOARD_WIDE, BOARD_TALL, (portrait ? BOARD_PITCH_PORTRAIT : BOARD_PITCH) + (orbiting ? orbitPitch : 0));
          const wantYaw = cinema ? cinema.yaw * cinema.weight : orbiting ? orbitYaw : 0;
          const wantLook = cinema ? 1.1 * cinema.weight : orbiting ? 0.7 : 0;
          boardYaw += wrapAngle(wantYaw - boardYaw) * (1 - Math.exp(-dt * 2.6));
          boardLook += (wantLook - boardLook) * (1 - Math.exp(-dt * 2.6));
          lookHeight = boardLook;
          aimBoard();
          if (orbiting && !cinema) target.copy(partyBoard.focus(orbitSeat));
          zoom += (zoomTarget - zoom) * (1 - Math.exp(-dt * 10));
          if (!cinema && !boardPlayback) cameraOffset.multiplyScalar(zoom);
          if (cinema) target.lerp(cinema.at, cinema.weight);
          if(boardPlayback?.focusPoint){if(boardPlayback.focusSeat!==null)target.copy(partyBoard.focus(boardPlayback.focusSeat));else target.set(boardPlayback.focusPoint.x,partyBoard.groundAt(boardPlayback.focusPoint.x,boardPlayback.focusPoint.z)+1,boardPlayback.focusPoint.z);frameBox(7,6,portrait?48:32);}
        } else if (shot) {
          frameBox(shot.half[0], shot.half[1], portrait ? (shot.portraitPitch ?? shot.pitch) : shot.pitch);
          lookHeight = shot.lookHeight ?? (portrait ? -0.6 : -1.6);
          target.set(shot.target?.[0] ?? 0, 0, shot.target?.[1] ?? 0);
        } else {
          frameBox(11.5, 9.5, portrait ? 52 : 38);
          lookHeight = portrait ? -0.6 : -1.6;
          target.set(0, 0, 0);
        }
        if (snapCamera || boardPlayback) follow.copy(target);
        else follow.lerp(target, 1 - Math.exp(-dt * 3.4));
        snapCamera = false;
        camera.rotation.set(0, 0, 0);
        if (onBoard && !overview) {
          camera.position
            .copy(follow)
            .add(cameraOffset.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), boardYaw));
        } else if (atTable) {
          // Your own seat sits at the bottom of the screen, whichever it is.
          const seatedPeople = [...members].sort((a, b) => a.seat - b.seat);
          const tableView = room?.minigame === 'landlord' ? landlordView : covenView;
          const mySeat = tableView ? tableView.me : seatedPeople.findIndex((m) => m.user_id === me);
          const table = room?.minigame === 'landlord' ? landlordTable : covenTable;
          const yaw = table.seatAngle(mySeat === null || mySeat < 0 ? null : mySeat);
          camera.position.copy(follow).add(cameraOffset.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), yaw));
        } else {
          camera.position.copy(follow).add(cameraOffset);
        }
        camera.position.x += (Math.random() - 0.5) * shake;
        camera.position.y += (Math.random() - 0.5) * shake;
        camera.position.z += (Math.random() - 0.5) * shake;
        if (onBoard) {
          // The island has hills now, so keep the shot above the ground.
          const floor = partyBoard.groundAt(camera.position.x, camera.position.z) + 3;
          camera.position.y = Math.max(camera.position.y, floor);
        }
        lookAt.set(follow.x, follow.y + lookHeight, follow.z);
        // Whoever's turn it is goes down a tunnel, and the camera goes after them.
        const cave = onBoard && !overview ? partyBoard.cave(room?.board?.turn_seat ?? null) : null;
        if (cave) {
          if (caveMix < 0.01) {
            caveEye.copy(cave.eye);
            caveLook.copy(cave.look);
          }
          caveEye.lerp(cave.eye, 1 - Math.exp(-dt * 5));
          caveLook.lerp(cave.look, 1 - Math.exp(-dt * 6));
        }
        caveMix += ((cave ? 1 : 0) - caveMix) * (1 - Math.exp(-dt * (cave ? 3.2 : 2.4)));
        if (caveMix < 0.002) caveMix = 0;
        sfx?.underground(caveMix, dt);
        if (caveMix > 0) {
          camera.position.lerp(caveEye, caveMix);
          lookAt.lerp(caveLook, caveMix);
        }
        camera.lookAt(lookAt);
        // A close shot on the board moves whatever trees stand in front of it.
        const close = onBoard && !overview && !lobbing ? partyBoard.shot() : null;
        if (close && close.weight > 0.3) partyBoard.clearSightline(camera.position, close.at);
        else if (onBoard && !overview && !lobbing) {
          // Zoomed in among the town's buildings, the camera mustn't end up
          // inside one, or have one between it and whoever's turn it is.
          partyBoard.clearSightline(camera.position, partyBoard.focus(room?.board?.turn_seat ?? null).add(new THREE.Vector3(0, 1.2, 0)));
        } else if (!lobbing) partyBoard.clearSightline(null);
        if (glide) {
          // Back from the throw camera to the board's, eased rather than cut.
          glide.t = Math.min(1, glide.t + dt / 0.9);
          const ease = glide.t * glide.t * (3 - 2 * glide.t);
          camera.position.lerpVectors(glide.position, camera.position.clone(), ease);
          camera.quaternion.slerpQuaternions(glide.quaternion, camera.quaternion.clone(), ease);
          camera.fov = THREE.MathUtils.lerp(glide.fov, camera.fov, ease);
          camera.updateProjectionMatrix();
          if (glide.t >= 1) glide = null;
        }
      }
      const cameraMode=cityWorld?'city-heist':myCandyRun?'candy-run':candyPaused?'candy-paused':phase==='map_finale'||(phase==='results'&&!!finale)?'map-finale':myDriver?'driving':myCrawler?'crawler-control':myRaid?'raid':boardPlayback?'board-replay':playback?'bout-replay':myWalk?'walking-vacuum':lobbing?`item:${lobStep?.step}`:myPosition?'position':atTable?'table':onBoard?(overview?'overview':voting?'vote':partyBoard.shot()?'board-cutscene':caveMix>0.02?'cave':'board'):`bout:${bout?.key}:${shot?.kind??'frame'}`;
      cameraFlow.apply(camera,cameraMode,now/1000);
      throwHud.refit(camera);
      if(myCandyRun&&candyVan){const direction=camera.getWorldDirection(new THREE.Vector3());candyBearing=wrapAngle(Math.atan2(direction.x,direction.z)-Math.atan2(candyVan.x-myCandyRun.x,candyVan.z-myCandyRun.z));}
      const listening=cityWorld&&cityScene?cityScene.focus(mine?.seat??null):partyBoard.focus(mine?.seat??room?.board?.turn_seat??null);
      const engineCars=[...(room?.board?.traffic?.cars??[]),...(room?.board?.driving??[]).map(driver=>({id:100+driver.seat,x:driver.x,z:driver.z,speed:driver.speed,braking:Math.abs(driver.speed)<.2}))];
      sfx?.cityFrame(dt,{x:listening.x,z:listening.z,yaw:camera.rotation.y},onBoard&&!boardPlayback?engineCars:[],onBoard&&!boardPlayback?(room?.board?.crawler??null):null,onBoard&&!boardPlayback?(room?.board?.pedestrians??[]):[],onBoard&&!boardPlayback&&(cityWorld||boardView?.map==='frozen'));
      // Stationary own characters always face the actual rendered camera.
      if(onBoard && phase==='board' && mine && !spectating && !boardPlayback && !room?.board?.walk && !myRaid && !myDriver && !myCrawler && !positioning && !lobbing && !partyBoard.shot() && !overview) {
        const at=partyBoard.focus(mine.seat);
        const yaw=Math.atan2(camera.position.x-at.x,camera.position.z-at.z);
        if(Math.abs(wrapAngle(yaw-partyBoard.facing(mine.seat)))>0.025){pendingFacing=yaw;partyBoard.faceSeat(mine.seat,yaw);}
      }
      const renderRatio=frameBudget.sample(now);if(renderRatio!==null)manager.setPixelRatio(renderRatio);
      if(now-lastShadowAt>1000/(mobile?15:30)){renderer.shadowMap.needsUpdate=true;lastShadowAt=now;}
      manager.render();

      if (pendingFacing !== null && phase==='board' && now - lastFacingSent >= 90 && !room?.board?.walk && !positioning && !myRaid && !myDriver && !lobbing && !boardPlayback) {
        connection.send({type:'face',yaw:pendingFacing});lastFacingSent = now;pendingFacing = null;
      }

      // A controller looks round the board with its right stick, as a mouse
      // drag does, whatever prompt is up; clicking the stick in puts it back.
      const lookPad=playPad();
      if(lookPad&&canOrbit()){
        const [, , rx=0, ry=0]=lookPad.axes;const {deadzone}=settings().pad;const {sensitivity,invertY}=settings().camera;
        const sx=Math.abs(rx)>deadzone?rx:0,sy=Math.abs(ry)>deadzone?ry:0;
        if(sx||sy){
          orbitYaw-=sx*dt*2.6*sensitivity;
          orbitPitch=THREE.MathUtils.clamp(orbitPitch+sy*dt*55*sensitivity*(invertY?-1:1),-15,25);
          orbitSeat=room!.phase.name==='map_finale'?(mySeatNow??room!.board!.turn_seat):room!.board!.turn_seat;
        }
        if(lookPad.buttons[11]?.pressed){orbitYaw=0;orbitPitch=0;orbitSeat=null;}
      }
      // The candy van on a controller: its action button joins the visit, then buys.
      const vanPress=!!playPad()?.buttons[settings().pad.action]?.pressed;
      if(vanPress&&!vanPadHeld&&room?.phase.name==='board'){const van=room.board?.candy_van;if(van?.phase==='offer'&&van.can_join)send({type:'join_candy_visit'});else if(van?.phase==='visit'&&van.can_buy)send({type:'buy_candy'});}
      vanPadHeld=vanPress;
      // A controller's action button acts in the bouts that take one, as the key does.
      const padAction=!!playPad()?.buttons[settings().pad.action]?.pressed;
      if(padAction&&!padActionHeld)askAction();
      padActionHeld=padAction;
      if(phase==='city_heist'){
        const vehiclePressed=!!playPad()?.buttons[3]?.pressed;
        if(vehiclePressed&&!cityVehiclePadHeld&&!isGameplayTyping())send({type:'city_vehicle'});
        cityVehiclePadHeld=vehiclePressed;
        if(now-lastSendAt>=50){const pad=playPad();const movable=!!myCity&&!myCity.automated&&!myCity.jailed&&!room?.city?.resolved&&!spectating;
          connection.send({type:'input',x:movable?stick.x*Math.cos(orbitYaw)+stick.z*Math.sin(orbitYaw):0,z:movable?-stick.x*Math.sin(orbitYaw)+stick.z*Math.cos(orbitYaw):0,yaw:orbitYaw+Math.PI});
          connection.send({type:'city_action',held:movable&&!isGameplayTyping()&&(cityHeld||!!pad?.buttons[0]?.pressed),suction:movable&&!isGameplayTyping()&&(citySuction||rightVacuumHeld||!!pad?.buttons[7]?.pressed)});
          lastSendAt=now;
        }return;
      }
      if(myCandyRun){if(now-lastCandySend>=50){const yaw=controls.yaw;connection.send({type:'candy_visit_input',x:stick.x*Math.cos(yaw)+stick.z*Math.sin(yaw),z:-stick.x*Math.sin(yaw)+stick.z*Math.cos(yaw)});lastCandySend=now;}return;}
      if(candyPaused)return;
      if(myWalk){
        // Mouse holds clear on unlock; touch and controller holds work without
        // requiring the browser's mouse-only pointer-lock permission.
        if(now-lastWalkSend>=50){connection.send({type:'walk_vacuum',yaw:controls.yaw+Math.PI,pitch:controls.pitch,on:controls.holdingAction||rightVacuumHeld||walkBlowing||!!playPad()?.buttons[settings().pad.blow]?.pressed,blow:walkBlowing||!!playPad()?.buttons[settings().pad.blow]?.pressed});lastWalkSend=now;}
        return;
      }
      if(phase==='map_finale'){
        if(myRacer && now-lastSendAt>=50){
          const movable=!myRacer.finished&&!myRacer.dnf&&(finale?.ready??0)<=0&&myRacer.stun<=0;
          const x=movable?stick.x:0,z=movable?stick.z:0;
          connection.send({type:'input',x:x*Math.cos(orbitYaw)+z*Math.sin(orbitYaw),z:-x*Math.sin(orbitYaw)+z*Math.cos(orbitYaw)});
          lastSendAt=now;
        }
        return;
      }
      if(myDriver){
        if(now-lastDriveSend>=50){
          const pad=playPad();const triggers=(pad?.buttons[7]?.value??0)-(pad?.buttons[6]?.value??0);
          connection.send({type:'drive_input',throttle:Math.abs(triggers)>.1?triggers:-stick.z,steer:stick.x,horn:driveHorn||!!pad?.buttons[settings().pad.action]?.pressed});
          lastDriveSend=now;
        }
        return;
      }
      if(myCrawler){if(now-lastCrawlerSend>=50){const yaw=controls.yaw;connection.send({type:'crawler_input',x:stick.x*Math.cos(yaw)+stick.z*Math.sin(yaw),z:-stick.x*Math.sin(yaw)+stick.z*Math.cos(yaw)});lastCrawlerSend=now;}return;}
      if(myRaid){
        if(now-lastRaidSend>=50){
          const yaw=controls.yaw;
          connection.send({type:'raid_input',x:stick.x*Math.cos(yaw)+stick.z*Math.sin(yaw),z:-stick.x*Math.sin(yaw)+stick.z*Math.cos(yaw),yaw:yaw+Math.PI});
          lastRaidSend=now;
        }
        if(myRaid.item==='drive_by'){const firing=controls.holdingAction;if(firing||firing!==lastRaidFiring)connection.send({type:'raid_fire',on:firing,yaw:controls.yaw+Math.PI});lastRaidFiring=firing;}
        else if(controls.holdingAction && now-lastRaidSlash>=620){connection.send({type:'raid_slash'});lastRaidSlash=now;}
        return;
      }
      if(myPosition){
        if(now-lastPositionSend>=50){
          const yaw=controls.yaw;
          connection.send({type:'position_input',x:stick.x*Math.cos(yaw)+stick.z*Math.sin(yaw),z:-stick.x*Math.sin(yaw)+stick.z*Math.cos(yaw)});
          lastPositionSend=now;
        }
        wasPositioning=true;
        return;
      }
      if(wasPositioning){connection.send({type:'position_input',x:0,z:0});wasPositioning=false;}
      if(playback||boardPlayback)return;
      if (!input || !bout) return;
      if (fpBout) {
        // Where you are looking changes constantly, so send at a steady rate.
        if (now - lastSendAt > 45) {
          connection.send({ type: 'input', x: stick.x, z: stick.z, yaw: controls.yaw, pitch: controls.pitch, jump: controls.jump, crouch: controls.crouch });
          lastSent = stick;
          lastSendAt = now;
        }
        if (controls.takeShot(now)) {
          connection.send({ type: 'fire', yaw: controls.yaw, pitch: controls.pitch });
        }
        return;
      }
      if (actionPressed) {
        actionPressed = false;
        if (input.enabled) connection.send({ type: 'fire', yaw: aimYaw, pitch: 0 });
      }
      const changed = Math.abs(stick.x - lastSent.x) + Math.abs(stick.z - lastSent.z) > 0.04;
      const moving = stick.x !== 0 || stick.z !== 0;
      // A turret keeps turning even when the driver does not.
      const turning = !!bout.renderer.aim && input.enabled && now - lastSendAt > 60;
      if ((changed && now - lastSendAt > 33) || (moving && now - lastSendAt > 250) || turning) {
        connection.send({ type: 'input', x: stick.x, z: stick.z, yaw: aimYaw });
        lastSent = stick;
        lastSendAt = now;
      }
    };
    frame = requestAnimationFrame(loop);

    return () => {
      stopped = true;
      captureCandy=null;
      cancelAnimationFrame(frame);
      document.removeEventListener('fullscreenchange', onFullscreenChange);
      document.removeEventListener('keydown', onEscape);
      resizeRoot = null;
      viewport.destroy();
      clearTimeout(passFallback);
      if (errorTimer) clearTimeout(errorTimer);
      if (goTimer) clearTimeout(goTimer);
      resize.disconnect();
      canvas.removeEventListener('mousedown', askAction);
      canvas.removeEventListener('click', onTableClick);
      canvas.removeEventListener('click', onBoardClick);
      canvas.removeEventListener('pointerdown',onWalkBlow);
      window.removeEventListener('pointerup',onWalkBlowUp);
      window.removeEventListener('keydown',onWalkBlowKey);
      window.removeEventListener('keyup',onWalkBlowKey);
      window.removeEventListener('blur',clearWalkBlow);
      document.removeEventListener('pointerlockchange',unlockWalkBlow);
      throwHud.dispose();
      voteStage.dispose();
      canvas.removeEventListener('pointermove', onVoteMove);
      canvas.removeEventListener('click', onVoteClick);
      canvas.removeEventListener('pointerdown', onBoutClick);
      document.removeEventListener('mousemove', onLobAim);
      canvas.removeEventListener('pointermove', onLobTouchAim);
      document.removeEventListener('pointerlockchange', onLobUnlock);
      window.removeEventListener('blur', cancelLobInput);
      document.removeEventListener('focusin',onTypingFocus);
      canvas.removeEventListener('pointerdown', onLobDown);
      canvas.removeEventListener('contextmenu', onLobMenu);
      window.removeEventListener('pointerup', throwNow);
      window.removeEventListener('keydown', onLobKey);
      window.removeEventListener('keydown', onVacuumKey);
      window.removeEventListener('keyup', onLobKey);
      window.removeEventListener('keydown', onBraceKey);
      window.removeEventListener('keyup', onBraceKey);
      canvas.removeEventListener('pointerdown', onFlyDown);
      window.removeEventListener('pointermove', onFlyMove);
      window.removeEventListener('pointerup', onFlyUp);
      canvas.removeEventListener('mousemove', onPointerMove);
      canvas.removeEventListener('pointerdown', onOrbitDown);
      canvas.removeEventListener('pointercancel', onOrbitCancel);
      canvas.removeEventListener('auxclick', onOrbitAux);
      canvas.removeEventListener('wheel', onOrbitWheel);
      window.removeEventListener('pointerup', onOrbitUp);
      window.removeEventListener('blur', onOrbitCancel);
      document.removeEventListener('pointerlockchange', onOrbitUnlock);
      window.removeEventListener('keydown',onCandyKey);
      window.removeEventListener('keydown',onCityKey);window.removeEventListener('keyup',onCityKey);window.removeEventListener('blur',clearCity);
      window.removeEventListener('keydown',onDriveKey);window.removeEventListener('keyup',onDriveKey);window.removeEventListener('blur',releaseDriveHorn);
      window.removeEventListener('keydown', onActionKey);
      recamera = null;
      if (hookHost.__party === testHook) delete hookHost.__party;
      connection?.close();
      connection = null;
      input?.dispose();
      sfx?.dispose();
      bout?.renderer.dispose();
      bout = null;
      controls.dispose();
      partyBoard.dispose();cityScene?.dispose();
      covenTable.dispose();
      landlordTable.dispose();
      stage.dispose();
      manager.dispose();
    };
  });
</script>

<div
  bind:this={root}
  class="party-root w-full overflow-hidden bg-sky-300 {expanded ? 'is-expanded fixed inset-0 z-[100]' : 'relative'}"
>
  <canvas bind:this={canvas} class="block h-full w-full touch-none" aria-label="Tawkathon arena"></canvas>
  {#if !parked}
  {#if tip}
    <!-- Beside the pointer, flipped to its left past the middle so it stays on screen. -->
    <div
      class="pointer-events-none absolute z-40 max-w-64 rounded-xl border border-white/10 bg-slate-950/90 px-3 py-2 text-xs text-slate-300 shadow-lg backdrop-blur"
      style="top: {tip.y + 16}px; {tip.x > (canvas?.clientWidth ?? 0) / 2 ? `right: ${(canvas?.clientWidth ?? 0) - tip.x + 16}px` : `left: ${tip.x + 16}px`}"
      role="tooltip"
    >
      <div class="font-bold text-white">{tip.title}</div>
      <div class="mt-0.5 leading-snug">{tip.body}</div>
    </div>
  {/if}
  {#if drainingMe}
    <div
      class="pointer-events-none absolute top-[18%] left-1/2 z-30 -translate-x-1/2 rounded-full border px-4 py-2 text-sm font-bold backdrop-blur {bracingNow
        ? 'border-emerald-300/40 bg-emerald-950/80 text-emerald-200'
        : 'animate-pulse border-rose-300/40 bg-rose-950/80 text-rose-100'}"
      aria-live="polite"
    >
      {bracingNow
        ? 'Holding on: you keep half your coins'
        : `Your coins are being sucked away. Hold ${keyName(settings().keys.action[0] ?? 'Space')} to brace`}
    </div>
  {/if}
  {#if hud?.award}
    {#key hud.award.id}
      <div class="target-award pointer-events-none absolute left-1/2 top-[38%] z-30 -translate-x-1/2 text-center" aria-live="polite">
        <div class="text-5xl font-black tabular-nums text-amber-300 drop-shadow-[0_4px_0_rgba(15,23,42,0.9)]">+{hud.award.points}</div>
        <div class="mt-1 rounded-full border border-white/20 bg-slate-950/75 px-3 py-1 text-xs font-bold uppercase tracking-[0.16em] text-emerald-200 backdrop-blur">
          {hud.award.caption}
        </div>
      </div>
    {/key}
  {/if}
  <RoomHud
    {room}
    {roster}
    {me}
    {spectating}
    {status}
    hasPass={!!pass}
    {timeLeft}
    {showGo}
    {muted}
    {filming}
    {guest}
    {touch}
    {overview}
    {toggleOverview}
    {fullscreen}
    {toggleFullscreen}
    {pointerLocked}
    {briefHidden}
    {replayStatus}
    onskipreplay={()=>connection?.replay.skip()}
    {hud}
    {firstPerson}
    onlook={(dx, dy) => look(dx, dy)}
    onfire={(down) => trigger(down)}
    oncrouch={(down) => duck(down)}
    onaim={(item) => (aimItem = item)}
    {lobView}
    {teleporting}
    onteleport={(on) => (teleporting = on)}
    {errorMessage}
    {feedbackResult}
    {onpopout}
    {poppedOut}
    {send}
    toggleMute={() => {
      muted = !muted;
      sfx?.setMuted(muted);
    }}
    onstick={(value) => input?.setTouch(value)}
  />
  {#if room?.phase.name==='board'&&!room.board?.candy_van&&room.board?.crawler&&!room.board.raids?.some(r=>r.seat===mySeatNow)&&!room.board.driving?.some(d=>d.seat===mySeatNow)}<ZombieControlHud crawler={room.board.crawler} seat={mySeatNow} members={room.members} onexit={room.sandbox?()=>send({type:'dev',tool:{action:'crawler'}}):undefined}/>{/if}
  {#if room?.phase.name==='board'}<CandyVanHud {room} seat={mySeatNow} {send} bearing={candyBearing}/>{/if}
  {#if room?.phase.name==='city_heist'&&room.city}<CityHud city={room.city} seat={mySeatNow} members={room.members} {send} yaw={cityYaw} {touch} onstick={value=>input?.setTouch(value)} onsuction={held=>citySuction=held} onhold={held=>cityHeld=held}/>{/if}
  {#if room?.phase.name==='map_finale'&&room.board?.finale}<FinaleHud finale={room.board.finale} seat={mySeatNow} members={room.members} {spectating} {touch} oncountdown={count=>sfx?.count(count===0)}/>{/if}
  {#if myDrivingNow&&!spectating}<DrivingHud driver={myDrivingNow} onhorn={setDriveHorn}/>{/if}
  {#if !spectating}<RaidHud {room} seat={mySeatNow} {send}/>{/if}
  {#if room?.minigame === 'coven'}
    <CovenHud view={covenView} {room} hit={covenHit} {send} />
  {:else if room?.minigame === 'landlord'}
    <LandlordHud view={landlordView} {room} hit={covenHit} {send} />
  {/if}
  {/if}
</div>

<style>
  /* The lobby page keeps about 10rem of header, tabs and footer around
     the party, and anything taller is cut off at the bottom. */
  .party-root {
    height: calc(100dvh - 10.25rem);
    max-height: 70rem;
    min-height: 26rem;
    contain: layout paint;
  }

  .party-root.is-expanded,
  .party-root:fullscreen {
    width: 100vw;
    height: 100dvh;
    max-height: none;
    min-height: 0;
  }

  @media (max-width: 640px), (max-height: 640px) {
    .party-root {
      height: calc(100dvh - 10.25rem);
      min-height: 0;
    }
  }

  .target-award {
    animation: target-award 1.25s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  @keyframes target-award {
    0% { opacity: 0; transform: translate(-50%, 18px) scale(0.7); }
    16% { opacity: 1; transform: translate(-50%, 0) scale(1.12); }
    72% { opacity: 1; transform: translate(-50%, -8px) scale(1); }
    100% { opacity: 0; transform: translate(-50%, -28px) scale(0.94); }
  }
</style>

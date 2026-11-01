import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ReplayDirector, REPLAY_LIMITS, replaySpeed } from '../src/party/replay.ts';

const snap = (time: number, x = time): any => ({game:'shooter',time,limit:30,players:[{seat:0,x,y:0,z:0,alive:true},{seat:1,x:3,y:0,z:2,alive:time<2}]});
const room = (phase: string, seconds = 6, key = 'shooter'): any => ({minigame:key,phase:{name:phase,seconds,placements:[[0],[1]]}});
const blend: any = {blend:(a:any,b:any,t:number)=>({
  ...b,
  time:a.time+(b.time-a.time)*t,
  players:b.players.map((p:any,i:number)=>({...p,x:a.players[i].x+(p.x-a.players[i].x)*t})),
})};

function recorded() {
  let clock=100;
  const replay=new ReplayDirector(()=>clock);
  replay.observeRoom(room('playing'));
  for(let i=0;i<=40;i++)replay.record(i*.05,snap(i*.05),i===40?[{kind:'down',seat:1,by:0}]:[]);
  const results=()=>{clock=102;replay.observeRoom(room('results'));replay.record(2.05,snap(2.05),[]);replay.record(2.1,snap(2.1),[]);replay.record(2.15,snap(2.15),[]);};
  return {replay,results,get clock(){return clock},set clock(value:number){clock=value}};
}

test('speed ramps continuously through 20%, with gentle entrances and exits',()=>{
  assert.equal(replaySpeed(-1),1);
  assert.equal(replaySpeed(0),.2);
  assert.equal(replaySpeed(.5),1);
  assert.ok(replaySpeed(-.35)>.2&&replaySpeed(-.35)<1);
  for(const boundary of [-.55,-.12,.08,.42])assert.ok(Math.abs(replaySpeed(boundary-1e-6)-replaySpeed(boundary+1e-6))<1e-4);
  assert.equal(replaySpeed(.15,.15),1,'a short actual post-roll still eases back to normal');
});

test('rewinds actual recorded snapshots instead of merely slowing future frames',()=>{
  const x=recorded();x.results();
  assert.equal(x.replay.pending,true);
  assert.equal(x.replay.sample(blend,'shooter',102.1),null);
  const first=x.replay.sample(blend,'shooter',102.151)!;
  assert.ok(first.snapshot.players[0].x<1,'lead-in precedes the killing blow');
  assert.equal(first.speed,1);
  assert.equal(first.label,'The finishing shot');
  assert.equal(first.focusSeat,1);
  assert.deepEqual(first.focusPoint,{x:3,y:0,z:2});
});

test('historical effects play exactly once as source time crosses the impact',()=>{
  const x=recorded();x.results();
  const seen:any[]=[];let slow=false,gradual=false,ended=false;
  for(let i=0;i<400;i++){
    const view=x.replay.sample(blend,'shooter',102.151+i/60);
    if(view){seen.push(...view.events);slow ||=view.speed<.205;gradual ||=view.speed>.21&&view.speed<.95;assert.ok(view.dt>=0&&view.dt<=.1);}
    else if(i>1&&!x.replay.pending){ended=true;break;}
  }
  assert.equal(seen.filter(e=>e.kind==='down').length,1);
  assert.equal(slow,true);assert.equal(gradual,true);assert.equal(ended,true);
});

test('the final event arriving after the results room message is included',()=>{
  let now=0;const replay=new ReplayDirector(()=>now);replay.observeRoom(room('playing'));
  for(let i=0;i<40;i++)replay.record(i*.05,snap(i*.05),[]);
  replay.observeRoom(room('results'));
  replay.record(2,snap(2),[{kind:'down',seat:1,by:0}]);
  replay.record(2.1,snap(2.1),[]);
  const first=replay.sample(blend,'shooter',.151);
  assert.equal(first?.label,'The finishing shot');
});

test('live players and practice sessions never have their presentation replaced',()=>{
  const x=recorded();assert.equal(x.replay.sample(blend,'shooter',101),null);
  const replay=new ReplayDirector(()=>0);replay.observeRoom(room('practice'));
  for(let i=0;i<40;i++)replay.record(i*.05,snap(i*.05),[{kind:'down',seat:1,by:0}]);
  replay.observeRoom(room('results'));assert.equal(replay.pending,false);assert.equal(replay.retained.frames,0);
});

test('skip is final for this result phase and a subsequent round cancels immediately',()=>{
  const x=recorded();x.results();x.replay.sample(blend,'shooter',102.151);
  x.replay.skip();x.replay.observeRoom(room('results',5));
  assert.equal(x.replay.sample(blend,'shooter',103),null);
  x.replay.observeRoom(room('countdown'));assert.equal(x.replay.active,false);assert.equal(x.replay.pending,false);
});

test('new phase, game, or reset never leaks a previous replay',()=>{
  for(const next of [room('playing'),room('lobby'),room('board'),room('results',6,'dodgeball')]){
    const x=recorded();x.results();x.replay.sample(blend,'shooter',102.151);assert.equal(x.replay.active,true);
    x.replay.observeRoom(next);assert.equal(x.replay.active,false);assert.equal(x.replay.pending,false);
  }
  const x=recorded();x.results();x.replay.reset();assert.equal(x.replay.retained.frames,0);assert.equal(x.replay.status,null);
});

test('finishes before the next round, preserving a full second of actual results',()=>{
  const x=recorded();x.results();x.replay.sample(blend,'shooter',102.151);
  assert.equal(x.replay.sample(blend,'shooter',107),null);
  assert.equal(x.replay.active,false);
  const late=recorded();late.results();assert.equal(late.replay.sample(blend,'shooter',106.9),null);
});

test('recording and playback cannot mutate retained or live snapshots',()=>{
  let now=0;const replay=new ReplayDirector(()=>now);replay.observeRoom(room('playing'));
  const originals:any[]=[];
  for(let i=0;i<=40;i++) {const frame=snap(i*.05);originals.push(frame);replay.record(i*.05,frame,i===40?[{kind:'down',seat:1,by:0}]:[]);}
  replay.observeRoom(room('results'));replay.record(2.1,snap(2.1),[]);
  originals.forEach(s=>s.players[0].x=999);
  const first=replay.sample(blend,'shooter',.151)!;
  assert.ok(first.snapshot.players[0].x<2);
  first.snapshot.players[0].x=777;
  const next=replay.sample(blend,'shooter',.16)!;
  assert.ok(next.snapshot.players[0].x<2);
});

test('history stays bounded by time, frame count, event count and bytes',()=>{
  const replay=new ReplayDirector(()=>0);replay.observeRoom(room('playing'));
  for(let i=0;i<3000;i++)replay.record(i*.01,snap(i*.01),[{kind:'shot',seat:0,from:[0,0,0],to:[1,0,0],hit:false}],90000);
  assert.ok(replay.retained.frames<=REPLAY_LIMITS.frames);
  assert.ok(replay.retained.events<=REPLAY_LIMITS.events);
  assert.ok(replay.retained.bytes<=REPLAY_LIMITS.bytes);
  replay.record(31,snap(31),[],REPLAY_LIMITS.bytes+1);
  assert.equal(replay.retained.frames,0);
});

test('boring shots and late joins do not manufacture a highlight',()=>{
  const replay=new ReplayDirector(()=>0);replay.observeRoom(room('playing'));
  for(let i=0;i<40;i++)replay.record(i*.05,snap(i*.05),[]);
  replay.observeRoom(room('results'));assert.equal(replay.sample(blend,'shooter',.151),null);
  const joined=new ReplayDirector(()=>0);joined.observeRoom(room('results'));assert.equal(joined.pending,false);
});

test('winning finish outranks a later consolation finish',()=>{
  const replay=new ReplayDirector(()=>0);replay.observeRoom(room('playing',6,'race'));
  for(let i=0;i<80;i++)replay.record(i*.05,{...snap(i*.05),game:'race'} as any,i===40?[{kind:'finish',seat:0,place:1}]:i===60?[{kind:'finish',seat:1,place:2}]:[]);
  replay.observeRoom(room('results',6,'race'));
  assert.equal(replay.sample(blend,'race',.151)?.label,'Winning finish');
});

const boardRoom=(turn=1,log:any[]=[],match='board-match'):any=>({
  ...room('board'),match_id:match,members:[{user_id:'viewer',seat:0},{user_id:'friend',seat:1}],
  board:{map:'island',turn_seat:turn,turn:{step:'items'},log,tokens:[{seat:0,space:0},{seat:1,space:1}],spaces:[{id:0,x:0,z:0},{id:1,x:3,z:2}]},
});
function recordedBoard(turn=1){
  let now=0;const replay=new ReplayDirector(()=>now);replay.setViewer('viewer');
  const logs:any[]=[];
  for(let i=0;i<=40;i++){now=i*.05;replay.observeRoom(boardRoom(turn,logs));}
  logs.push({kind:'item',by:1,item:'bat',target:0},{kind:'knockout',seat:0,coins:10});
  now=2.05;replay.observeRoom(boardRoom(turn,logs));
  for(let i=42;i<=49;i++){now=i*.05;replay.observeRoom(boardRoom(turn,logs));}
  return {replay,logs,get now(){return now},set now(t:number){now=t}};
}

test('board knockout rewinds an actual room log, with a gradual 20% impact',()=>{
  const x=recordedBoard();assert.equal(x.replay.pending,true);
  const first=x.replay.sampleBoard(x.now)!;
  assert.equal(first.room.board!.log.length,0,'the lead-in predates the actual hit');
  assert.equal(first.focusSeat,0);assert.deepEqual(first.focusPoint,{x:0,y:0,z:0});
  let slow=false,ramp=false,impact=false,finished=false;
  for(let i=1;i<400;i++){
    const sample=x.replay.sampleBoard(x.now+i/60);
    if(!sample){finished=true;break;}
    slow ||=sample.speed<.205;ramp ||=sample.speed>.21&&sample.speed<.95;
    impact ||=sample.room.board!.log.some((entry:any)=>entry.kind==='knockout');
    assert.ok(sample.dt<=.1);
  }
  assert.equal(slow,true);assert.equal(ramp,true);assert.equal(impact,true);assert.equal(finished,true);
});

test('an attacker keeps their action controls, then sees the saved impact after the turn passes',()=>{
  const x=recordedBoard(0);
  assert.equal(x.replay.pending,false);assert.equal(x.replay.active,false);assert.equal(x.replay.sampleBoard(x.now),null);
  // Wait longer than the normal rolling buffer: the clip was frozen separately.
  x.now=12;x.replay.observeRoom(boardRoom(1,x.logs));
  assert.equal(x.replay.pending,true);
  const view=x.replay.sampleBoard(x.now)!;assert.ok(view.time<2.05);
  assert.equal(view.room.board!.turn_seat,0,'presentation history preserves the actual prior turn');
});

test('your turn beginning cancels a board replay before the live room callback',()=>{
  const x=recordedBoard();x.replay.sampleBoard(x.now);assert.equal(x.replay.active,true);
  x.now+=.1;x.replay.observeRoom(boardRoom(0,x.logs));
  assert.equal(x.replay.active,false);assert.equal(x.replay.pending,false);
  assert.equal(x.replay.sampleBoard(x.now),null);
});

test('late joins, repeated log packets and skipped board highlights do not loop old knockouts',()=>{
  const replay=new ReplayDirector(()=>0);replay.setViewer('viewer');
  replay.observeRoom(boardRoom(1,[{kind:'knockout',seat:0,coins:10}]));
  assert.equal(replay.pending,false);
  const x=recordedBoard();x.replay.skip();
  x.now+=.1;x.replay.observeRoom(boardRoom(1,x.logs));
  assert.equal(x.replay.pending,false);assert.equal(x.replay.sampleBoard(x.now),null);
});

test('board replay room snapshots are isolated from live state and renderer changes',()=>{
  const x=recordedBoard();const view=x.replay.sampleBoard(x.now)!;
  view.room.board!.tokens[0].space=99;view.room.board!.spaces[0].x=777;
  const again=x.replay.sampleBoard(x.now+.001)!;
  assert.equal(again.room.board!.tokens[0].space,0);assert.equal(again.room.board!.spaces[0].x,0);
  const actual=boardRoom(1,x.logs);x.now+=.1;x.replay.observeRoom(actual);actual.board.spaces[0].x=999;
  assert.equal(x.replay.sampleBoard(x.now)!.room.board!.spaces[0].x,0);
});

test('board highlights cancel on match, phase, reset, or viewer changes',()=>{
  for(const action of [
    (x:any)=>x.replay.observeRoom(boardRoom(1,x.logs,'new-match')),
    (x:any)=>x.replay.observeRoom(room('playing')),
    (x:any)=>x.replay.reset(),
    (x:any)=>x.replay.setViewer('other-user'),
  ]){
    const x=recordedBoard();x.replay.sampleBoard(x.now);action(x);
    assert.equal(x.replay.active,false);assert.equal(x.replay.pending,false);
  }
});

test('deferred highlights expire instead of surprising a player much later',()=>{
  const x=recordedBoard(0);x.now=25;x.replay.observeRoom(boardRoom(1,x.logs));
  assert.equal(x.replay.pending,false);assert.equal(x.replay.sampleBoard(x.now),null);
});

test('an idle board still gives the recorded impact a gentle exit ramp',()=>{
  let now=0;const replay=new ReplayDirector(()=>now);replay.setViewer('viewer');
  for(let i=0;i<40;i++){now=i*.05;replay.observeRoom(boardRoom());}
  now=2;replay.observeRoom(boardRoom(1,[{kind:'knockout',seat:0,coins:10}]));
  now=2.36;
  let slow=false,recovered=false;
  for(let i=0;i<300;i++){
    const view=replay.sampleBoard(now+i/60);
    if(!view&&i>0)break;
    if(view){slow ||=view.speed<.205;if(slow&&view.speed>.9)recovered=true;}
  }
  assert.equal(slow,true);assert.equal(recovered,true);
});

test('distant killing blows frame an explicit impact or victim, rather than the shooter',()=>{
  for(const event of [{kind:'down',by:0,seat:0,target:1},{kind:'down',by:0,seat:1,x:12,y:2,z:8}]){
    let now=0;const replay=new ReplayDirector(()=>now);replay.observeRoom(room('playing'));
    for(let i=0;i<=40;i++)replay.record(i*.05,snap(i*.05),i===40?[event as any]:[]);
    replay.observeRoom(room('results'));replay.record(2.1,snap(2.1),[]);
    const sample=replay.sample(blend,'shooter',.151)!;
    assert.equal(sample.focusSeat,1);
    assert.deepEqual(sample.focusPoint,'x' in event?{x:12,y:2,z:8}:{x:3,y:0,z:2});
  }
});

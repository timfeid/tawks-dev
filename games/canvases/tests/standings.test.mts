import{test}from'node:test';import assert from'node:assert/strict';
import{boardStandings,placeLabel}from'../src/party/board/standings-state.ts';
const members=Array.from({length:8},(_,seat)=>({seat,user_id:`u${seat}`,name:`P${seat}`})) as any;
const board={tokens:[{seat:0,prizes:2,coins:10},{seat:1,prizes:1,coins:99},{seat:2,prizes:2,coins:20},{seat:3,prizes:2,coins:20}],order:[3,2,1,0],turn_seat:2} as any;
test('candy leads, coins break ties, and tied ranks skip the next occupied place',()=>{
 const rows=boardStandings(board,members);assert.deepEqual(rows.map(r=>r.place),[3,4,1,1]);assert.equal(rows[2].current,true);assert.equal(rows[2].turn,2);
});
test('corner positions stay stable when minigame turn order changes',()=>{
 const tokens=members.map((m:any)=>({seat:m.seat,prizes:0,coins:10}));
 const rows=boardStandings({...board,tokens},[...members].reverse());assert.deepEqual(rows.map(r=>r.member.seat),[0,1,2,3,4,5,6,7]);assert.deepEqual(rows.map(r=>r.corner),[0,1,2,3,0,1,2,3]);
});
test('authoritative ranks work with hidden coins without treating unknown money as zero',()=>{
 const rows=boardStandings({...board,tokens:board.tokens.map((t:any)=>({...t,coins:null})),standings:[[2,3],[0],[1]]},members);assert.deepEqual(rows.map(r=>r.place),[3,4,1,1]);
 assert.ok(boardStandings({...board,tokens:board.tokens.map((t:any)=>({...t,coins:null}))},members).every(r=>r.place===null));assert.equal(placeLabel(1),'1st');assert.equal(placeLabel(8),'8th');
});

import{test}from'node:test';import assert from'node:assert/strict';
import{visibleTeams,visibleTeamFor}from'../src/party/team-visibility.ts';
import{PAIR_COLORS,type RoomView}from'../src/party/protocol.ts';
const room=()=>({pairs_on:true,pairs:[{seat:0,team:0},{seat:1,team:0},{seat:2,team:1},{seat:3,team:1}],teams:[{seat:0,team:1},{seat:1,team:0},{seat:2,team:1},{seat:3,team:0}]})as RoomView;
test('board partnerships use pair labels/colors instead of the previous minigame sides',()=>{const teams=visibleTeams(room(),{teams:['ATTACK','RETAKE']},'board');assert.equal(visibleTeamFor(teams,0)?.label,'Orange pair');assert.equal(visibleTeamFor(teams,1)?.id,0);assert.equal(visibleTeamFor(teams,2)?.label,'Cyan pair');});
test('active evaluated player teams override room assignments and board partnerships',()=>{const teams=visibleTeams(room(),{teams:['ATTACK','RETAKE'],scores:[{seat:0,team:0},{seat:1,team:1},{seat:2,team:0},{seat:3,team:1}]},'bout');assert.equal(visibleTeamFor(teams,0)?.label,'Attack team');assert.equal(visibleTeamFor(teams,1)?.label,'Retake team');assert.deepEqual(teams[0].seats,[0,2]);});
test('countdown falls back to current room sides before evaluated scores arrive',()=>{const teams=visibleTeams(room(),null,'bout');assert.equal(visibleTeamFor(teams,0)?.label,'Blue team');assert.equal(visibleTeamFor(teams,1)?.label,'Red team');});
test('free for all and disabled board pairs create no team decorations',()=>{const r=room();r.teams=[];assert.deepEqual(visibleTeams(r,{scores:[{seat:0}]},'bout'),[]);r.pairs_on=false;assert.deepEqual(visibleTeams(r,null,'board'),[]);});
test('sparse team identifiers keep their own id and no viewer is assigned a guessed side',()=>{const r=room();r.teams=[{seat:0,team:1},{seat:1,team:3}];const teams=visibleTeams(r,null,'bout');assert.deepEqual(teams.map(t=>t.id),[1,3]);assert.equal(visibleTeamFor(teams,0)?.id,1);assert.equal(visibleTeamFor(teams,99),null);assert.equal(visibleTeamFor(teams,null),null);});

test('board team cues retain the shared protocol palette',()=>{const r=room();assert.deepEqual(visibleTeams(r,null,'board').map(team=>team.color),PAIR_COLORS.slice(0,2));});

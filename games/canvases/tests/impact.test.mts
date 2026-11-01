import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spikeImpactClock,impactSweep,SPIKE_SWEEP_SECONDS,SPIKE_SLOW_SECONDS} from '../src/party/board/impact.ts';

test('spike animation runs at 20% through impact, then returns to normal',()=>{
  const start=SPIKE_SWEEP_SECONDS;
  assert.ok(Math.abs((spikeImpactClock(start+.8)-spikeImpactClock(start+.2))/.6-.2)<1e-10);
  const after=start+SPIKE_SLOW_SECONDS;
  assert.ok(Math.abs(spikeImpactClock(after+.3)-spikeImpactClock(after+.1)-.2)<1e-10);
  for(const boundary of [start,after])assert.ok(Math.abs(spikeImpactClock(boundary-1e-6)-spikeImpactClock(boundary+1e-6))<3e-6);
});

test('impact camera sweep is smooth, bounded, and finishes before the slow hit',()=>{
  assert.equal(impactSweep(-1),0);
  assert.equal(impactSweep(SPIKE_SWEEP_SECONDS),1);
  assert.equal(impactSweep(9),1);
  assert.ok(impactSweep(.1)>0&&impactSweep(.1)<.5);
  assert.ok(impactSweep(SPIKE_SWEEP_SECONDS-1e-4)>.999);
});

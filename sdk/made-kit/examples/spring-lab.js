// Spring Lab: a block hangs from a spring. Pull it down and let go, change
// the stiffness, the mass and the damping, and watch the energy trade between
// motion and the spring. The example sim the Builder agent reads first.
import { defineSim, slider, toggle, choice, button, plot, rk4 } from '@tawks/made-kit/sim';

const ANCHOR_Y = 1.6;
const GRAVITY = { earth: 9.81, moon: 1.62, none: 0 };

function rest(p) {
  return ANCHOR_Y - p.length - (p.mass * GRAVITY[p.world]) / p.k;
}

export default defineSim({
  meta: {
    name: 'Spring Lab',
    about: 'A block on a spring: stretch it, let go, and watch it bounce.',
    explain:
      'Drag the block down and let go. A stiffer spring or a lighter block bounces faster. Damping drains the energy until the block rests where the spring holds its weight.',
    world: { x: [-3.6, 3.6], y: [-2.2, 2], unit: 'm', grid: 0.5 },
    rate: 240,
  },
  params: {
    k: slider('Stiffness', { min: 2, max: 60, step: 1, value: 20, unit: 'N/m' }),
    mass: slider('Mass', { min: 0.2, max: 4, step: 0.1, value: 1, unit: 'kg' }),
    damping: slider('Damping', { min: 0, max: 2, step: 0.05, value: 0.1, unit: 'kg/s' }),
    length: slider('Spring length', { min: 0.4, max: 1.4, step: 0.05, value: 0.8, unit: 'm' }),
    world: choice('Gravity', { earth: 'Earth', moon: 'Moon', none: 'None' }, 'earth'),
    forces: toggle('Show forces', true),
    pull: button('Pull down 0.5 m'),
  },
  plots: {
    energy: plot('Energy', { y: 'J', seconds: 10, series: { kinetic: ['Motion', 'blue'], spring: ['Spring', 'orange'], total: ['Total', 'ink'] } }),
  },

  setup(p) {
    return { y: rest(p) - 0.5, v: 0, held: false };
  },

  step(s, p, dt) {
    if (s.held) {
      s.v = 0;
      return;
    }
    const g = GRAVITY[p.world];
    const [y, v] = rk4([s.y, s.v], 0, dt, ([y, v]) => {
      const stretch = ANCHOR_Y - p.length - y;
      return [v, (p.k * stretch - p.damping * v) / p.mass - g];
    });
    s.y = y;
    s.v = v;
  },

  press(s, p, key) {
    if (key === 'pull') {
      s.y = rest(p) - 0.5;
      s.v = 0;
    }
  },

  grab(s, p, g) {
    s.held = g.phase !== 'end';
    s.y = Math.min(ANCHOR_Y - 0.2, Math.max(-2, g.y));
    s.v = 0;
  },

  sample(s, p) {
    const stretch = ANCHOR_Y - p.length - s.y;
    const restStretch = ANCHOR_Y - p.length - rest(p);
    const kinetic = 0.5 * p.mass * s.v * s.v;
    // Energy measured from the resting point, so it settles to zero with damping.
    const spring = 0.5 * p.k * (stretch - restStretch) ** 2;
    return { energy: { kinetic, spring, total: kinetic + spring } };
  },

  view(s, p, v) {
    const size = 0.25 + 0.12 * Math.sqrt(p.mass);
    v.rect('ceiling', 0, ANCHOR_Y + 0.08, 1.6, 0.16, { fill: 'surface', color: 'muted' });
    v.spring('spring', 0, ANCHOR_Y, 0, s.y + size / 2, { coils: 12, r: 0.12, color: 'muted', width: 2 });
    v.rect('block', 0, s.y, size, size, { fill: 'blue', color: 'ink', width: 1.5, drag: true, label: `${p.mass.toFixed(1)} kg` });
    v.line('rest', -0.9, rest(p), 0.9, rest(p), { color: 'green', dash: true, label: 'rest' });
    if (p.forces) {
      const stretch = ANCHOR_Y - p.length - s.y;
      v.arrow('spring-force', 0.4, s.y, 0, p.k * stretch, { scale: 0.04, color: 'orange', label: 'spring' });
      v.arrow('weight', -0.4, s.y, 0, -p.mass * GRAVITY[p.world], { scale: 0.04, color: 'purple', label: 'weight' });
    }
    v.readout('Stretch', ANCHOR_Y - p.length - s.y, { unit: 'm', digits: 2 });
    v.readout('Speed', Math.abs(s.v), { unit: 'm/s', digits: 2 });
    v.readout('Period', 2 * Math.PI * Math.sqrt(p.mass / p.k), { unit: 's', digits: 2 });
    if (s.held) v.notice('Let go to release the block');
  },
});

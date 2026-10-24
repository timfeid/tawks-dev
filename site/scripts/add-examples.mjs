// Builds each example game's client into the site under /examples/<name>/,
// so the sandbox can register them at an https address every player can load.
// An absolute base keeps their assets loading whatever the URL's trailing slash.
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const out = join(root, 'site/build/examples');
rmSync(out, { recursive: true, force: true });
for (const name of readdirSync(join(root, 'examples'))) {
  const client = join(root, 'examples', name, 'client');
  if (!existsSync(join(client, 'index.html'))) continue;
  execFileSync('pnpm', ['exec', 'vite', 'build', '--base', `/examples/${name}/`, '--outDir', join(out, name), '--emptyOutDir'], {
    cwd: client,
    stdio: 'inherit',
  });
  console.log(`added /examples/${name}/`);
}

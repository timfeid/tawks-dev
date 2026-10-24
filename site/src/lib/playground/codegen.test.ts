import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { typescriptFor } from './codegen.ts';

const data = JSON.parse(readFileSync(new URL('./generated.json', import.meta.url), 'utf8'));
const builder = new URL('../../../../sdk/bot-js/src/builder.ts', import.meta.url).href;
const dir = mkdtempSync(join(tmpdir(), 'tawks-codegen-'));

// The generated TypeScript, run against the real builders, has to give back
// the very definition it was generated from.
for (const template of data.templates) {
  test(`the TypeScript for ${template.key} rebuilds it exactly`, async () => {
    const code = typescriptFor(template.definition).replace(`from '@tawks/bot'`, `from '${builder}'`);
    const file = join(dir, `${template.key}.ts`);
    writeFileSync(file, code);
    const { definition } = await import(file);
    assert.deepEqual(JSON.parse(JSON.stringify(definition)), template.definition);
  });
}

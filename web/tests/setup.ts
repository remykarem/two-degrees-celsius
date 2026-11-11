// Vitest setup: pure panel and scene functions read the UI catalog from the store. Load the REAL catalog() output (docs/fixtures/catalog.json) so tests need no game.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { setCatalog } from '../src/store/catalog';
import type { Catalog } from '../src/contract';

setCatalog(JSON.parse(readFileSync(resolve(__dirname, '../../docs/fixtures/catalog.json'), 'utf8')) as Catalog);

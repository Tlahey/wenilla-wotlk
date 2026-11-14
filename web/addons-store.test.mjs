import assert from 'node:assert/strict';
import test from 'node:test';
import { installAddonStore } from './addons-store.js';

test('served sources retain raw bytes while non-source assets stay lazy', async () => {
  const previous = { window: globalThis.window, fetch: globalThis.fetch, XMLHttpRequest: globalThis.XMLHttpRequest };
  const lua = new Uint8Array([0xef, 0xbb, 0xbf, ...new TextEncoder().encode('local key = "提示訊息"\n')]);
  const fetched = [];
  let syncReads = 0;
  const sources = new Map([
    ['Probe/Probe.toc', new TextEncoder().encode('## Interface: 30300\nprobe.lua')],
    ['Probe/probe.lua', lua],
  ]);
  globalThis.window = {};
  globalThis.XMLHttpRequest = class {
    open(_method, url) { assert.equal(url, '/addons/Probe/art.tga'); }
    overrideMimeType() {}
    send() { syncReads++; this.status = 200; this.responseText = '\x00\xff'; }
  };
  globalThis.fetch = async (url) => {
    if (url === '/addons/__catalog') return {
      ok: true, json: async () => ({ files: [...sources.keys(), 'Probe/broken.lua', 'Probe/art.tga'] }),
    };
    fetched.push(url);
    const bytes = sources.get(url.slice('/addons/'.length));
    return { ok: !!bytes, status: bytes ? 200 : 404, arrayBuffer: async () => bytes.buffer };
  };
  try {
    await installAddonStore();
    assert.deepEqual(window.__wenilla_addons.folders(), ['Probe']);
    assert.deepEqual(window.__wenilla_addons.read('probe/PROBE.lua'), lua);
    assert.equal(window.__wenilla_addons.read('Probe/broken.lua'), null);
    assert.equal(syncReads, 0);
    assert.ok(!fetched.includes('/addons/Probe/art.tga'));
    assert.deepEqual(window.__wenilla_addons.read('Probe/art.tga'), new Uint8Array([0, 255]));
    window.__wenilla_addons.read('Probe/art.tga');
    assert.equal(syncReads, 1);
  } finally {
    Object.assign(globalThis, previous);
  }
});

test('an imported folder replaces the entire served folder without fetching its sources', async () => {
  const previous = { window: globalThis.window, indexedDB: globalThis.indexedDB, fetch: globalThis.fetch };
  const bytes = new Uint8Array([0xef, 0xbb, 0xbf, 0xff, 0x0a]);
  globalThis.window = { indexedDB: {} };
  globalThis.indexedDB = {
    open() {
      const request = { result: {
        close() {},
        transaction: () => ({ objectStore: () => ({ getAll() {
          const read = { result: [{ path: 'Probe/Probe.toc', bytes: bytes.buffer }] };
          queueMicrotask(() => read.onsuccess());
          return read;
        } }) }),
      } };
      queueMicrotask(() => request.onsuccess());
      return request;
    },
  };
  globalThis.fetch = async (url) => {
    assert.equal(url, '/addons/__catalog');
    return { ok: true, json: async () => ({ files: ['Probe/Probe.toc', 'Probe/old.lua'] }) };
  };
  try {
    await installAddonStore();
    assert.deepEqual(window.__wenilla_addons.read('Probe/Probe.toc'), bytes);
    assert.equal(window.__wenilla_addons.has('Probe/old.lua'), false);
  } finally {
    Object.assign(globalThis, previous);
  }
});

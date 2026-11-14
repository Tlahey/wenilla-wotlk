// addons-store.js — the page's `Interface\AddOns` folder, built before the client starts.
//
// The browser build has no filesystem, so the client reads its AddOns folder through
// `window.__wenilla_addons` (benilla-formats `web_addons`), synchronously. Two sources:
//
//   - the addons the host serves read-only (`wenilla-host --addons`): `GET /addons/__catalog`
//     lists their files, and each file is read on first use from `/addons/<path>`;
//   - the addons the player imported into this browser (`addons.html`), kept in IndexedDB and
//     never uploaded. A local folder replaces a served one of the same name, whole.
//
// Paths are relative to the AddOns root (`MyAddon/MyAddon.toc`), matched without regard to case
// or separator, as the reference's Windows filesystem matches them. An addon imported or removed
// while the game is open counts at the next page load.

export const DB_NAME = 'wenilla-local-addons';
export const DB_VERSION = 1;
export const FILE_STORE = 'files';
export const META_STORE = 'meta';

export function addonKey(path) {
  return String(path).replace(/\\/g, '/').replace(/^\/+/, '').toLowerCase();
}

export function openAddonDb() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(FILE_STORE)) db.createObjectStore(FILE_STORE);
      if (!db.objectStoreNames.contains(META_STORE)) db.createObjectStore(META_STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function localFiles() {
  if (!window.indexedDB) return [];
  const db = await openAddonDb();
  try {
    return await new Promise((resolve, reject) => {
      const request = db.transaction(FILE_STORE, 'readonly').objectStore(FILE_STORE).getAll();
      request.onsuccess = () => resolve(request.result || []);
      request.onerror = () => reject(request.error);
    });
  } finally {
    db.close();
  }
}

async function servedFiles() {
  const response = await fetch('/addons/__catalog', { cache: 'no-store', credentials: 'same-origin' });
  if (!response.ok) return [];
  const body = await response.json();
  return Array.isArray(body.files) ? body.files.filter((f) => typeof f === 'string') : [];
}

// A blocking GET of one served file: the client's reads are synchronous (see benilla-formats
// web.rs `fetch_sync` for the x-user-defined decoding).
function fetchServed(path) {
  const xhr = new XMLHttpRequest();
  xhr.open('GET', '/addons/' + path.split('/').map(encodeURIComponent).join('/'), false);
  xhr.overrideMimeType('text/plain; charset=x-user-defined');
  try {
    xhr.send();
  } catch (_) {
    return null;
  }
  if (xhr.status !== 200) return null;
  const text = xhr.responseText;
  const bytes = new Uint8Array(text.length);
  for (let i = 0; i < text.length; i++) bytes[i] = text.charCodeAt(i) & 0xff;
  return bytes;
}

export async function installAddonStore() {
  const [local, served] = await Promise.all([
    localFiles().catch((e) => { console.warn('addons: local store unavailable:', e); return []; }),
    servedFiles().catch(() => []),
  ]);
  const localFolders = new Set(local.map((f) => addonKey(f.path).split('/')[0]));
  const entries = new Map();
  for (const path of served) {
    const key = addonKey(path);
    if (!localFolders.has(key.split('/')[0])) entries.set(key, { path });
  }
  for (const file of local) {
    if (!file || typeof file.path !== 'string' || !file.bytes) continue;
    entries.set(addonKey(file.path), { path: file.path, bytes: new Uint8Array(file.bytes), local: true });
  }

  const folders = [];
  for (const [key, entry] of entries) {
    const parts = key.split('/');
    if (parts.length === 2 && parts[1] === parts[0] + '.toc') folders.push(entry.path.split(/[\\/]/)[0]);
  }

  window.__wenilla_addons = {
    folders: () => folders.slice(),
    has: (rel) => entries.has(addonKey(rel)),
    read: (rel) => {
      const entry = entries.get(addonKey(rel));
      if (!entry) return null;
      if (!entry.bytes) entry.bytes = fetchServed(entry.path);
      return entry.bytes;
    },
  };
  const localCount = folders.filter((f) => localFolders.has(f.toLowerCase())).length;
  console.log(`addons: ${folders.length - localCount} served, ${localCount} local`);
}

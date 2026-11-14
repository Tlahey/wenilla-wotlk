import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

const source = readFileSync(new URL('./error-clipboard.js', import.meta.url), 'utf8');

function setup(writeText) {
  class Element {
    children = [];
    style = {};
    append(...children) { this.children.push(...children); }
    setAttribute(name, value) { this[name] = value; }
    querySelector(tag) { return this.children.find((child) => child.tag === tag); }
    get lastElementChild() { return this.children.at(-1); }
    showModal() { this.open = true; }
    close() { this.open = false; }
    focus() { this.focused = true; }
    select() { this.selected = true; }
  }
  const body = new Element();
  const window = {};
  runInNewContext(source, {
    window, navigator: { clipboard: { writeText } },
    document: {
      body,
      createElement(tag) { const element = new Element(); element.tag = tag; return element; },
    },
  });
  return { window, body };
}

test('copies the full report locally and reuses the dialog with the new snapshot', async () => {
  const writes = [];
  const { window, body } = setup(async (text) => writes.push(text));
  const report = '#1 x4059\nfull traceback\n\n#2 x1\nFrench: é';
  await window.__wenilla_copy_errors(report, 'frFR');
  const dialog = body.children[0];
  assert.deepEqual(writes, [report]);
  assert.equal(dialog.querySelector('textarea').value, report);
  assert.equal(dialog.querySelector('button').textContent, 'Tout copier');
  assert.match(dialog.querySelector('p').textContent, /copié/);
  dialog.close();
  await window.__wenilla_copy_errors('empty log', 'enUS');
  assert.equal(body.children.length, 1);
  assert.equal(dialog.open, true);
  assert.deepEqual(writes, [report, 'empty log']);
});

test('denied clipboard access leaves selectable text and allows a fresh click to retry', async () => {
  let allowed = false;
  let copied;
  const { window, body } = setup(async (text) => {
    if (!allowed) throw new Error('NotAllowedError');
    copied = text;
  });
  await window.__wenilla_copy_errors('all errors', 'enUS');
  const dialog = body.children[0];
  assert.equal(dialog.querySelector('textarea').selected, true);
  assert.match(dialog.querySelector('p').textContent, /blocked/);
  allowed = true;
  await dialog.querySelector('button').onclick();
  assert.equal(copied, 'all errors');
  assert.match(dialog.querySelector('p').textContent, /Copied/);
});

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const html = readFileSync(new URL('../assets/diff.html', import.meta.url), 'utf8');

test('outer document and loading message inherit host appearance', () => {
  assert.match(html, /html,body,#diff-container\{[^}]*background:var\(--renderer-background\);color:var\(--renderer-foreground\)/);
  assert.match(html, /:root\{[^}]*--renderer-background:__BACKGROUND__/);
  assert.match(html, /color-scheme:__COLOR_SCHEME__/);
  assert.ok(html.indexOf('__INITIAL_APPEARANCE__') < html.indexOf('__PIERRE_BUNDLE__'));
});

test('host appearance updates outer DOM before bundle loads and switches live diff', () => {
  const script = html.match(/<script id="renderer-appearance">([\s\S]*?)<\/script>/)?.[1];
  assert.ok(script, 'outer appearance bridge must exist before Pierre');
  const properties = new Map();
  const style = { setProperty: (name, value) => properties.set(name, value) };
  const timers = [];
  const window = {};
  vm.runInNewContext(script.replace('__INITIAL_APPEARANCE__', ''), {
    window, document: { documentElement: { style } }, setTimeout: (fn) => timers.push(fn),
  });
  const apply = (scheme, background, foreground) => {
    window.setRendererAppearance(scheme, background, foreground);
    assert.equal(properties.get('--renderer-background'), background);
    assert.equal(properties.get('--renderer-foreground'), foreground);
    assert.equal(style.colorScheme, scheme);
  };
  apply('light', 'hsl(220, 23%, 95%)', 'hsl(234, 16%, 35%)');
  timers.splice(0).forEach(fn => fn()); // Still loading: no Pierre bridge.
  const themes = [];
  window.pierreBridge = { setTheme: theme => themes.push(theme) };
  apply('dark', 'hsl(229, 19%, 23%)', 'hsl(227, 70%, 87%)');
  apply('light', 'hsl(220, 23%, 95%)', 'hsl(234, 16%, 35%)');
  timers.splice(0).forEach(fn => fn());
  assert.deepEqual(themes, ['dark', 'light']);
});

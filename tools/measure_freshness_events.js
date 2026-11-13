#!/usr/bin/env node
'use strict';

// Experimental event delivery only: events do not certify graph freshness.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { performance } = require('node:perf_hooks');

const delay = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));

function mutation(root, scenario, number) {
  const source = path.join(root, 'app.ts');
  const text = `export function app(){return ${number};}\n`;
  const actions = {
    body: () => fs.writeFileSync(source, text),
    addition: () => fs.writeFileSync(path.join(root, `added-${number}.ts`), text),
    deletion: () => fs.unlinkSync(source),
    rename: () => fs.renameSync(source, path.join(root, 'renamed.ts')),
    atomic: () => {
      const temporary = path.join(root, 'save.tmp');
      fs.writeFileSync(temporary, text);
      fs.renameSync(temporary, source);
    },
    burst: () => { for (let index = 0; index < 10; index++) fs.writeFileSync(source, text + index); },
    configuration: () => fs.writeFileSync(path.join(root, 'tsconfig.json'), JSON.stringify({ strict: true, number })),
  };
  actions[scenario]();
}

async function observe(scenario, number) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccnavigator-events-'));
  fs.writeFileSync(path.join(root, 'app.ts'), 'export function app(){return 0;}\n');
  let watcher;
  let timer;
  let started;
  try {
    const delivered = new Promise((resolve, reject) => {
      watcher = fs.watch(root, { recursive: true }, (event, filename) => {
        if (started === undefined) return;
        const expected = scenario === 'addition' ? `added-${number}.ts`
          : scenario === 'configuration' ? 'tsconfig.json' : 'app.ts';
        if (String(filename) !== expected) return;
        resolve({ milliseconds: performance.now() - started, event, filename });
      });
      watcher.on('error', reject);
      timer = setTimeout(() => reject(new Error('No filesystem event within 5 seconds')), 5000);
    });
    // Register before the mutation, then measure first delivery, not debounce completion.
    await Promise.race([delay(20), delivered]);
    started = performance.now();
    mutation(root, scenario, number);
    return { scenario, repetition: number, ...(await delivered) };
  } finally {
    clearTimeout(timer);
    watcher?.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
}

async function idle() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccnavigator-events-idle-'));
  let events = 0;
  const watcher = fs.watch(root, { recursive: true }, () => events++);
  const failed = new Promise((resolve, reject) => watcher.on('error', reject));
  try {
    // Drain registration notifications before measuring idle activity.
    await Promise.race([delay(100), failed]);
    events = 0;
    const before = process.cpuUsage();
    const started = performance.now();
    await Promise.race([delay(2000), failed]);
    return { duration_ms: performance.now() - started, cpu_microseconds: process.cpuUsage(before), events };
  } finally {
    watcher.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
}

async function main() {
  const runs = Number(process.argv[2] || 20);
  if (!Number.isInteger(runs) || runs < 1) throw new Error('Usage: node measure_freshness_events.js [positive runs]');
  const records = [];
  for (const scenario of ['body', 'addition', 'deletion', 'rename', 'atomic', 'burst', 'configuration']) {
    for (let number = 1; number <= runs; number++) records.push(await observe(scenario, number));
  }
  const summary = {};
  for (const scenario of new Set(records.map(record => record.scenario))) {
    const values = records.filter(record => record.scenario === scenario).map(record => record.milliseconds).sort((a, b) => a - b);
    summary[scenario] = { count: values.length, p50_ms: values[Math.ceil(values.length * 0.5) - 1], p95_ms: values[Math.ceil(values.length * 0.95) - 1], max_ms: values.at(-1) };
  }
  console.log(JSON.stringify({ schema: 'codecallnavigator.freshness_events.v1', node: process.version, platform: os.platform(), architecture: os.arch(), method: 'First recursive fs.watch event in isolated tiny temporary directory; no graph verification, production integration, dropped-event recovery or large-tree coverage.', records, summary, idle: await idle() }, null, 2));
}

main().catch(error => { console.error(error.message); process.exitCode = 1; });

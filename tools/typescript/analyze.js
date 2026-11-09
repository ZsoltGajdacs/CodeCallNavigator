#!/usr/bin/env node
'use strict';
const fs = require('node:fs');
const {prepare,failDiagnostics} = require('./config.js');
const {extract} = require('./extract.js');
function main() {
  const [mode,config,output]=process.argv.slice(2);
  if (!['prepare','analyze'].includes(mode)) throw new Error('Expected prepare or analyze mode');
  if (![config,output].every(Boolean)) throw new Error('Usage: node analyze.js prepare|analyze TSCONFIG OUTPUT_JSON');
  const context=prepare(config);
  if (mode==='analyze') for (const {program} of context.programs) failDiagnostics(context.ts,context.ts.getPreEmitDiagnostics(program));
  const result=mode==='prepare' ? context.metadata : extract(context);
  fs.writeFileSync(output,JSON.stringify(result));
}
try { main(); } catch (error) { console.error(error.message); process.exitCode=1; }

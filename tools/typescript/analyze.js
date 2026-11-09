#!/usr/bin/env node
'use strict';
const {prepare} = require('./config.js');
const {worker,analyzed,writeOutput}=require('./worker.js');
function main() {
  const [mode,config,output]=process.argv.slice(2);
  if (!['prepare','analyze','worker'].includes(mode)) throw new Error('Expected prepare, analyze or worker mode');
  if (![config,output].every(Boolean)) throw new Error('Usage: node analyze.js prepare|analyze TSCONFIG OUTPUT_JSON');
  const context=prepare(config);
  if (mode==='worker') return worker(context,output);
  writeOutput(output,mode==='prepare' ? context.metadata : analyzed(context));
}
try { main(); } catch (error) { console.error(error.message); process.exitCode=1; }

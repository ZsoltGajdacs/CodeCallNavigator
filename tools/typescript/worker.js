'use strict';
const fs=require('node:fs');
const readline=require('node:readline');
const {failDiagnostics}=require('./compiler.js');
const {extract}=require('./extract.js');
function writeOutput(output,result) {
  const temporary=`${output}.tmp`;
  fs.writeFileSync(temporary,JSON.stringify(result));
  fs.renameSync(temporary,output);
}
function analyzed(context) {
  context.observations.verify();
  for (const {program} of context.programs) failDiagnostics(context.ts,context.ts.getPreEmitDiagnostics(program));
  const result=extract(context);
  context.observations.verify();
  return result;
}
function worker(context,output) {
  context.observations.verify();
  writeOutput(output,context.metadata);
  const commands=readline.createInterface({input:process.stdin,terminal:false});
  let requested=false;
  commands.on('line',line=>{
    try {
      if (requested) throw new Error('TypeScript worker accepts one analysis request');
      requested=true;
      const request=JSON.parse(line);
      if (request.command!=='analyze' || typeof request.output!=='string') throw new Error('Invalid TypeScript worker request');
      writeOutput(request.output,analyzed(context));
    } catch (error) { console.error(error.message); process.exitCode=1; }
    commands.close();
    process.stdin.destroy();
  });
}
module.exports={worker,analyzed,writeOutput};

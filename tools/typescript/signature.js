'use strict';
const path=require('node:path');
const {canonical,sourcePath}=require('./paths.js');

function templateToken(ts,scanner,kind,braces) {
  if (kind===ts.SyntaxKind.TemplateHead) {
    braces.push(0);
    return kind;
  }
  if (!braces.length) return kind;
  const last=braces.length-1;
  if (kind===ts.SyntaxKind.OpenBraceToken) braces[last]++;
  if (kind===ts.SyntaxKind.CloseBraceToken) return closeTemplate(ts,scanner,braces);
  return kind;
}

function closeTemplate(ts,scanner,braces) {
  const last=braces.length-1;
  if (braces[last]>0) {
    braces[last]--;
    return ts.SyntaxKind.CloseBraceToken;
  }
  // Template text is opaque, just like a quoted literal type. Resume
  // scanning types only inside its next ${...} interpolation.
  const kind=scanner.reScanTemplateToken(false);
  if (kind===ts.SyntaxKind.TemplateTail) braces.pop();
  return kind;
}

function isAbsoluteImport(ts,scanner,previous,beforePrevious) {
  if (scanner.getToken()!==ts.SyntaxKind.StringLiteral || scanner.isUnterminated()) return false;
  if (previous!==ts.SyntaxKind.OpenParenToken || beforePrevious!==ts.SyntaxKind.ImportKeyword) return false;
  return path.isAbsolute(scanner.getTokenValue());
}

function relativeSpecifier(file,specifier) {
  // Compiler module IDs may omit the source extension. Canonicalize only the
  // containing directory: an unrelated existing directory at that stem must
  // not redirect the module ID away from its actual source file.
  const physicalSpecifier=path.join(canonical(path.dirname(specifier)),path.basename(specifier));
  const relative=path.relative(path.dirname(file),physicalSpecifier).split(path.sep).join('/');
  return relative.startsWith('./') || relative.startsWith('../') ? relative : `./${relative}`;
}

function relativeImportTypes(ts,signature,file) {
  if (!signature.includes('import')) return signature;
  const scanner=ts.createScanner(ts.ScriptTarget.Latest,true,ts.LanguageVariant.Standard,signature);
  const braces=[], replacements=[];
  let previous, beforePrevious;
  for (let kind=scanner.scan();kind!==ts.SyntaxKind.EndOfFileToken;kind=scanner.scan()) {
    kind=templateToken(ts,scanner,kind,braces);
    if (isAbsoluteImport(ts,scanner,previous,beforePrevious)) {
      replacements.push({start:scanner.getTokenPos(),end:scanner.getTextPos(),
        value:JSON.stringify(relativeSpecifier(file,scanner.getTokenValue()))});
    }
    beforePrevious=previous;
    previous=kind;
  }
  // Token spans retain the compiler's formatting and truncation. Only complete
  // import-type module literals change; authored string literal types do not.
  for (const {start,end,value} of replacements.reverse()) {
    signature=signature.slice(0,start)+value+signature.slice(end);
  }
  return signature;
}

function signatureText(ts,checker,signature,declaration) {
  return relativeImportTypes(ts,checker.signatureToString(signature,declaration),sourcePath(declaration.getSourceFile()));
}
module.exports={relativeImportTypes,signatureText};

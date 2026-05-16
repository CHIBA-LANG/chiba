import fs from "node:fs";
import process from "node:process";

const FIXTURE = "level-1b/tests/source/doc_surface.chiba";
const LEXER_SPEC = "src/frontend/chiba-level1.chibalex";
const LEXER = "src/frontend/chiba_level1_lexer.chiba";
const LEXER_SHOW = "src/frontend/chiba_level1_lexer_show.chiba";
const PARSER_SPEC = "src/frontend/chiba-level1.chibacc";
const PARSER = "src/frontend/chiba_level1_parser.chiba";
const PARSER_HELPERS = "src/frontend/parserspec_helpers.chiba";
const LEVEL1B_ROOT = "level-1b";

function fail(message) {
  console.error(`[FAIL] ${message}`);
  process.exit(1);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function walk(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const path = `${dir}/${entry.name}`;
    if (entry.isDirectory()) out.push(...walk(path));
    else out.push(path);
  }
  return out;
}

function requireIncludes(name, source, needles) {
  for (const needle of needles) {
    if (!source.includes(needle)) fail(`${name} missing ${needle}`);
  }
}

const fixture = read(FIXTURE);
const lexerSpec = read(LEXER_SPEC);
const lexer = read(LEXER);
const lexerShow = read(LEXER_SHOW);
const parserSpec = read(PARSER_SPEC);
const parser = read(PARSER);
const parserHelpers = read(PARSER_HELPERS);

requireIncludes("source fixture", fixture, [
  "/// level-1b source surface fixture.",
  "#[doc(path=\"level-1b/tests/source/doc_surface.chiba\")]",
  "namespace tests.source.doc_surface",
  "/// Public API doc comment",
]);

if (/\/\*/.test(fixture) || /\*\//.test(fixture)) {
  fail("source fixture must not use block comments");
}

for (const file of walk(LEVEL1B_ROOT)) {
  if (!/\.(chiba|chibalex|chibacc|md)$/.test(file)) continue;
  const source = read(file);
  if (/\blevel1b\.(?=[A-Za-z_])/.test(source)) {
    fail(`${file} must not use level1b-prefixed namespace names`);
  }
}

requireIncludes("lexer spec", lexerSpec, [
  "@doccom",
  "{ DocComment(s) }",
  "DocComment(Str)",
  "DocComment(_) => 1",
  "DocComment(text) => strhasnewline(text)",
]);

requireIncludes("generated lexer", lexer, [
  "DocComment(Str)",
  "DocComment(s)",
  "DocComment(_) => 1",
  "DocComment(text) => strhasnewline(text)",
  "type TokenSpan { token: Token  span: Span",
]);

requireIncludes("lexer show", lexerShow, [
  "DocComment(s)",
  "\"DocComment(\"",
  "span_to_str",
  "tokenspan_to_string",
]);

requireIncludes("parser spec", parserSpec, [
  "_:DocComment => 0",
  "file_attr_list ns:namespace_decl",
  "AttrArgIdentNamedString",
]);

requireIncludes("generated parser", parser, [
  "match_token_DocComment",
  "DocComment(v) => MatchOK",
]);

requireIncludes("parser helpers", parserHelpers, [
  "DocComment(text) => 1",
  "LineComment(text) => 1",
  "Newline(text) => 1",
]);

console.log("[PASS] level-1b C00 source surface");

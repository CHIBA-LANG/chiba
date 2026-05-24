import fs from "node:fs";
import process from "node:process";

const SPEC_ROOT = "/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec";
const ATTR = `${SPEC_ROOT}/01-core-language/attributes.md`;
const LEX = `${SPEC_ROOT}/chibalex.md`;
const ACC = `${SPEC_ROOT}/chibacc.md`;
const FIXTURE = "level-1b/supports/attributes/complex_attribute.chiba";
const TODO = "TODO.md";

function fail(message) {
  console.error("[FAIL] level-1b attribute spec gate");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

const attr = read(ATTR);
const lex = read(LEX);
const acc = read(ACC);
const fixture = read(FIXTURE);
const todo = read(TODO);

for (const needle of [
  "data AttrArg",
  "AttrNamed(Ident, AttrArg)",
  "AttrCall(Ident, Vec[AttrArg])",
  "AttrList(Vec[AttrArg])",
  "AttrObject(Vec[(Ident, AttrArg)])",
  "#[attribute(all(someident, a=b, c=[1,2,3,4]))]",
  "someident = true",
]) {
  if (!attr.includes(needle)) fail(`attributes.md missing ${needle}`);
}

if (!lex.includes("lexer 不解析 attribute 内部嵌套")) {
  fail("chibalex spec must keep attribute nesting in parser, not lexer");
}
for (const needle of ["rule attr_arg", "AttrNamed", "AttrCall", "AttrList", "AttrObject"]) {
  if (!acc.includes(needle)) fail(`chibacc spec missing ${needle}`);
}
if (!fixture.includes("#[attribute(all(someident, a=b, c=[1,2,3,4], meta={owner=\"compiler\", stable=true}))]")) {
  fail("complex attribute fixture missing nested/named/list/object coverage");
}
if (!todo.includes("attribute spec/golden：spec 已定义 AttrArg AST 与 nested/named/list/object grammar")) {
  fail("TODO.md must mark completed attribute spec/golden slice");
}

pass("attribute grammar spec and fixture coverage");

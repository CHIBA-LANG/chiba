import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const PRELUDE_ROOT = "level-1b/prelude";
const POLICY = "level-1b/compiler/source/import_policy.chiba";
const SMOKE_ROOT = "level-1b/supports/pre-c03-smokes";
const REQUIRED_PRELUDE = [
  "use level1b.std.option.*",
  "use level1b.std.result.*",
  "use level1b.std.array.*",
  "use level1b.std.slice.*",
  "use level1b.std.string.*",
  "use level1b.std.vec.*",
  "use level1b.std.map.*",
  "use level1b.std.range.*",
  "def print",
  "def println",
  "def panic",
  "def assert",
];

function fail(message) {
  console.error("[FAIL] level-1b C03 prelude contract");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function run(command, args) {
  return spawnSync(command, args, { encoding: "utf8" });
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function listChiba(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listChiba(file));
    else if (entry.isFile() && entry.name.endsWith(".chiba")) out.push(file);
  }
  return out.sort();
}

function previousDocBlock(lines, index) {
  const docs = [];
  let cursor = index - 1;
  while (cursor >= 0 && lines[cursor].trim() === "") cursor -= 1;
  while (cursor >= 0 && lines[cursor].trimStart().startsWith("///")) {
    docs.push(lines[cursor].trimStart());
    cursor -= 1;
  }
  return docs.reverse().join("\n");
}

function isPublicItem(line) {
  return /^(namespace|type|data|def)\b/.test(line.trimStart());
}

function stripLineComments(source) {
  return source
    .split(/\n/)
    .filter((line) => !line.trimStart().startsWith("///") && !line.trimStart().startsWith("//"))
    .join("\n");
}

function checkDocs(file, source) {
  const rel = file;
  const lines = source.split(/\n/);
  const errors = [];
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${rel}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function main() {
  const preludeFiles = listChiba(PRELUDE_ROOT);
  if (preludeFiles.length === 0) fail("prelude has no Chiba source");

  const prelude = preludeFiles.map(read).join("\n");
  const policy = read(POLICY);
  const missing = REQUIRED_PRELUDE.filter((needle) => !prelude.includes(needle));
  if (missing.length !== 0) fail(`missing prelude exports:\n${missing.join("\n")}`);

  const preludeCode = stripLineComments(prelude);
  if (preludeCode.includes("#![Metal]") || /\bmetalstd\b|extern\s+"metal"|extern\s+"wasi"|Ptr\s*\[|UnsafeRef\s*\[/.test(preludeCode)) {
    fail("prelude must not depend on metalstd or expose Metal capability");
  }
  if (/\bdef\s+(map|filter|fold)\s*\[/.test(preludeCode)) {
    fail("prelude must not define naked map/filter/fold; use receiver methods");
  }

  const policyNeedles = [
    "data PreludeImportPolicy",
    "PreludeImportDefault",
    "PreludeImportDisabled",
    "PreludeImportMetal",
    "def prelude_policy_for_header",
    "def PreludeImportPolicy.should_inject",
    "\"level1b.prelude\"",
  ];
  for (const needle of policyNeedles) {
    if (!policy.includes(needle)) fail(`missing import policy text: ${needle}`);
  }

  const docErrors = [
    ...preludeFiles.flatMap((file) => checkDocs(file, read(file))),
    ...checkDocs(POLICY, policy),
  ];
  if (docErrors.length !== 0) fail(docErrors.join("\n"));
  pass("prelude source contract");

  const defaultSmoke = read(path.join(SMOKE_ROOT, "default_prelude.chiba"));
  const noPrelude = read(path.join(SMOKE_ROOT, "no_prelude_invalid.chiba"));
  const metal = read(path.join(SMOKE_ROOT, "metal_no_prelude_invalid.chiba"));
  if (defaultSmoke.includes("use level1b.prelude")) fail("default prelude smoke must rely on implicit import");
  if (!noPrelude.includes("#![no_prelude_import]")) fail("no_prelude invalid smoke missing attribute");
  if (!metal.includes("#![Metal]")) fail("Metal invalid smoke missing #![Metal]");
  pass("prelude smoke source shape");

  for (const file of listChiba(SMOKE_ROOT)) {
    const parsed = run("./target/debug/level1c.o", ["parse", file]);
    if (parsed.status !== 0 || !parsed.stdout.startsWith("OK(")) {
      fail(`C03 smoke does not parse: ${file}\n${parsed.stdout || parsed.stderr}`);
    }
  }
  pass("prelude smoke parse");
}

main();

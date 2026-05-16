import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/std";
const REQUIRED_FILES = [
  "array.chiba",
  "diagnostic.chiba",
  "io.chiba",
  "list.chiba",
  "map.chiba",
  "option.chiba",
  "process.chiba",
  "range.chiba",
  "result.chiba",
  "slice.chiba",
  "string.chiba",
  "vec.chiba",
];

const REQUIRED_SYMBOLS = [
  "data Option[T]",
  "data Result[T, E]",
  "data List[T]",
  "def Array[T].len",
  "def Slice[T].len",
  "String == Array[u8]",
  "str == Slice[u8]",
  "def String.char_at",
  "def str.char_at",
  "type Vec[T]",
  "def Vec[T].freeze",
  "type Map[K, V]",
  "type StrMap[V]",
  "type Range[T]",
  "a..b",
  "type DiagnosticBuilder",
  "def File.read",
  "def Process.argv",
];

function fail(message) {
  console.error("[FAIL] level-1b C02 std contract");
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
  while (cursor >= 0 && lines[cursor].trimStart().startsWith("#[")) {
    cursor -= 1;
    while (cursor >= 0 && lines[cursor].trim() === "") cursor -= 1;
  }
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

function checkSource(file, source) {
  const rel = path.relative(ROOT, file);
  const lines = source.split(/\n/);
  const code = stripLineComments(source);
  const errors = [];

  if (source.includes("#![Metal]")) errors.push(`${rel}: std source must not be Metal`);
  if (/\/\*/.test(source) || /\*\//.test(source)) errors.push(`${rel}: block comments are not allowed`);
  if (/\b(metalstd|Ptr\s*\[|UnsafeRef\s*\[|extern\s+"metal"|extern\s+"std"|extern\s+"wasi"|unsafe\s*\{)/.test(code)) {
    errors.push(`${rel}: std public source leaks Metal or ABI capability`);
  }
  if (/\b(heap_alloc|load(?:8|16|32|64)|store(?:8|16|32|64))\s*\(/.test(code)) {
    errors.push(`${rel}: std source uses raw memory operation`);
  }

  for (let i = 0; i < lines.length; i += 1) {
    if (!isPublicItem(lines[i])) continue;
    if (previousDocBlock(lines, i).length === 0) {
      errors.push(`${rel}:${i + 1}: public item is missing /// doc comment`);
    }
  }

  return errors;
}

function main() {
  const files = listChiba(ROOT);
  const seen = new Set(files.map((file) => path.basename(file)));
  const missing = REQUIRED_FILES.filter((file) => !seen.has(file));
  if (missing.length !== 0) fail(`missing std files:\n${missing.join("\n")}`);

  const joined = files.map(read).join("\n");
  const missingSymbols = REQUIRED_SYMBOLS.filter((symbol) => !joined.includes(symbol));
  if (missingSymbols.length !== 0) fail(`missing std symbols:\n${missingSymbols.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("std source contract");

  const surface = run("vp", ["run", "level1b:std-surface"]);
  if (surface.status !== 0) fail(surface.stdout || surface.stderr);
  pass("legacy std smoke matrix");
}

main();

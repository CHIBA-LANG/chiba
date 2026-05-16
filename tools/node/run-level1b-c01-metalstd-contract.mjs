import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const ROOT = "level-1b/metalstd";
const REQUIRED_FILES = [
  "abi_scratch.chiba",
  "atomic.chiba",
  "env_import.chiba",
  "pointer.chiba",
  "trap.chiba",
  "wasi_preview1.chiba",
  "wasm_gc.chiba",
];

function fail(message) {
  console.error("[FAIL] level-1b C01 metalstd contract");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
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

function stripDocAndLineComments(source) {
  return source
    .split(/\n/)
    .filter((line) => !line.trimStart().startsWith("///") && !line.trimStart().startsWith("//"))
    .join("\n");
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
  const trimmed = line.trimStart();
  return /^(namespace|type|data|def)\b/.test(trimmed);
}

function isUnsafeSurface(line) {
  return /\b(Ptr|UnsafeRef)\s*\[/.test(line) || /\bextern\s+"(metal|wasi|env|c|C)"/.test(line);
}

function checkRequiredFiles(files) {
  const seen = new Set(files.map((file) => path.basename(file)));
  const missing = REQUIRED_FILES.filter((file) => !seen.has(file));
  if (missing.length !== 0) fail(`missing metalstd files:\n${missing.join("\n")}`);
  pass("required metalstd files exist");
}

function checkSource(file, source) {
  const errors = [];
  const rel = path.relative(ROOT, file);
  const code = stripDocAndLineComments(source);
  const lines = source.split(/\n/);

  if (!source.includes("#![Metal]")) errors.push(`${rel}: missing #![Metal]`);
  if (/\/\*/.test(source) || /\*\//.test(source)) errors.push(`${rel}: block comments are not allowed`);

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (!isPublicItem(line)) continue;
    const docs = previousDocBlock(lines, i);
    if (docs.length === 0) errors.push(`${rel}:${i + 1}: public item is missing /// doc comment`);
    if (isUnsafeSurface(line) && !/Safety:/m.test(docs)) {
      errors.push(`${rel}:${i + 1}: unsafe/ABI public item is missing /// Safety section`);
    }
  }

  const forbiddenHighLevel = [
    /\bOption\s*\[/,
    /\bList\s*\[/,
    /\bMap\s*\[/,
    /\bVec\s*\[/,
    /\bString\b/,
    /\bStr\b/,
    /\bRegex\b/,
    /\bParser\b/,
    /\bLexer\b/,
    /\bprintln\s*\(/,
    /\bprint\s*\(/,
  ];
  for (const pattern of forbiddenHighLevel) {
    if (pattern.test(code)) errors.push(`${rel}: high-level API leaked into metalstd: ${pattern}`);
  }

  const pointerLikeI64 = /\b(ptr|pointer|addr|address|raw|handle|fd)\w*\s*:\s*i64\b/i;
  const i64PointerReturn = /def\s+\w*(ptr|pointer|addr|address|raw|handle|fd)\w*\s*\([^)]*\)\s*:\s*i64\b/i;
  if (pointerLikeI64.test(code)) errors.push(`${rel}: pointer/resource parameter uses i64`);
  if (i64PointerReturn.test(code)) errors.push(`${rel}: pointer/resource return uses i64`);

  if (/\bextern\s+"metal"/.test(code)) errors.push(`${rel}: metal intrinsics must not be modeled as extern \"metal\" ABI`);
  if (/\bextern\s+"std"/.test(code)) errors.push(`${rel}: std is not an extern ABI`);
  if (/\bheap_alloc\s*\(/.test(code)) errors.push(`${rel}: heap_alloc cannot be the ordinary heap`);
  if (/\blinear_(alloc|heap)\b/.test(code)) errors.push(`${rel}: linear memory allocator cannot be default heap`);

  return errors;
}

function main() {
  const files = listChiba(ROOT);
  checkRequiredFiles(files);
  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("metalstd source contract");
}

main();

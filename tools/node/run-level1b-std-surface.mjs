import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const DOC = "level-1b/supports/std-surface.md";
const SMOKE_ROOT = "level-1b/supports/pre-c01-smokes";
const WAT_DIR = ".scratch/level-1b/pre-c01-wat";
const REQUIRED = [
  "String == Array[u8]",
  "str == Slice[u8]",
  "String[index]",
  ".char_at(n)",
  "#![Metal]",
  "Ref[T]",
  "UnsafeRef[T]",
  "Ptr[T]",
  "Atomic[T]",
  "Pre-C01 Smoke Matrix",
  "pre-c01-smokes",
];

function fail(message) {
  console.error("[FAIL] level-1b std surface");
  console.error(message);
  process.exit(1);
}

const doc = fs.readFileSync(DOC, "utf8");
for (const needle of REQUIRED) {
  if (!doc.includes(needle)) fail(`missing required surface text: ${needle}`);
}

function run(command, args) {
  return spawnSync(command, args, { encoding: "utf8" });
}

function listChiba(dir) {
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listChiba(file));
    else if (entry.isFile() && entry.name.endsWith(".chiba")) out.push(file);
  }
  return out.sort();
}

const rawPointerPatterns = [
  /\bload(?:8|16|32|64)\s*\(/,
  /\bstore(?:8|16|32|64)\s*\(/,
  /\bheap_alloc\s*\(/,
  /\bas\s+Ptr\[/,
];

const errors = [];
for (const file of listChiba("level-1b/src")) {
  const source = fs.readFileSync(file, "utf8");
  const metal = source.includes("#![Metal]");
  if (metal) continue;
  for (const pattern of rawPointerPatterns) {
    if (pattern.test(source)) errors.push(`${file}: non-Metal source uses ${pattern}`);
  }
}

if (errors.length !== 0) fail(errors.join("\n"));

const smokeFiles = [
  "string_slice.chiba",
  "collections.chiba",
  "io_process.chiba",
  "refs_atomic_valid.chiba",
  "refs_atomic_invalid.chiba",
];

for (const file of smokeFiles) {
  const full = path.join(SMOKE_ROOT, file);
  if (!fs.existsSync(full)) fail(`missing Pre-C01 smoke source: ${full}`);
  const parsed = run("./target/debug/level1c.o", ["parse", full]);
  if (parsed.status !== 0 || !parsed.stdout.startsWith("OK(")) {
    fail(`Pre-C01 smoke does not parse: ${full}\n${parsed.stdout || parsed.stderr}`);
  }
}

fs.mkdirSync(WAT_DIR, { recursive: true });
const stringSmoke = path.join(SMOKE_ROOT, "string_slice.chiba");
const wat = run("./target/debug/level1c.o", ["wat", stringSmoke]);
if (wat.status !== 0 || !wat.stdout.includes("(module")) {
  fail(`Pre-C01 string/slice WAT emit failed\n${wat.stdout || wat.stderr}`);
}
fs.writeFileSync(path.join(WAT_DIR, "string_slice.wat"), wat.stdout);
if (!wat.stdout.includes("(type $array_u8 (array i8))")) fail("String backing Array[u8] layout missing from Pre-C01 WAT");
if (!wat.stdout.includes("(type $slice_u8 (struct (field (ref $array_u8)) (field i32) (field i32)))")) fail("str Slice[u8] layout missing from Pre-C01 WAT");
if (!wat.stdout.includes("array.new_fixed $array_u8 21")) fail("String literal does not lower real Array[u8] payload in Pre-C01 WAT");
if (!wat.stdout.includes("typed string index pending")) fail("typed String index/slice should remain explicit until typed WAT params land");

const validRefs = fs.readFileSync(path.join(SMOKE_ROOT, "refs_atomic_valid.chiba"), "utf8");
const invalidRefs = fs.readFileSync(path.join(SMOKE_ROOT, "refs_atomic_invalid.chiba"), "utf8");
if (!validRefs.includes("#[world_local]")) fail("valid Ref smoke must mark top-level Ref with #[world_local]");
if (!invalidRefs.includes("def bad_global_ref: Ref[i64]")) fail("invalid Ref smoke must include unmarked top-level Ref fixture");
if (!invalidRefs.includes("Atomic[String]")) fail("invalid Atomic smoke must include unsupported Atomic[String] fixture");

console.log("[PASS] level-1b std surface");

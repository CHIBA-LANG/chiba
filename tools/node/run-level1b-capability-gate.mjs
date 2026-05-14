import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOTS = ["level-1b/src", "level-1b/supports/pre-c12-smokes"];
const FIXTURE_ROOT = "level-1b/supports/pre-c12-smokes";

function fail(message) {
  console.error("[FAIL] level-1b capability gate");
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
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listChiba(file));
    else if (entry.isFile() && entry.name.endsWith(".chiba")) out.push(file);
  }
  return out.sort();
}

function stripUnsafeBlocks(source) {
  let out = "";
  let i = 0;
  while (i < source.length) {
    if (source.startsWith("unsafe", i) && !/[A-Za-z0-9_]/.test(source[i - 1] || "")) {
      let j = i + "unsafe".length;
      while (/\s/.test(source[j] || "")) j += 1;
      if (source[j] === "{") {
        let depth = 0;
        while (j < source.length) {
          if (source[j] === "{") depth += 1;
          else if (source[j] === "}") {
            depth -= 1;
            if (depth === 0) {
              j += 1;
              break;
            }
          }
          j += 1;
        }
        out += " unsafe { } ";
        i = j;
        continue;
      }
    }
    out += source[i];
    i += 1;
  }
  return out;
}

function isMetal(source) {
  return source.includes("#![Metal]");
}

const nonMetalRawPointerPatterns = [
  /\bload(?:8|16|32|64)\s*\(/,
  /\bstore(?:8|16|32|64)\s*\(/,
  /\bheap_alloc\s*\(/,
  /\bas\s+Ptr\[/,
  /\b[A-Za-z_]\w*\s*:\s*i64\s*(?:,|\)).*\b(?:ptr|pointer|addr|address|raw)\b/i,
];

function checkNonMetalRawPointer(file, source) {
  if (isMetal(source)) return [];
  const exposed = stripUnsafeBlocks(source);
  const errors = [];
  for (const pattern of nonMetalRawPointerPatterns) {
    if (pattern.test(exposed)) errors.push(`${file}: non-Metal source uses opaque pointer pattern ${pattern}`);
  }
  return errors;
}

function checkUnsafeCapability(file, source) {
  if (isMetal(source)) return [];
  const exposed = stripUnsafeBlocks(source);
  const errors = [];
  if (/\bUnsafeRef\s*\[/.test(exposed) || /\bUnsafeRef\s*\./.test(exposed)) {
    errors.push(`${file}: UnsafeRef used outside unsafe block`);
  }
  if (/\bPtr\s*\[/.test(exposed) || /\bPtr\s*\./.test(exposed) || /\bas\s+Ptr\s*\[/.test(exposed)) {
    errors.push(`${file}: Ptr used outside unsafe block`);
  }
  return errors;
}

function checkMetalRawI64PointerApi(file, source) {
  if (!isMetal(source)) return [];
  const errors = [];
  for (const match of source.matchAll(/def\s+([A-Za-z_]\w*)\s*\(([^)]*)\)\s*:\s*([A-Za-z_]\w*(?:\[[^\]]+\])?)/g)) {
    const [, name, params, ret] = match;
    const pointerNamedI64 = [...params.matchAll(/\b(ptr|pointer|addr|address|raw|p)\s*:\s*i64\b/gi)];
    if (pointerNamedI64.length !== 0) {
      errors.push(`${file}: Metal function ${name} exposes pointer-like i64 parameter; use Ptr[T]`);
    }
    if (/^(alloc|heap_alloc|ptr_|raw_|.*_ptr|.*pointer.*)$/i.test(name) && ret === "i64") {
      errors.push(`${file}: Metal function ${name} returns pointer-like i64; use Ptr[T]`);
    }
  }
  return errors;
}

function checkSourceGates() {
  const errors = [];
  for (const root of ROOTS) {
    for (const file of listChiba(root)) {
      const source = read(file);
      errors.push(...checkNonMetalRawPointer(file, source));
      errors.push(...checkUnsafeCapability(file, source));
      errors.push(...checkMetalRawI64PointerApi(file, source));
    }
  }

  const expectedInvalid = [
    "invalid_ptr_without_unsafe.chiba",
    "invalid_unsaferef_without_unsafe.chiba",
    "invalid_metal_raw_i64_pointer.chiba",
  ];
  for (const name of expectedInvalid) {
    const hit = errors.some((err) => err.includes(name));
    if (!hit) errors.push(`${path.join(FIXTURE_ROOT, name)}: invalid fixture was not rejected`);
  }

  const unexpectedValid = errors.filter((err) => err.includes("valid_capability.chiba") || err.includes("valid_metal_typed_ptr.chiba"));
  if (unexpectedValid.length !== 0) fail(unexpectedValid.join("\n"));

  if (errors.filter((err) => !err.includes("invalid_")).length !== 0) {
    fail(errors.join("\n"));
  }
  pass("level-1b source capability scan");
}

function checkCompilerFixtures() {
  const valid = ["valid_capability.chiba", "valid_metal_typed_ptr.chiba"];
  const invalid = [
    ["invalid_ptr_without_unsafe.chiba", "Ptr requires unsafe block"],
    ["invalid_unsaferef_without_unsafe.chiba", "UnsafeRef requires unsafe block"],
    ["invalid_ref_without_world_local.chiba", "top-level Ref requires #[world_local]"],
    ["invalid_metal_raw_i64_pointer.chiba", "Metal pointer API must use Ptr[T]"],
  ];

  for (const file of [...valid, ...invalid.map(([file]) => file)]) {
    const parsed = run("./target/debug/level1c.o", ["parse", path.join(FIXTURE_ROOT, file)]);
    if (parsed.status !== 0 || !parsed.stdout.startsWith("OK(")) {
      fail(`Pre-C12 fixture does not parse: ${file}\n${parsed.stdout || parsed.stderr}`);
    }
  }

  for (const file of valid) {
    const checked = run("./target/debug/level1c.o", ["check", path.join(FIXTURE_ROOT, file)]);
    if (checked.status !== 0 || !checked.stdout.includes("check ok")) {
      fail(`Pre-C12 valid fixture rejected: ${file}\n${checked.stdout || checked.stderr}`);
    }
  }

  for (const [file, expected] of invalid) {
    const checked = run("./target/debug/level1c.o", ["check", path.join(FIXTURE_ROOT, file)]);
    if (checked.status !== 0 || !checked.stderr.includes(expected)) {
      fail(`Pre-C12 invalid fixture did not report ${expected}: ${file}\n${checked.stdout || checked.stderr}`);
    }
  }
  pass("level-1b compiler capability fixtures");
}

checkSourceGates();
checkCompilerFixtures();

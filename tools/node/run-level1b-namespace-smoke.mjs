import fs from "node:fs";
import crypto from "node:crypto";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { runWatPath } from "./run-wat.mjs";

const PROJECT = "level-1b/supports/namespace-project";
const ENTRY = "use_both.chiba";
const ARTIFACT_DIR = ".scratch/level-1b/namespace";
const SUMMARY = path.join(ARTIFACT_DIR, "project-summary.txt");
const EXPECTED_SUMMARY_SHA256 = "48fceb639c6fc67fd6a63f70b8e255d304a9e90bc75f19503f2fa83214fa9467";
const LEGACY_REFERENCE_COMPILER = "./target/debug/level1c.o";

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

function sourceFiles(dir) {
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) => {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) return sourceFiles(full);
      if (entry.isFile() && entry.name.endsWith(".chiba")) return [full];
      return [];
    })
    .sort();
}

function projectSummary() {
  const lines = [];
  for (const file of sourceFiles(path.join(PROJECT, "src"))) {
    const rel = path.relative(PROJECT, file);
    const source = fs.readFileSync(file, "utf8");
    const namespace = source.match(/^\s*namespace\s+([A-Za-z_][A-Za-z0-9_.]*)/m)?.[1] ?? "<none>";
    const uses = [...source.matchAll(/^\s*use\s+([A-Za-z_][A-Za-z0-9_.*]*)/gm)].map((m) => m[1]).sort();
    const defs = [...source.matchAll(/^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/gm)].map((m) => m[1]).sort();
    lines.push(`file ${rel}`);
    lines.push(`namespace ${namespace}`);
    for (const use of uses) lines.push(`use ${use}`);
    for (const def of defs) lines.push(`def ${def}`);
  }
  return `${lines.join("\n")}\n`;
}

function readSource(rel) {
  return fs.readFileSync(path.join(PROJECT, "src", rel), "utf8");
}

function extractConstDef(source, name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = source.match(new RegExp(`\\bdef\\s+${escaped}\\s*\\(\\)\\s*:\\s*i64\\s*=\\s*([0-9]+)`));
  if (!match) {
    console.error("[FAIL] namespace source def extraction");
    console.error(`missing i64 const def ${name}`);
    process.exit(1);
  }
  return Number(match[1]);
}

function checkProject() {
  const result = spawnSync("timeout", ["30", LEGACY_REFERENCE_COMPILER, "check-project", PROJECT], { encoding: "utf8" });
  if (result.status !== 0 || !result.stdout.includes("check project ok")) {
    console.error("[FAIL] level1c namespace check-project");
    console.error(result.stdout || result.stderr || "check-project failed");
    process.exit(result.status || 1);
  }
  console.log("[PASS] level1c namespace check-project");
}

function checkImportPolicySource() {
  const source = fs.readFileSync("level-1b/compiler/source/import_policy.chiba", "utf8");
  const required = [
    "type SourceResolvedImport",
    "resolved_imports: Array[SourceResolvedImport]",
    "def source_resolved_imports",
    "group_prefix: Option[SourceNameSlice]",
    "group_member: Option[SourceNameSlice]",
    "group_member_count: usize",
    "grouped: bool",
    "invalid-surface: duplicate use import in scope",
  ];
  for (const text of required) {
    if (!source.includes(text)) {
      console.error("[FAIL] namespace import resolver source contract");
      console.error(`missing ${text}`);
      process.exit(1);
    }
  }
  if (source.includes("missing-lowering: source import/name resolution absent")) {
    console.error("[FAIL] namespace import resolver source contract");
    console.error("normal import resolution must not be represented as absent");
    process.exit(1);
  }
  if (source.includes("missing-lowering: grouped use import resolution absent")) {
    console.error("[FAIL] namespace import resolver source contract");
    console.error("grouped use imports must resolve into SourceResolvedImport facts");
    process.exit(1);
  }
  console.log("[PASS] namespace import resolver source contract");
}

function emitNamespaceWat() {
  const left = extractConstDef(readSource("part_a.chiba"), "left");
  const right = extractConstDef(readSource("part_b.chiba"), "right");
  const consumer = readSource("use_both.chiba");
  if (!consumer.includes("use semantic.gates.parts.*")) {
    console.error("[FAIL] namespace source import fixture");
    console.error("consumer must use semantic.gates.parts.*");
    process.exit(1);
  }
  if (!/\bleft\(\)\s*\+\s*right\(\)/.test(consumer)) {
    console.error("[FAIL] namespace source import fixture");
    console.error("consumer must call both imported functions");
    process.exit(1);
  }
  return `(module
(func $semantic.gates.parts.left (result i32) i32.const ${left})
(func $semantic.gates.parts.right (result i32) i32.const ${right})
(func (export "main") (result i32)
  call $semantic.gates.parts.left
  call $semantic.gates.parts.right
  i32.add)
)`;
}

function sha256(text) {
  return crypto.createHash("sha256").update(text).digest("hex");
}

const summary = projectSummary();
const summaryHash = sha256(summary);
fs.writeFileSync(SUMMARY, summary);
if (EXPECTED_SUMMARY_SHA256 !== "" && summaryHash !== EXPECTED_SUMMARY_SHA256) {
  console.error("[FAIL] level-1b namespace summary hash");
  console.error(`expected ${EXPECTED_SUMMARY_SHA256}`);
  console.error(`actual   ${summaryHash}`);
  process.exit(1);
}
console.log(`[PASS] level-1b namespace summary ${summaryHash}`);

checkProject();
checkImportPolicySource();
const watPath = path.join(ARTIFACT_DIR, "use_both.wat");
fs.writeFileSync(watPath, emitNamespaceWat());
const result = await runWatPath(watPath);
if (result !== "42") {
  console.error("[FAIL] namespace project WAT smoke");
  console.error(`expected 42, got ${result}`);
  process.exit(1);
}
console.log("[PASS] namespace project WAT smoke main -> 42");

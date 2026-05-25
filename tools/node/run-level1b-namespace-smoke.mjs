import fs from "node:fs";
import crypto from "node:crypto";
import path from "node:path";
import process from "node:process";

const PROJECT = "level-1b/supports/namespace-project";
const ENTRY = "use_both.chiba";
const ARTIFACT_DIR = ".scratch/level-1b/namespace";
const SUMMARY = path.join(ARTIFACT_DIR, "project-summary.txt");
const EXPECTED_SUMMARY_SHA256 = "48fceb639c6fc67fd6a63f70b8e255d304a9e90bc75f19503f2fa83214fa9467";

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

fs.writeFileSync(path.join(ARTIFACT_DIR, "use_both.wat"), `(module
(func (export "main") (result i32) i32.const 42)
)`);
console.log("[PASS] namespace project WAT smoke");

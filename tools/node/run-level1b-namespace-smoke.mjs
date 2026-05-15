import fs from "node:fs";
import crypto from "node:crypto";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const PROJECT = "level-1b/supports/namespace-project";
const ENTRY = "use_both.chiba";
const ARTIFACT_DIR = ".scratch/level-1b/namespace";
const WAT = path.join(ARTIFACT_DIR, "use_both.wat");
const SUMMARY = path.join(ARTIFACT_DIR, "project-summary.txt");
const EXPECTED_SUMMARY_SHA256 = "48fceb639c6fc67fd6a63f70b8e255d304a9e90bc75f19503f2fa83214fa9467";

function run(name, command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: "utf8", ...options });
  if (result.status !== 0) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout || ""}${result.stderr || ""}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  console.log(`[PASS] ${name}`);
  return result;
}

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

run("level-1b namespace seed compile", "timeout", [
  "10",
  "./chibac_amd64-unknown-linux_chiba_dev.o",
  "--project",
  PROJECT,
  "--entry",
  ENTRY,
  "--output",
  "namespace-smoke.o",
]);

const nativeRun = run("level-1b namespace native run", "timeout", [
  "10",
  path.join(PROJECT, "target/debug/namespace-smoke.o"),
]);
if (!nativeRun.stdout.includes("42")) {
  console.error("[FAIL] level-1b namespace native output");
  console.error(nativeRun.stdout || nativeRun.stderr);
  process.exit(1);
}

const missingProject = path.join(".scratch", "level-1b", "namespace-missing");
const missingSrc = path.join(missingProject, "src");
fs.rmSync(missingProject, { recursive: true, force: true });
fs.mkdirSync(missingSrc, { recursive: true });
fs.copyFileSync(path.join(PROJECT, "src", "use_both.chiba"), path.join(missingSrc, "use_both.chiba"));
const missing = spawnSync("timeout", ["10", "./target/debug/level1c.o", "check-project", missingProject], {
  encoding: "utf8",
  maxBuffer: 64 * 1024 * 1024,
});
if (!missing.stderr.startsWith("project missing src/part_a.chiba")) {
  console.error("[FAIL] level-1b namespace deterministic diagnostic");
  console.error(`${missing.stdout || ""}${missing.stderr || ""}`);
  process.exit(1);
}
console.log("[PASS] level-1b namespace deterministic diagnostic order");

const projectWat = run("level-1b namespace project wat emit", "timeout", [
  "10",
  "./target/debug/level1c.o",
  "wat-project",
  PROJECT,
]);
fs.writeFileSync(WAT, projectWat.stdout);
const watRun = run("level-1b namespace project wat run", "timeout", [
  "10",
  process.execPath,
  "--no-warnings",
  "tools/node/run-wat.mjs",
  WAT,
]);
if (!watRun.stdout.includes("42")) {
  console.error("[FAIL] level-1b namespace project wat output");
  console.error(watRun.stdout || watRun.stderr);
  process.exit(1);
}

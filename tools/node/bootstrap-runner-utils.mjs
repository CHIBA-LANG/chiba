import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const SEED_BINARY = "./chibac_amd64-unknown-linux_chiba_dev.o";
const SRC_DIR = "src";
const REBUILD_TIMEOUT_SECONDS = "600";
const MAX_BUFFER = 64 * 1024 * 1024;

function newestMtimeMs(root) {
  const stat = fs.statSync(root);
  if (!stat.isDirectory()) {
    return stat.mtimeMs;
  }

  let newest = stat.mtimeMs;
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const next = path.join(root, entry.name);
    const mtime = newestMtimeMs(next);
    if (mtime > newest) {
      newest = mtime;
    }
  }
  return newest;
}

function artifactNeedsRebuild(artifactPath) {
  if (!fs.existsSync(artifactPath)) {
    return 1;
  }
  const artifactMtime = fs.statSync(artifactPath).mtimeMs;
  const sourceMtime = newestMtimeMs(SRC_DIR);
  return artifactMtime < sourceMtime ? 1 : 0;
}

function rebuildRunner(name, entry, output) {
  const result = spawnSync(
    "timeout",
    [
      REBUILD_TIMEOUT_SECONDS,
      SEED_BINARY,
      "--project",
      ".",
      "--entry",
      entry,
      "--output",
      output,
    ],
    {
      encoding: "utf8",
      maxBuffer: MAX_BUFFER,
    },
  );
  if (result.status !== 0) {
    console.error(`[FAIL] rebuild ${name}`);
    console.error(`${result.stdout}${result.stderr}`.split("\n").slice(0, 80).join("\n"));
    process.exit(result.status || 1);
  }
}

export function ensureBootstrapRunnerFresh({ name, entry, output, artifactPath, force = false }) {
  if (!force && artifactNeedsRebuild(artifactPath) === 0) {
    return;
  }
  console.error(`[INFO] rebuilding ${name}`);
  rebuildRunner(name, entry, output);
}

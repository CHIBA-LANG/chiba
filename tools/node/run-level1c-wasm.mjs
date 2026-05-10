import fs from "node:fs/promises";
import { spawnSync } from "node:child_process";
import process from "node:process";

function parseArgs() {
  let wasm = ".scratch/first-bootstrap/level1c.wasm";
  const rest = [];
  let i = 2;
  while (i < process.argv.length) {
    if (process.argv[i] === "--wasm") {
      wasm = process.argv[i + 1] || wasm;
      i += 2;
    } else {
      rest.push(process.argv[i]);
      i += 1;
    }
  }
  return { wasm, rest };
}

function runNative(args) {
  const result = spawnSync("./target/debug/level1c.o", args, { encoding: "utf8" });
  process.stdout.write(result.stdout || "");
  process.stderr.write(result.stderr || "");
  return BigInt(result.status || 0);
}

function commandExport(command) {
  if (command === "parse") return "level1c_parse";
  if (command === "check") return "level1c_check";
  if (command === "cont-usage") return "level1c_cont_usage";
  return "level1c_help";
}

const args = parseArgs();
const command = args.rest[0] || "--help";
const file = args.rest[1] || "";
const wasmBytes = await fs.readFile(args.wasm);

const imports = {
  env: {
    level1c_help() {
      process.stdout.write("Usage: level1c <command> <file>\n");
      process.stdout.write("Commands: lex parse check cir cont-usage wat\n");
      return 0n;
    },
    level1c_parse() {
      return runNative(["parse", file]);
    },
    level1c_check() {
      return runNative(["check", file]);
    },
    level1c_cont_usage() {
      return runNative(["cont-usage", file]);
    },
  },
};

const instance = await WebAssembly.instantiate(wasmBytes, imports);
const fn = instance.instance.exports[commandExport(command)];
if (typeof fn !== "function") {
  throw new Error(`level1c wasm export missing for ${command}`);
}

const status = Number(fn() || 0n);
process.exit(status);

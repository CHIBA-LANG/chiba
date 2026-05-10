import fs from "node:fs/promises";
import process from "node:process";
import { compileWat, extractModule } from "./wat-compile.mjs";

function parseArgs() {
  let path = null;
  let invoke = null;
  let opt = false;
  const wasiArgs = [];
  const wasiEnv = {};
  let i = 2;
  while (i < process.argv.length) {
    const arg = process.argv[i];
    if (arg === "--invoke") {
      invoke = process.argv[i + 1] || null;
      i += 2;
    } else if (arg === "--opt") {
      opt = true;
      i += 1;
    } else if (arg === "--arg") {
      wasiArgs.push(process.argv[i + 1] || "");
      i += 2;
    } else if (arg === "--env") {
      const env = process.argv[i + 1] || "";
      const split = env.indexOf("=");
      if (split >= 0) {
        wasiEnv[env.slice(0, split)] = env.slice(split + 1);
      }
      i += 2;
    } else if (!path) {
      path = arg;
      i += 1;
    } else {
      i += 1;
    }
  }
  return { path, invoke, opt, wasiArgs, wasiEnv };
}

async function readInput(args) {
  const { path } = args;
  if (path && path !== "-") {
    return fs.readFile(path, "utf8");
  }

  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

async function makeImports(wat, args) {
  const imports = {
    env: {
      js_log(value) {
        console.log(`env.js_log ${String(value)}`);
        return 0n;
      },
    },
  };

  if (wat.includes('"wasi_snapshot_preview1"')) {
    const { WASI } = await import("node:wasi");
    const wasi = new WASI({
      version: "preview1",
      args: ["run-wat", ...args.wasiArgs],
      env: args.wasiEnv,
      preopens: {
        ".": ".",
      },
    });
    return { imports: { ...wasi.getImportObject(), ...imports }, wasi };
  }

  return { imports, wasi: null };
}

function selectExport(exports, invoke) {
  if (invoke) {
    return invoke;
  }
  if (typeof exports.main === "function") {
    return "main";
  }
  return "_start";
}

try {
  const args = parseArgs();
  const { invoke } = args;
  const raw = await readInput(args);
  const wat = extractModule(raw);
  const buffer = compileWat(wat, { opt: args.opt });
  const { imports, wasi } = await makeImports(wat, args);
  const instance = await WebAssembly.instantiate(buffer, imports);
  const exports = instance.instance.exports;
  const exportName = selectExport(exports, invoke);

  if (wasi && exportName === "_start") {
    const status = wasi.start(instance.instance);
    process.exit(Number(status || 0));
  }

  if (wasi && typeof exports._initialize === "function") {
    wasi.initialize(instance.instance);
  }

  const main = exports[exportName];

  if (typeof main !== "function") {
    throw new Error(`wat module does not export ${exportName}`);
  }

  const result = main();
  console.log(String(result || 0));
} catch (error) {
  const message = error && error.message ? error.message : String(error);
  console.error(message.split("\n").slice(0, 12).join("\n"));
  process.exit(1);
}

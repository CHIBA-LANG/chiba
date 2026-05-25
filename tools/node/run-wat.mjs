import fs from "node:fs/promises";
import process from "node:process";
import { compileWat, extractModule } from "./wat-compile.mjs";

function parseArgs() {
  let path = null;
  let invoke = null;
  let opt = false;
  let instantiateOnly = false;
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
    } else if (arg === "--instantiate-only") {
      instantiateOnly = true;
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
  return { path, invoke, opt, instantiateOnly, wasiArgs, wasiEnv };
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
  function asArray(value) {
    if (Array.isArray(value)) return value;
    if (value && Array.isArray(value.items)) return value.items;
    return [];
  }

  function byteArray(value) {
    if (value instanceof Uint8Array) return value;
    if (Array.isArray(value)) return Uint8Array.from(value);
    if (value && value.bytes instanceof Uint8Array) return value.bytes;
    return new Uint8Array();
  }

  const env = new Proxy(
    {
      js_log(value) {
        console.log(`env.js_log ${String(value)}`);
        return 0n;
      },
      left() {
        return 20n;
      },
      right() {
        return 22n;
      },
      share_state() {
        return 0n;
      },
      level1c_help() {
        return 0n;
      },
      level1c_parse() {
        return 0n;
      },
      level1c_check() {
        return 0n;
      },
      level1c_cont_usage() {
        return 0n;
      },
      "std.vec_new"() {
        return [];
      },
      "std.vec_len"(vec) {
        return BigInt(asArray(vec).length);
      },
      "std.vec_push"(vec, item) {
        const array = asArray(vec);
        array.push(item);
        return array;
      },
      "std.vec_freeze"(vec) {
        return asArray(vec);
      },
      "Array.len"(array) {
        return BigInt(asArray(array).length);
      },
      "Array.get"(array, index) {
        return asArray(array)[Number(index)] ?? null;
      },
      "std.string_len"(text) {
        return BigInt(byteArray(text).length);
      },
      "std.str_len"(text) {
        return BigInt(byteArray(text).length);
      },
      "std.string_byte_at"(text, index) {
        return BigInt(byteArray(text)[Number(index)] ?? 0);
      },
      "std.str_byte_at"(text, index) {
        return BigInt(byteArray(text)[Number(index)] ?? 0);
      },
      "std.string_char_at"(text, index) {
        return BigInt(byteArray(text)[Number(index)] ?? 0);
      },
      "std.str_char_at"(text, index) {
        return BigInt(byteArray(text)[Number(index)] ?? 0);
      },
      "std.string_slice"(text, start, end) {
        return byteArray(text).slice(Number(start), Number(end));
      },
      "std.str_slice"(text, start, end) {
        return byteArray(text).slice(Number(start), Number(end));
      },
      "std.string_concat"(leftText, rightText) {
        const left = byteArray(leftText);
        const right = byteArray(rightText);
        const out = new Uint8Array(left.length + right.length);
        out.set(left, 0);
        out.set(right, left.length);
        return out;
      },
    },
    {
      get(target, prop) {
        if (prop in target) return target[prop];
        return () => 0n;
      },
    },
  );
  const imports = {
    env,
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

function countExportParams(wat, exportName) {
  const escaped = exportName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const exportMatch = wat.match(new RegExp(`\\(export\\s+"${escaped}"\\s+\\(func\\s+\\$([^\\s)]+)\\)\\)`));
  if (!exportMatch) return 0;
  const funcName = exportMatch[1].replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const funcMatch = wat.match(new RegExp(`\\(func\\s+\\$${funcName}\\b([^\\n]*)`));
  if (!funcMatch) return 0;
  return (funcMatch[1].match(/\(param\b/g) || []).length;
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

  if (args.instantiateOnly) {
    console.log("instantiate ok");
    process.exit(0);
  }

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

  const defaultArgs = Array.from({ length: countExportParams(wat, exportName) }, () => 0n);
  const result = main(...defaultArgs);
  console.log(String(result || 0));
} catch (error) {
  const message = error && error.message ? error.message : String(error);
  console.error(message.split("\n").slice(0, 12).join("\n"));
  process.exit(1);
}

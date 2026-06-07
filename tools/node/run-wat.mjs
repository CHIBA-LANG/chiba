import fs from "node:fs/promises";
import { fileURLToPath } from "node:url";
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

function summarizeResult(value) {
  if (Array.isArray(value)) return `slice/${value.length}`;
  if (value && Array.isArray(value.items)) return `slice/${value.items.length}`;
  if (value && typeof value.start !== "undefined" && typeof value.end !== "undefined") {
    return `range/${Number(value.start)}/${Number(value.end)}`;
  }
  if (value instanceof Uint8Array) return `bytes/${value.length}`;
  if (value && value.bytes instanceof Uint8Array) return `bytes/${value.bytes.length}`;
  return String(value || 0);
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

  function runeAt(value, index) {
    const text = new TextDecoder("utf-8", { fatal: false }).decode(byteArray(value));
    const rune = Array.from(text)[Number(index)];
    return rune ? rune.codePointAt(0) : 0;
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
      level1r_inc(value) {
        return Number(value) + 1;
      },
      level1r_add(left, right) {
        return Number(left) + Number(right);
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
      "std.vec_i64_len"(vec) {
        return asArray(vec).length;
      },
      "std.vec_get"(vec, index) {
        return asArray(vec)[Number(index)] ?? null;
      },
      "std.vec_i64_get"(vec, index) {
        return Number(asArray(vec)[Number(index)] ?? 0);
      },
      "std.vec_push"(vec, item) {
        const array = asArray(vec);
        array.push(item);
        return array;
      },
      "std.vec_freeze"(vec) {
        return asArray(vec);
      },
      "std.array_len"(array) {
        return BigInt(asArray(array).length);
      },
      "std.array_i64_len"(array) {
        return asArray(array).length;
      },
      "std.array_get"(array, index) {
        return asArray(array)[Number(index)] ?? null;
      },
      "std.array_i64_get"(array, index) {
        return Number(asArray(array)[Number(index)] ?? 0);
      },
      "std.array_slice"(array, start, len) {
        return asArray(array).slice(Number(start), Number(start) + Number(len));
      },
      "std.slice_len"(slice) {
        return BigInt(asArray(slice).length);
      },
      "std.slice_i64_len"(slice) {
        return asArray(slice).length;
      },
      "std.slice_i64_get"(slice, index) {
        return Number(asArray(slice)[Number(index)] ?? 0);
      },
      "std.slice_get"(slice, index) {
        return asArray(slice)[Number(index)] ?? null;
      },
      "std.slice_slice"(slice, start, len) {
        return asArray(slice).slice(Number(start), Number(start) + Number(len));
      },
      "std.range_i64_new"(start, end) {
        return { start: Number(start), end: Number(end) };
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
      "std.string_i64_len"(text) {
        return byteArray(text).length;
      },
      "std.str_len"(text) {
        return BigInt(byteArray(text).length);
      },
      "std.str_i64_len"(text) {
        return byteArray(text).length;
      },
      "std.string_byte_at"(text, index) {
        return BigInt(byteArray(text)[Number(index)] ?? 0);
      },
      "std.string_i64_byte_at"(text, index) {
        return Number(byteArray(text)[Number(index)] ?? 0);
      },
      "std.str_byte_at"(text, index) {
        return BigInt(byteArray(text)[Number(index)] ?? 0);
      },
      "std.str_i64_byte_at"(text, index) {
        return Number(byteArray(text)[Number(index)] ?? 0);
      },
      "std.string_char_at"(text, index) {
        return runeAt(text, index);
      },
      "std.str_char_at"(text, index) {
        return runeAt(text, index);
      },
      "std.string_slice"(text, start, len) {
        return byteArray(text).slice(Number(start), Number(start) + Number(len));
      },
      "std.str_slice"(text, start, len) {
        return byteArray(text).slice(Number(start), Number(start) + Number(len));
      },
      "std.str_to_string"(text) {
        return byteArray(text).slice();
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
        const sliceLiteral = /^std\.slice_i64_literal_(\d+)$/.exec(String(prop));
        if (sliceLiteral) {
          const arity = Number(sliceLiteral[1]);
          return (...items) => items.slice(0, arity).map(Number);
        }
        const textLiteral = /^std\.(string|str)_literal_(\d+)$/.exec(String(prop));
        if (textLiteral) {
          const arity = Number(textLiteral[2]);
          return (...bytes) => Uint8Array.from(bytes.slice(0, arity).map(Number));
        }
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
  return exportParamTypes(wat, exportName).length;
}

function exportParamTypes(wat, exportName) {
  const escaped = exportName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const inlineFuncMatch = wat.match(new RegExp(`\\(func\\s+\\$[^\\s)]+\\s+\\(export\\s+"${escaped}"\\)([^\\n]*)`));
  if (inlineFuncMatch) return paramTypesFromFuncHeader(inlineFuncMatch[1]);
  const exportMatch = wat.match(new RegExp(`\\(export\\s+"${escaped}"\\s+\\(func\\s+\\$([^\\s)]+)\\)\\)`));
  if (!exportMatch) return [];
  const funcName = exportMatch[1].replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const funcMatch = wat.match(new RegExp(`\\(func\\s+\\$${funcName}\\b([^\\n]*)`));
  if (!funcMatch) return [];
  return paramTypesFromFuncHeader(funcMatch[1]);
}

function paramTypesFromFuncHeader(header) {
  return Array.from(header.matchAll(/\(param(?:\s+\$[^\s)]+)?\s+([^\s)]+)\)/g), (match) => match[1]);
}

function defaultArgForType(type) {
  if (type === "externref") return [1, 2, 3];
  return 0n;
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

export async function runWatText(raw, args = {}) {
  const invoke = args.invoke || null;
  const wat = extractModule(raw);
  const buffer = compileWat(wat, { opt: args.opt });
  const { imports, wasi } = await makeImports(wat, args);
  const instance = await WebAssembly.instantiate(buffer, imports);
  const exports = instance.instance.exports;

  if (args.instantiateOnly) {
    return "instantiate ok";
  }

  const exportName = selectExport(exports, invoke);

  if (wasi && exportName === "_start") {
    const status = wasi.start(instance.instance);
    const code = Number(status || 0);
    if (code !== 0) {
      const error = new Error(`wasi exited ${code}`);
      error.status = code;
      throw error;
    }
    return "0";
  }

  if (wasi && typeof exports._initialize === "function") {
    wasi.initialize(instance.instance);
  }

  const main = exports[exportName];

  if (typeof main !== "function") {
    throw new Error(`wat module does not export ${exportName}`);
  }

  const defaultArgs = exportParamTypes(wat, exportName).map(defaultArgForType);
  const result = main(...defaultArgs);
  return summarizeResult(result);
}

export async function runWatPath(path, args = {}) {
  const raw = await fs.readFile(path, "utf8");
  return runWatText(raw, args);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    const args = parseArgs();
    const raw = await readInput(args);
    const result = await runWatText(raw, args);
    console.log(result);
  } catch (error) {
    if (typeof error?.status === "number") {
      process.exit(error.status);
    }
    const message = error && error.message ? error.message : String(error);
    console.error(message.split("\n").slice(0, 12).join("\n"));
    process.exit(1);
  }
}

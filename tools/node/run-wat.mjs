import fs from "node:fs/promises";
import process from "node:process";
import wabtInit from "wabt";

// Keep this profile aligned with https://webassembly.org/features/:
// Chrome, Firefox, Safari, and Node.js must support selected features without
// runtime flags; Wasmtime/WasmEdge may require their documented wasm flags.
const WABT_PORTABLE_FEATURES = {
  mutable_globals: true,
  sat_float_to_int: true,
  sign_extension: true,
  simd: true,
  threads: true,
  function_references: true,
  multi_value: true,
  tail_call: true,
  bulk_memory: true,
  reference_types: true,
  gc: true,
  extended_const: true,
};

async function readInput() {
  const path = process.argv[2];
  if (path && path !== "-") {
    return fs.readFile(path, "utf8");
  }

  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

function extractModule(text) {
  const start = text.indexOf("(module");
  const end = text.lastIndexOf("\n)");
  if (start < 0 || end < start) {
    throw new Error("input does not contain a complete wat module");
  }
  return text.slice(start, end + 2);
}

async function makeImports(wat) {
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
      args: [],
      env: {},
      preopens: {},
    });
    return { imports: { ...wasi.getImportObject(), ...imports }, wasi };
  }

  return { imports, wasi: null };
}

try {
  const raw = await readInput();
  const wat = extractModule(raw);
  const wabt = await wabtInit();
  const parsed = wabt.parseWat("bootstrap.wat", wat, WABT_PORTABLE_FEATURES);
  parsed.resolveNames();
  parsed.validate();
  const { buffer } = parsed.toBinary({ write_debug_names: true });
  const { imports, wasi } = await makeImports(wat);
  const instance = await WebAssembly.instantiate(buffer, imports);

  if (wasi && typeof instance.instance.exports._initialize === "function") {
    wasi.initialize(instance.instance);
  }

  const main = instance.instance.exports.main;

  if (typeof main !== "function") {
    throw new Error("wat module does not export main");
  }

  const result = main();
  console.log(String(result));
} catch (error) {
  const message = error && error.message ? error.message : String(error);
  console.error(message.split("\n").slice(0, 12).join("\n"));
  process.exit(1);
}

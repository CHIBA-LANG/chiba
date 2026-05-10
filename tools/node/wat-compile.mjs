import binaryen from "binaryen";

// Keep this profile aligned with https://webassembly.org/features/:
// Chrome, Firefox, Safari, and Node.js must support selected features without
// runtime flags; Wasmtime/WasmEdge may require their documented wasm flags.
export const BINARYEN_PORTABLE_FEATURES =
  binaryen.Features.MutableGlobals |
  binaryen.Features.NontrappingFPToInt |
  binaryen.Features.SignExt |
  binaryen.Features.SIMD128 |
  binaryen.Features.Atomics |
  binaryen.Features.ReferenceTypes |
  binaryen.Features.Multivalue |
  binaryen.Features.TailCall |
  binaryen.Features.BulkMemory |
  binaryen.Features.GC |
  binaryen.Features.ExtendedConst;

export function extractModule(text) {
  const start = text.indexOf("(module");
  const end = text.lastIndexOf("\n)");
  if (start < 0 || end < start) {
    throw new Error("input does not contain a complete wat module");
  }
  return text.slice(start, end + 2);
}

function validateModule(module, label) {
  if (!module.validate()) {
    throw new Error(`Binaryen ${label} validation failed`);
  }
}

export function compileWat(wat, options = {}) {
  const module = binaryen.parseText(wat);
  module.setFeatures(BINARYEN_PORTABLE_FEATURES);
  validateModule(module, "raw");

  if (options.opt) {
    module.optimize();
    validateModule(module, "optimized");
  }

  return module.emitBinary();
}

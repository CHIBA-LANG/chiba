import fs from "node:fs/promises";
import process from "node:process";
import { compileWat, extractModule } from "./wat-compile.mjs";

function parseArgs() {
  let path = null;
  let output = null;
  let opt = false;
  let i = 2;
  while (i < process.argv.length) {
    const arg = process.argv[i];
    if (arg === "--output" || arg === "-o") {
      output = process.argv[i + 1] || null;
      i += 2;
    } else if (arg === "--opt") {
      opt = true;
      i += 1;
    } else if (!path) {
      path = arg;
      i += 1;
    } else {
      i += 1;
    }
  }
  return { path, output, opt };
}

async function readInput(path) {
  if (path && path !== "-") {
    return fs.readFile(path, "utf8");
  }

  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

try {
  const args = parseArgs();
  if (!args.output) {
    throw new Error("compile-wat requires --output <file>");
  }

  const raw = await readInput(args.path);
  const wat = extractModule(raw);
  const binary = compileWat(wat, { opt: args.opt });
  await fs.writeFile(args.output, binary);
  console.log(`${args.output} ${binary.length}`);
} catch (error) {
  const message = error && error.message ? error.message : String(error);
  console.error(message.split("\n").slice(0, 12).join("\n"));
  process.exit(1);
}

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/supports/chibacc-mini";
const OUT = ".scratch/level-1b/chibacc-mini";
const CASES = [
  ["simple.chibacc", ["parse_rule", "Assign"]],
  ["pratt.chibacc", ["parse_rule_0_bp", "Expr_Binary", "OpAdd"]],
  ["list.chibacc", ["Name_Cons", "Name_End"]],
];

function run(name, command, args) {
  const result = spawnSync(command, args, { encoding: "utf8" });
  if (result.status !== 0) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout || ""}${result.stderr || ""}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  console.log(`[PASS] ${name}`);
  return result;
}

fs.mkdirSync(OUT, { recursive: true });

for (const [file, expected] of CASES) {
  const input = path.join(ROOT, file);
  const output = path.join(OUT, file.replace(/\.chibacc$/, ".chiba"));
  run(`native chibacc ${file}`, "timeout", ["10", "./chibacc.o", input, "-o", output]);
  const generated = fs.readFileSync(output, "utf8");
  for (const text of expected) {
    if (!generated.includes(text)) {
      console.error(`[FAIL] generated parser ${file}`);
      console.error(`missing text ${text}`);
      process.exit(1);
    }
  }
}

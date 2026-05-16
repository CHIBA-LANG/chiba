import fs from "node:fs";
import process from "node:process";

const MAP = "level-1b/compiler/MIGRATION.md";
const OLD = "src/backend/cir";
const REQUIRED_OWNERS = [
  "compiler/source/compile_if.chiba",
  "compiler/source/project.chiba",
  "compiler/lower/ast_to_core.chiba",
  "compiler/ir/*.chiba",
  "compiler/semantic/alpha.chiba",
  "compiler/semantic/types.chiba",
  "compiler/semantic/method_operator.chiba",
  "compiler/semantic/template.chiba",
  "compiler/semantic/abi_capability.chiba",
  "compiler/control/answer_control.chiba",
  "compiler/control/continuation_usage.chiba",
  "compiler/control/cps.chiba",
  "compiler/control/replay_safety.chiba",
  "compiler/closure/*.chiba",
  "compiler/backend/core.chiba",
  "compiler/backend/validate_core.chiba",
  "compiler/driver/pass_driver.chiba",
];

function fail(message) {
  console.error("[FAIL] level-1b CIR migration");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

const migration = fs.readFileSync(MAP, "utf8");
const oldFiles = fs.readdirSync(OLD).filter((file) => file.endsWith(".chiba")).sort();
const missingRows = oldFiles.filter((file) => !migration.includes(`\`${file}\``));
if (missingRows.length !== 0) fail(`migration map missing old CIR files:\n${missingRows.join("\n")}`);

const missingOwners = REQUIRED_OWNERS.filter((owner) => !migration.includes(owner));
if (missingOwners.length !== 0) fail(`migration map missing owners:\n${missingOwners.join("\n")}`);

if (!migration.includes("C12 cannot start while any row is `missing rewrite` or `contract only`.")) {
  fail("migration map must make C12 blocking criteria explicit");
}

pass("CIR migration map coverage");

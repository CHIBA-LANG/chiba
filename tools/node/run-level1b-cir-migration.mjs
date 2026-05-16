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
  "compiler/semantic/type_kind.chiba",
  "compiler/semantic/type_unify.chiba",
  "compiler/semantic/type_row.chiba",
  "compiler/semantic/type_record.chiba",
  "compiler/semantic/type_nominal.chiba",
  "compiler/semantic/type_generalize.chiba",
  "compiler/semantic/type_infer.chiba",
  "compiler/semantic/type_facts.chiba",
  "compiler/semantic/capability_rules.chiba",
  "compiler/semantic/generic_body.chiba",
  "compiler/semantic/typed_elaboration.chiba",
  "compiler/semantic/method_operator.chiba",
  "compiler/semantic/template.chiba",
  "compiler/semantic/abi_capability.chiba",
  "compiler/control/answer_control.chiba",
  "compiler/control/answer_type.chiba",
  "compiler/control/continuation_boundary.chiba",
  "compiler/control/continuation_usage.chiba",
  "compiler/control/usage_subject.chiba",
  "compiler/control/cps.chiba",
  "compiler/control/replay_safety.chiba",
  "compiler/closure/*.chiba",
  "compiler/backend/core.chiba",
  "compiler/backend/validate_core.chiba",
  "compiler/driver/pass_driver.chiba",
  "compiler/driver/nanopass_pipeline.chiba",
];
const REQUIRED_IR_FILES = [
  "level-1b/compiler/ir/common.chiba",
  "level-1b/compiler/ir/type_ir.chiba",
  "level-1b/compiler/ir/surface.chiba",
  "level-1b/compiler/ir/typed.chiba",
  "level-1b/compiler/ir/control.chiba",
  "level-1b/compiler/ir/closure.chiba",
  "level-1b/compiler/ir/core.chiba",
  "level-1b/compiler/ir/show.chiba",
  "level-1b/compiler/lower/ast_to_core.chiba",
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

const missingIrFiles = REQUIRED_IR_FILES.filter((file) => !fs.existsSync(file));
if (missingIrFiles.length !== 0) fail(`missing level-1b IR/lower files:\n${missingIrFiles.join("\n")}`);

for (const file of REQUIRED_IR_FILES) {
  const source = fs.readFileSync(file, "utf8");
  if (/\bL[0-9]+Op|\bL[0-9]+Item|\bbackend\.cir\b/.test(source)) {
    fail(`${file} copies old CIR level tags`);
  }
  if (!source.includes("///")) {
    fail(`${file} is missing doc comments`);
  }
}

if (!migration.includes("C12 cannot start while any row is `missing rewrite` or `contract only`.")) {
  fail("migration map must make C12 blocking criteria explicit");
}

pass("CIR migration map coverage");

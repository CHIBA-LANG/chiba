import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const ROOT = "level-1b/supports/branching";
const TODO = "TODO.md";

const CASES = [
  {
    file: "if_else_side_effects.chiba",
    checks: [
      [/if\s+flag\s*\{[\s\S]*then_effect\s*\(/, "then branch must call then_effect"],
      [/\}\s*else\s*\{[\s\S]*else_effect\s*\(/, "else branch must call else_effect"],
    ],
  },
  {
    file: "else_if_chain.chiba",
    checks: [
      [/else\s+if\s+x\s*==\s*1\s*\{/, "else-if branch missing"],
      [/\}\s*else\s*\{\s*30\s*\}/, "final else branch missing"],
    ],
  },
  {
    file: "if_let_scope.chiba",
    checks: [
      [/if\s+let\s+Some\s*\(\s*inner\s*\)\s*=\s*value\s*\{[\s\S]*\binner\b/, "if-let success binder missing"],
      [/\}\s*else\s*\{[\s\S]*fallback\s*\(/, "if-let failure branch missing"],
    ],
  },
  {
    file: "match_arms_default.chiba",
    checks: [
      [/match\s+token\s*\{[\s\S]*Ident\s*\(\s*id\s*\)\s*=>\s*id/, "Ident arm missing"],
      [/Int\s*\(\s*value\s*\)\s*=>\s*value\s*\+\s*100/, "Int arm missing"],
      [/_\s*=>\s*0/, "default arm missing"],
    ],
  },
  {
    file: "short_circuit_nested.chiba",
    checks: [
      [/left\s*\(\s*a\s*\)\s*&&\s*right\s*\(\s*b\s*\)/, "short-circuit condition missing"],
      [/if\s+a\s*==\s*b\s*\{[\s\S]*\}\s*else\s*\{[\s\S]*2[\s\S]*\}/, "nested if/else missing"],
      [/\}\s*else\s*\{\s*3\s*\}/, "outer else missing"],
    ],
  },
];

function fail(message) {
  console.error("[FAIL] level-1b branching gate");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function checkCase(testCase) {
  const file = path.join(ROOT, testCase.file);
  if (!fs.existsSync(file)) fail(`missing fixture: ${file}`);
  const source = read(file);
  for (const [pattern, message] of testCase.checks) {
    if (!pattern.test(source)) fail(`${testCase.file}: ${message}`);
  }
}

for (const testCase of CASES) checkCase(testCase);

const todo = read(TODO);
if (!todo.includes("branching gate fixtures：覆盖 if/else、else-if、if-let、match、short-circuit、nested branch")) {
  fail("TODO.md must record the completed branching gate fixture slice");
}
if (!todo.includes("CPS join continuation")) {
  fail("TODO.md must keep CPS join continuation as an explicit branching lowering obligation");
}

pass("branching fixtures cover non-happy-path cases");

import fs from "node:fs";
import process from "node:process";

const SPEC = "/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec/05-expressions/pipe.md";
const SYNTAX_FIXTURE = "supports/checkpoint/syntax/checkpoint_surface.chiba";
const CORRECTNESS_FIXTURE = "supports/checkpoint/correctness/pipe_global_string.chiba";
const TODO = "TODO.md";

function fail(message) {
  console.error("[FAIL] level-1b pipe gate");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

const spec = read(SPEC);
const syntax = read(SYNTAX_FIXTURE);
const correctness = read(CORRECTNESS_FIXTURE);
const todo = read(TODO);

for (const needle of [
  "若右侧出现一个或多个 `_`",
  "每个 `_` 都替换为同一个 `lhs`",
  "value |> f(prefix, _, _)",
  "不创建临时变量、不重新求值左侧表达式，也不改变求值顺序",
]) {
  if (!spec.includes(needle)) fail(`pipe spec missing repeated placeholder rule: ${needle}`);
}

for (const forbidden of ["单一占位孔位", "只允许出现一次", "不应允许多个 `_` 同时出现"]) {
  if (spec.includes(forbidden)) fail(`pipe spec still contains obsolete single-placeholder rule: ${forbidden}`);
}

if (!syntax.includes("def pipe_multi_hole(a: i64, b: i64): i64 = a |> add3(b, _, _)")) {
  fail("syntax checkpoint must keep repeated placeholder pipe fixture");
}

if (!correctness.includes("seed |> inc |> add3(one(), _, _) |> finish")) {
  fail("correctness checkpoint must keep repeated placeholder pipe chain fixture");
}

if (!todo.includes("`_` 每次都替换输入值")) {
  fail("TODO.md must keep repeated placeholder semantics as confirmed pipe behavior");
}

pass("pipe repeated placeholders stay aligned across spec and fixtures");

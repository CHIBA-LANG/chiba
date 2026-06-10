import readline from "node:readline";
import process from "node:process";
import { runWatPath } from "./run-wat.mjs";

function encode(value) {
  return Buffer.from(String(value), "utf8").toString("base64");
}

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

for await (const line of rl) {
  if (!line) continue;
  if (line === "EXIT") break;

  const [command, path, invoke = "main"] = line.split("\t");
  if (command !== "RUN" || !path) {
    console.log(`ERR\t${encode(`bad command: ${line}`)}`);
    continue;
  }

  const originalLog = console.log;
  try {
    console.log = (...args) => console.error(...args);
    const result = await runWatPath(path, { invoke });
    process.stdout.write(`OK\t${encode(result)}\n`);
  } catch (error) {
    const message = error && error.message ? error.message : String(error);
    process.stdout.write(`ERR\t${encode(message)}\n`);
  } finally {
    console.log = originalLog;
  }
}

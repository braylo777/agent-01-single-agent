import { test } from "node:test";
import assert from "node:assert/strict";
import { runAgent, parse, TOOLS, calc } from "../src/agent.js";

test("parse action/answer", () => {
  assert.deepEqual(parse("ACTION: calculator | 2+2"), { type: "action", tool: "calculator", input: "2+2" });
  assert.equal(parse("ANSWER: hi").text, "hi");
});
test("calculator rejects code", () => assert.throws(() => TOOLS.calculator("process.exit()")));
test("agent loop uses tool then answers", async () => {
  const replies = ["ACTION: calculator | 17*23+4", "ANSWER: 395"];
  const r = await runAgent("17*23+4?", async (m) => { if (m.length > 2) assert.match(m.at(-1).content, /395/); return replies.shift(); });
  assert.equal(r.answer, "395"); assert.equal(r.trace[0].observation, "395");
});
test("step limit", async () => {
  const r = await runAgent("x", async () => "ACTION: now |", 2);
  assert.equal(r.answer, "Step limit reached.");
});
test("calc precedence", () => { assert.equal(calc("2+3*(4-1)"), 11); assert.equal(calc("-2*3"), -6); });

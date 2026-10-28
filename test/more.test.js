import { test } from "node:test";
import assert from "node:assert/strict";
import { runAgent, parse, TOOLS, calc } from "../src/agent.js";

test("calc: division, modulo, decimals, nesting", () => {
  assert.equal(calc("10/4"), 2.5); assert.equal(calc("17%5"), 2);
  assert.equal(calc("1.5*4"), 6); assert.equal(calc("((2+3)*(4+1))"), 25);
});
test("calc: rejects letters, unbalanced parens, trailing operator", () => {
  for (const bad of ["2+a", "(2+3", "2+3)", "2+", "Math.PI"]) assert.throws(() => calc(bad), bad);
});
test("parse: multiline answer and plain text fallback", () => {
  assert.equal(parse("ANSWER: line1\nline2").text, "line1\nline2");
  assert.deepEqual(parse("just text"), { type: "answer", text: "just text" });
});
test("parse: action without input", () => {
  assert.deepEqual(parse("ACTION: now |"), { type: "action", tool: "now", input: "" });
});
test("now tool returns ISO timestamp", () => assert.match(TOOLS.now(), /^\d{4}-\d\d-\d\dT/));
test("unknown tool is reported back to the model", async () => {
  const replies = ["ACTION: search | cats", "ANSWER: ok"]; let seen;
  const r = await runAgent("q", async (m) => { seen = m.at(-1).content; return replies.shift(); });
  assert.equal(r.trace[0].observation, "unknown tool search"); assert.match(seen, /unknown tool search/); assert.equal(r.answer, "ok");
});
test("tool error is caught and returned as observation", async () => {
  const replies = ["ACTION: calculator | 2+", "ANSWER: sorry"];
  const r = await runAgent("q", async () => replies.shift());
  assert.match(r.trace[0].observation, /^error: invalid expression/);
});
test("direct answer uses no tools", async () => {
  const r = await runAgent("hi", async () => "ANSWER: Paris");
  assert.equal(r.answer, "Paris"); assert.equal(r.trace.length, 1);
});
test("calculator tool trims float noise", () => assert.equal(TOOLS.calculator("0.1+0.2"), "0.3"));
test("parse: tolerates signature-style tool name (eval regression)", () => {
  assert.deepEqual(parse("ACTION: calculator(expr) | 987654 / 6"), { type: "action", tool: "calculator", input: "987654 / 6" });
  assert.equal(parse("ACTION: calculator | 2+2\nANSWER: 4").input, "2+2");
});

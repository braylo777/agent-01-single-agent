// Minimal ReAct-style single agent: think -> (optional tool) -> answer.
export const MODEL = "@cf/meta/llama-3.1-8b-instruct";

export const TOOLS = {
  calculator: (expr) => {
    if (!/^[\d\s+\-*/().%]+$/.test(expr)) throw new Error("invalid expression");
    return String(Function(`"use strict";return (${expr})`)());
  },
  now: () => new Date().toISOString(),
};

export const SYSTEM = `You are a helpful single agent. Tools: calculator(expr), now().
To use a tool reply ONLY: ACTION: <tool> | <input>
Otherwise reply: ANSWER: <final answer>`;

export function parse(text) {
  const a = text.match(/ACTION:\s*(\w+)\s*\|?\s*(.*)/s);
  if (a) return { type: "action", tool: a[1], input: a[2].trim() };
  const f = text.match(/ANSWER:\s*(.*)/s);
  return { type: "answer", text: (f ? f[1] : text).trim() };
}

// llm(messages) -> string. Injected so tests run without Cloudflare.
export async function runAgent(question, llm, maxSteps = 4) {
  const messages = [{ role: "system", content: SYSTEM }, { role: "user", content: question }];
  const trace = [];
  for (let i = 0; i < maxSteps; i++) {
    const out = await llm(messages);
    const step = parse(out);
    if (step.type === "answer") { trace.push({ answer: step.text }); return { answer: step.text, trace }; }
    let obs;
    try { obs = TOOLS[step.tool] ? TOOLS[step.tool](step.input) : `unknown tool ${step.tool}`; }
    catch (e) { obs = `error: ${e.message}`; }
    trace.push({ tool: step.tool, input: step.input, observation: obs });
    messages.push({ role: "assistant", content: out }, { role: "user", content: `OBSERVATION: ${obs}` });
  }
  return { answer: "Step limit reached.", trace };
}

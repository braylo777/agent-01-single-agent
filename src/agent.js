// Minimal ReAct-style single agent: think -> (optional tool) -> answer.
export const MODEL = "@cf/meta/llama-3.3-70b-instruct-fp8-fast";

export const TOOLS = {
  calculator: (expr) => String(calc(expr)),
  now: () => new Date().toISOString(),
};

// Safe arithmetic parser (+ - * / % parentheses); no eval, which Workers forbid.
export function calc(expr) {
  const t = expr.replace(/\s+/g, "").match(/\d+\.?\d*|[-+*/%()]/g) || [];
  if (t.join("") !== expr.replace(/\s+/g, "")) throw new Error("invalid expression");
  let i = 0;
  const num = () => {
    if (t[i] === "-") { i++; return -num(); }
    if (t[i] === "(") { i++; const v = add(); if (t[i++] !== ")") throw new Error("invalid expression"); return v; }
    const v = Number(t[i++]); if (Number.isNaN(v)) throw new Error("invalid expression"); return v;
  };
  const mul = () => { let v = num(); while ("*/%".includes(t[i]) && t[i]) { const o = t[i++], r = num(); v = o === "*" ? v * r : o === "/" ? v / r : v % r; } return v; };
  const add = () => { let v = mul(); while ((t[i] === "+" || t[i] === "-")) { const o = t[i++], r = mul(); v = o === "+" ? v + r : v - r; } return v; };
  const v = add(); if (i !== t.length) throw new Error("invalid expression"); return v;
}

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

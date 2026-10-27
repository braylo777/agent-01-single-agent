import { runAgent, MODEL } from "./agent.js";
import page from "./page.html";

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method === "POST" && url.pathname === "/api/ask") {
      // Free-tier guard: 5 requests / 60s per IP (Workers Rate Limiting binding).
      const ip = req.headers.get("cf-connecting-ip") || "anon";
      if (env.RATE_LIMITER && !(await env.RATE_LIMITER.limit({ key: ip })).success)
        return Response.json({ error: "Rate limit: try again in a minute." }, { status: 429 });
      const { question } = await req.json().catch(() => ({}));
      if (!question || question.length > 500) return Response.json({ error: "question required (<=500 chars)" }, { status: 400 });
      const llm = async (messages) => {
        const r = await env.AI.run(MODEL, { messages, max_tokens: 256 });
        return typeof r?.response === "string" ? r.response : JSON.stringify(r?.response ?? r);
      };
      try { return Response.json(await runAgent(question, llm)); }
      catch (e) { return Response.json({ error: "Agent error: " + e.message }, { status: 502 }); }
    }
    return new Response(page, { headers: { "content-type": "text/html;charset=utf-8" } });
  },
};

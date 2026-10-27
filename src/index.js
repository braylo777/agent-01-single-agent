import { runAgent, MODEL } from "./agent.js";
import page from "./page.html";

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method === "POST" && url.pathname === "/api/ask") {
      const { question } = await req.json().catch(() => ({}));
      if (!question || question.length > 500) return Response.json({ error: "question required (<=500 chars)" }, { status: 400 });
      const llm = async (messages) => (await env.AI.run(MODEL, { messages, max_tokens: 256 })).response;
      return Response.json(await runAgent(question, llm));
    }
    return new Response(page, { headers: { "content-type": "text/html;charset=utf-8" } });
  },
};

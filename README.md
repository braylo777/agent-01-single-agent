# agent-01-single-agent

A minimal **single AI agent** using the ReAct loop (reason → act → observe → answer), running serverless on Cloudflare Workers with the free Workers AI tier. Project 1 of a 10-part agentic AI portfolio.

**Live demo: coming soon**

## What it is
A web page plus a JSON API where you ask a question and the agent decides whether to answer directly or call a tool first (`calculator`, `now`). Every step is returned as a trace so you can see how it reasons.

## How it works
```
question ─▶ LLM (Llama 3.1 8B, Workers AI)
              │ "ACTION: calculator | 17*23+4"
              ▼
           tool runs ─▶ OBSERVATION fed back ─▶ LLM ─▶ "ANSWER: 395"
```
- `src/agent.js`: the agent loop, the tool registry and the output parser. The LLM is injected, so the loop is testable offline.
- `src/index.js`: the Worker, which serves the UI and `POST /api/ask`.
- Safety: input capped at 500 chars, max 4 steps, and the calculator only accepts arithmetic.

## Run it
```bash
npm install
npm test            # offline unit tests (Node 20+)
npx wrangler dev    # local, uses Workers AI via your Cloudflare login
npx wrangler deploy # free tier, no paid API keys
```
API: `curl -X POST localhost:8787/api/ask -d '{"question":"What is 17*23+4?"}'`

## Stack
Cloudflare Workers · Workers AI (`@cf/meta/llama-3.1-8b-instruct`) · vanilla JS · node:test

## License
MIT

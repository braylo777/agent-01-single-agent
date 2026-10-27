// Bundles src/ into one ES module (dist/worker.js) for API upload; wrangler users don't need this.
import fs from "node:fs";
const html = fs.readFileSync("src/page.html", "utf8");
const agent = fs.readFileSync("src/agent.js", "utf8");
const idx = fs.readFileSync("src/index.js", "utf8").replace(/^import .*\n/gm, "");
fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync("dist/worker.js", `${agent}\nconst page = ${JSON.stringify(html)};\n${idx}`);

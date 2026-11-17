//! Cloudflare Worker entry (wasm32 only). Same routes and JSON as the old src/index.js.
use crate::{agent, page::PAGE};
use serde_json::json;
use wasm_bindgen::JsCast;
use worker::*;

async fn rate_ok(env: &Env, ip: &str) -> bool {
    let rl = match js_sys::Reflect::get(env.as_ref(), &"RATE_LIMITER".into()) {
        Ok(v) if !v.is_undefined() && !v.is_null() => v,
        _ => return true,
    };
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"key".into(), &ip.into());
    let f: js_sys::Function = match js_sys::Reflect::get(&rl, &"limit".into()).and_then(|f| f.dyn_into().map_err(|e| e)) {
        Ok(f) => f,
        Err(_) => return true,
    };
    let p = match f.call1(&rl, &opts) {
        Ok(p) => p,
        Err(_) => return true,
    };
    match wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(p)).await {
        Ok(r) => js_sys::Reflect::get(&r, &"success".into()).ok().and_then(|v| v.as_bool()).unwrap_or(true),
        Err(_) => true,
    }
}

#[event(fetch)]
async fn fetch(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let url = req.url()?;
    if req.method() == Method::Post && url.path() == "/api/ask" {
        let ip = req.headers().get("cf-connecting-ip")?.unwrap_or_else(|| "anon".into());
        if !rate_ok(&env, &ip).await {
            return Ok(Response::from_json(&json!({"error":"Rate limit: try again in a minute."}))?.with_status(429));
        }
        let body: serde_json::Value = req.json().await.unwrap_or(json!({}));
        let q = body.get("question").and_then(|v| v.as_str()).unwrap_or("");
        if q.is_empty() || q.chars().count() > 500 {
            return Ok(Response::from_json(&json!({"error":"question required (<=500 chars)"}))?.with_status(400));
        }
        let ai = env.ai("AI")?;
        let llm = |messages: Vec<serde_json::Value>| {
            let ai = &ai;
            async move {
                let r: serde_json::Value = ai
                    .run(agent::MODEL, json!({"messages": messages, "max_tokens": 256}))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok::<String, String>(match r.get("response") {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                    None => r.to_string(),
                })
            }
        };
        return match agent::run_agent(q, llm, 4).await {
            Ok(v) => Response::from_json(&v),
            Err(e) => Ok(Response::from_json(&json!({"error": format!("Agent error: {e}")}))?.with_status(502)),
        };
    }
    let h = Headers::new();
    h.set("content-type", "text/html;charset=utf-8")?;
    Ok(Response::ok(PAGE)?.with_headers(h))
}

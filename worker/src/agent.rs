//! Minimal ReAct-style single agent. Pure logic: no Cloudflare types, so it tests natively.
use regex::Regex;
use serde_json::{json, Value};
use std::future::Future;

pub const MODEL: &str = "@cf/meta/llama-3.3-70b-instruct-fp8-fast";

pub const SYSTEM: &str = "You are a helpful single agent. Tools: calculator(expr), now().\nTo use a tool reply ONLY: ACTION: <tool> | <input>   e.g. ACTION: calculator | 987654 / 6\nAfter an OBSERVATION, reply with ANSWER unless another tool call is truly needed.\nOtherwise reply: ANSWER: <final answer>";

#[derive(Debug, PartialEq)]
pub enum Step {
    Action { tool: String, input: String },
    Answer(String),
}

/// Format like JS String(+v.toPrecision(12)).
pub fn fmt_js(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let v: f64 = format!("{:.11e}", v).parse().unwrap_or(v);
    if v == 0.0 {
        return "0".into();
    }
    let a = v.abs();
    if a != 0.0 && (a < 1e-6 || a >= 1e21) {
        let s = format!("{:e}", v);
        return if a >= 1e21 { s.replace('e', "e+") } else { s };
    }
    format!("{}", v)
}

pub fn calc(expr: &str) -> Result<f64, String> {
    let stripped: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let re = Regex::new(r"\d+\.?\d*|[-+*/%()]").unwrap();
    let t: Vec<&str> = re.find_iter(&stripped).map(|m| m.as_str()).collect();
    if t.concat() != stripped {
        return Err("invalid expression".into());
    }
    let mut p = Parser { t, i: 0 };
    let v = p.add()?;
    if p.i != p.t.len() {
        return Err("invalid expression".into());
    }
    Ok(v)
}

struct Parser<'a> {
    t: Vec<&'a str>,
    i: usize,
}
const ERR: &str = "invalid expression";
impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.t.get(self.i).copied()
    }
    fn num(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some("-") => {
                self.i += 1;
                Ok(-self.num()?)
            }
            Some("(") => {
                self.i += 1;
                let v = self.add()?;
                if self.peek() != Some(")") {
                    return Err(ERR.into());
                }
                self.i += 1;
                Ok(v)
            }
            Some(tok) => {
                self.i += 1;
                tok.parse::<f64>().map_err(|_| ERR.to_string())
            }
            None => Err(ERR.into()),
        }
    }
    fn mul(&mut self) -> Result<f64, String> {
        let mut v = self.num()?;
        while let Some(o @ ("*" | "/" | "%")) = self.peek() {
            self.i += 1;
            let r = self.num()?;
            v = match o {
                "*" => v * r,
                "/" => v / r, // IEEE division matches JS (Infinity / NaN)
                _ => v % r,   // Rust % on f64 is fmod, same as JS
            };
        }
        Ok(v)
    }
    fn add(&mut self) -> Result<f64, String> {
        let mut v = self.mul()?;
        while let Some(o @ ("+" | "-")) = self.peek() {
            self.i += 1;
            let r = self.mul()?;
            v = if o == "+" { v + r } else { v - r };
        }
        Ok(v)
    }
}

pub fn tool(name: &str, input: &str) -> Option<Result<String, String>> {
    match name {
        "calculator" => Some(calc(input).map(fmt_js)),
        "now" => Some(Ok(now_iso())),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
fn now_iso() -> String {
    String::from(js_sys::Date::new_0().to_iso_string())
}
#[cfg(not(target_arch = "wasm32"))]
fn now_iso() -> String {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let (secs, ms) = (d.as_secs() as i64, d.subsec_millis());
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // civil-from-days (Howard Hinnant)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d_ = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z", y, m, d_, rem / 3600, rem % 3600 / 60, rem % 60, ms)
}

pub fn parse(text: &str) -> Step {
    let a = Regex::new(r"ACTION:\s*(\w+)\s*(?:\([^)]*\))?\s*\|?\s*([^\n]*)").unwrap();
    if let Some(c) = a.captures(text) {
        return Step::Action { tool: c[1].to_string(), input: c[2].trim().to_string() };
    }
    let f = Regex::new(r"(?s)ANSWER:\s*(.*)").unwrap();
    let body = f.captures(text).map(|c| c.get(1).unwrap().as_str()).unwrap_or(text);
    Step::Answer(body.trim().to_string())
}

/// llm(messages) -> reply text. Injected so tests run without Cloudflare.
pub async fn run_agent<F, Fut>(question: &str, mut llm: F, max_steps: usize) -> Result<Value, String>
where
    F: FnMut(Vec<Value>) -> Fut,
    Fut: Future<Output = Result<String, String>>,
{
    let mut messages = vec![json!({"role":"system","content":SYSTEM}), json!({"role":"user","content":question})];
    let mut trace: Vec<Value> = vec![];
    for _ in 0..max_steps {
        let out = llm(messages.clone()).await?;
        match parse(&out) {
            Step::Answer(text) => {
                trace.push(json!({ "answer": text }));
                return Ok(json!({ "answer": text, "trace": trace }));
            }
            Step::Action { tool: name, input } => {
                let obs = match tool(&name, &input) {
                    Some(Ok(s)) => s,
                    Some(Err(e)) => format!("error: {e}"),
                    None => format!("unknown tool {name}"),
                };
                trace.push(json!({"tool": name, "input": input, "observation": obs}));
                messages.push(json!({"role":"assistant","content":out}));
                messages.push(json!({"role":"user","content":format!("OBSERVATION: {obs}")}));
            }
        }
    }
    Ok(json!({"answer":"Step limit reached.","trace":trace}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    fn scripted(replies: Vec<&'static str>) -> impl FnMut(Vec<Value>) -> std::future::Ready<Result<String, String>> {
        let mut r = replies.into_iter();
        move |_| std::future::ready(Ok(r.next().unwrap().to_string()))
    }

    #[test]
    fn parse_action_answer() {
        assert_eq!(parse("ACTION: calculator | 2+2"), Step::Action { tool: "calculator".into(), input: "2+2".into() });
        assert_eq!(parse("ANSWER: hi"), Step::Answer("hi".into()));
    }
    #[test]
    fn calculator_rejects_code() {
        assert!(tool("calculator", "process.exit()").unwrap().is_err());
    }
    #[test]
    fn loop_uses_tool_then_answers() {
        let r = block_on(run_agent("17*23+4?", scripted(vec!["ACTION: calculator | 17*23+4", "ANSWER: 395"]), 4)).unwrap();
        assert_eq!(r["answer"], "395");
        assert_eq!(r["trace"][0]["observation"], "395");
    }
    #[test]
    fn step_limit() {
        let r = block_on(run_agent("x", scripted(vec!["ACTION: now |", "ACTION: now |"]), 2)).unwrap();
        assert_eq!(r["answer"], "Step limit reached.");
    }
    #[test]
    fn precedence_division_modulo() {
        assert_eq!(calc("2+3*(4-1)"), Ok(11.0));
        assert_eq!(calc("-2*3"), Ok(-6.0));
        assert_eq!(calc("10/4"), Ok(2.5));
        assert_eq!(calc("17%5"), Ok(2.0));
        assert_eq!(calc("((2+3)*(4+1))"), Ok(25.0));
    }
    #[test]
    fn rejects_bad() {
        for bad in ["2+a", "(2+3", "2+3)", "2+", "Math.PI"] {
            assert!(calc(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn multiline_answer_fallback_and_no_input() {
        assert_eq!(parse("ANSWER: line1\nline2"), Step::Answer("line1\nline2".into()));
        assert_eq!(parse("just text"), Step::Answer("just text".into()));
        assert_eq!(parse("ACTION: now |"), Step::Action { tool: "now".into(), input: "".into() });
    }
    #[test]
    fn now_iso_shape() {
        let s = tool("now", "").unwrap().unwrap();
        assert!(Regex::new(r"^\d{4}-\d\d-\d\dT").unwrap().is_match(&s));
    }
    #[test]
    fn unknown_tool_and_tool_error_reported() {
        let r = block_on(run_agent("q", scripted(vec!["ACTION: search | cats", "ANSWER: ok"]), 4)).unwrap();
        assert_eq!(r["trace"][0]["observation"], "unknown tool search");
        let r = block_on(run_agent("q", scripted(vec!["ACTION: calculator | 2+", "ANSWER: sorry"]), 4)).unwrap();
        assert!(r["trace"][0]["observation"].as_str().unwrap().starts_with("error: invalid expression"));
    }
    #[test]
    fn direct_answer_no_tools() {
        let r = block_on(run_agent("hi", scripted(vec!["ANSWER: Paris"]), 4)).unwrap();
        assert_eq!(r["answer"], "Paris");
        assert_eq!(r["trace"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn float_noise_and_exponent_format() {
        assert_eq!(tool("calculator", "0.1+0.2").unwrap().unwrap(), "0.3");
        assert_eq!(tool("calculator", "1/123456").unwrap().unwrap(), "0.00000810005184033");
        assert_eq!(tool("calculator", "0.1*0.2/(123456+0.1)").unwrap().unwrap(), "1.62000905585e-7");
        assert_eq!(tool("calculator", "1/0").unwrap().unwrap(), "Infinity");
    }
    #[test]
    fn signature_style_tool_name() {
        assert_eq!(parse("ACTION: calculator(expr) | 987654 / 6"), Step::Action { tool: "calculator".into(), input: "987654 / 6".into() });
        assert_eq!(parse("ACTION: calculator | 2+2\nANSWER: 4"), Step::Action { tool: "calculator".into(), input: "2+2".into() });
    }
}

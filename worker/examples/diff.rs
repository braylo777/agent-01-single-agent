// Differential check helper: prints calculator outputs for a JSON array of expressions.
use agent_01_single_agent::agent::tool;
fn main() {
    let p = std::env::args().nth(1).expect("path to JSON array");
    let ex: Vec<String> = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let out: Vec<String> = ex.iter().map(|e| match tool("calculator", e).unwrap() { Ok(s) => s, Err(m) => format!("error: {m}") }).collect();
    println!("{}", serde_json::to_string(&out).unwrap());
}

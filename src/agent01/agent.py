"""Minimal ReAct-style single agent: think -> (optional tool) -> answer."""
import math
from decimal import Decimal
import re
from datetime import datetime, timezone

MODEL = "@cf/meta/llama-3.3-70b-instruct-fp8-fast"

SYSTEM = """You are a helpful single agent. Tools: calculator(expr), now().
To use a tool reply ONLY: ACTION: <tool> | <input>   e.g. ACTION: calculator | 987654 / 6
After an OBSERVATION, reply with ANSWER unless another tool call is truly needed.
Otherwise reply: ANSWER: <final answer>"""

_TOKEN = re.compile(r"\d+\.?\d*|[-+*/%()]")


class CalcError(Exception):
    pass


def _fmt(v: float) -> str:
    """Format like JS String(+v.toPrecision(12))."""
    if math.isnan(v):
        return "NaN"
    if math.isinf(v):
        return "Infinity" if v > 0 else "-Infinity"
    v = float(f"{v:.12g}")
    if v == int(v) and abs(v) < 1e21:
        return str(int(v))
    a = abs(v)
    if 1e-6 <= a < 1e21:
        return format(Decimal(repr(v)), "f")
    m, e = repr(v).split("e") if "e" in repr(v) else (f"{v:e}".split("e"))
    return f"{m}e{'+' if int(e) >= 0 else '-'}{abs(int(e))}"


def _js_mod(a: float, b: float) -> float:
    if b == 0 or math.isinf(a) or math.isnan(a) or math.isnan(b):
        return math.nan
    return math.fmod(a, b)


def _js_div(a: float, b: float) -> float:
    if b == 0:
        if a == 0 or math.isnan(a):
            return math.nan
        neg = (a < 0) != (math.copysign(1.0, b) < 0)
        return -math.inf if neg else math.inf
    return a / b


def calc(expr: str) -> float:
    """Safe arithmetic parser (+ - * / % parentheses); no eval."""
    stripped = re.sub(r"\s+", "", expr)
    t = _TOKEN.findall(stripped)
    if "".join(t) != stripped:
        raise CalcError("invalid expression")
    i = 0

    def peek():
        return t[i] if i < len(t) else None

    def num() -> float:
        nonlocal i
        if peek() == "-":
            i += 1
            return -num()
        if peek() == "(":
            i += 1
            v = add()
            if peek() != ")":
                raise CalcError("invalid expression")
            i += 1
            return v
        tok = peek()
        i += 1
        try:
            return float(tok)
        except (TypeError, ValueError):
            raise CalcError("invalid expression")

    def mul() -> float:
        nonlocal i
        v = num()
        while peek() in ("*", "/", "%"):
            o = t[i]
            i += 1
            r = num()
            v = v * r if o == "*" else _js_div(v, r) if o == "/" else _js_mod(v, r)
        return v

    def add() -> float:
        nonlocal i
        v = mul()
        while peek() in ("+", "-"):
            o = t[i]
            i += 1
            r = mul()
            v = v + r if o == "+" else v - r
        return v

    v = add()
    if i != len(t):
        raise CalcError("invalid expression")
    return v


def _now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


TOOLS = {
    "calculator": lambda expr: _fmt(calc(expr)),
    "now": lambda _=None: _now(),
}

_ACTION = re.compile(r"ACTION:\s*(\w+)\s*(?:\([^)]*\))?\s*\|?\s*([^\n]*)")
_ANSWER = re.compile(r"ANSWER:\s*(.*)", re.S)


def parse(text: str) -> dict:
    a = _ACTION.search(text)
    if a:
        return {"type": "action", "tool": a.group(1), "input": a.group(2).strip()}
    f = _ANSWER.search(text)
    return {"type": "answer", "text": (f.group(1) if f else text).strip()}


def run_agent(question: str, llm, max_steps: int = 4) -> dict:
    """llm(messages) -> str is injected so tests run offline."""
    messages = [{"role": "system", "content": SYSTEM}, {"role": "user", "content": question}]
    trace = []
    for _ in range(max_steps):
        out = llm(messages)
        step = parse(out)
        if step["type"] == "answer":
            trace.append({"answer": step["text"]})
            return {"answer": step["text"], "trace": trace}
        try:
            fn = TOOLS.get(step["tool"])
            obs = fn(step["input"]) if fn else f"unknown tool {step['tool']}"
        except CalcError as e:
            obs = f"error: {e}"
        trace.append({"tool": step["tool"], "input": step["input"], "observation": obs})
        messages.append({"role": "assistant", "content": out})
        messages.append({"role": "user", "content": f"OBSERVATION: {obs}"})
    return {"answer": "Step limit reached.", "trace": trace}

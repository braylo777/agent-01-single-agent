import re
import pytest
from agent01.agent import run_agent, parse, TOOLS, calc


def test_calc_division_modulo_decimals_nesting():
    assert calc("10/4") == 2.5 and calc("17%5") == 2
    assert calc("1.5*4") == 6 and calc("((2+3)*(4+1))") == 25


@pytest.mark.parametrize("bad", ["2+a", "(2+3", "2+3)", "2+", "Math.PI"])
def test_calc_rejects_bad(bad):
    with pytest.raises(Exception):
        calc(bad)


def test_parse_multiline_answer_and_fallback():
    assert parse("ANSWER: line1\nline2")["text"] == "line1\nline2"
    assert parse("just text") == {"type": "answer", "text": "just text"}


def test_parse_action_without_input():
    assert parse("ACTION: now |") == {"type": "action", "tool": "now", "input": ""}


def test_now_tool_returns_iso():
    assert re.match(r"^\d{4}-\d\d-\d\dT", TOOLS["now"]())


def test_unknown_tool_reported_back():
    replies = ["ACTION: search | cats", "ANSWER: ok"]
    seen = []
    def llm(m):
        seen.append(m[-1]["content"])
        return replies.pop(0)
    r = run_agent("q", llm)
    assert r["trace"][0]["observation"] == "unknown tool search"
    assert "unknown tool search" in seen[-1] and r["answer"] == "ok"


def test_tool_error_caught():
    replies = ["ACTION: calculator | 2+", "ANSWER: sorry"]
    r = run_agent("q", lambda m: replies.pop(0))
    assert r["trace"][0]["observation"].startswith("error: invalid expression")


def test_direct_answer_no_tools():
    r = run_agent("hi", lambda m: "ANSWER: Paris")
    assert r["answer"] == "Paris" and len(r["trace"]) == 1


def test_calculator_trims_float_noise():
    assert TOOLS["calculator"]("0.1+0.2") == "0.3"


def test_parse_tolerates_signature_style_tool_name():
    assert parse("ACTION: calculator(expr) | 987654 / 6") == {"type": "action", "tool": "calculator", "input": "987654 / 6"}
    assert parse("ACTION: calculator | 2+2\nANSWER: 4")["input"] == "2+2"

import re
import pytest
from agent01.agent import run_agent, parse, TOOLS, calc


def test_parse_action_answer():
    assert parse("ACTION: calculator | 2+2") == {"type": "action", "tool": "calculator", "input": "2+2"}
    assert parse("ANSWER: hi")["text"] == "hi"


def test_calculator_rejects_code():
    with pytest.raises(Exception):
        TOOLS["calculator"]("process.exit()")


def test_agent_loop_uses_tool_then_answers():
    replies = ["ACTION: calculator | 17*23+4", "ANSWER: 395"]

    def llm(m):
        if len(m) > 2:
            assert re.search("395", m[-1]["content"])
        return replies.pop(0)

    r = run_agent("17*23+4?", llm)
    assert r["answer"] == "395" and r["trace"][0]["observation"] == "395"


def test_step_limit():
    r = run_agent("x", lambda m: "ACTION: now |", 2)
    assert r["answer"] == "Step limit reached."


def test_calc_precedence():
    assert calc("2+3*(4-1)") == 11
    assert calc("-2*3") == -6

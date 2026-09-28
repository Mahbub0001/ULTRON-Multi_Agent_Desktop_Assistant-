# Bob (Resident Software Developer) Professional Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform Agent Town's Bob into a fully capable, professional software developer agent equipped with project-root pathing, code searching (grep), line-range reading, surgical code editing with AST syntax validation, command execution in workspace, and strict verification before completion.

**Architecture:** Enhance `core/agent_town.py` with `resolve_agent_path`, `search_code`, line-ranged `read_file`, surgical `edit_file` with Python AST validation, and `run_command(cwd)`. Upgrade the ReAct engine (`dispatch_task`) to 10 iterations, prevent false positive "Task completed" reports on step limits, and update Bob's system instructions in `config/agents.json`.

**Tech Stack:** Python 3.10+, `ast`, `pathlib`, `re`, `subprocess`, `unittest`.

## Global Constraints
- Preserve backward compatibility for all other resident agents (Alice, Carol, Dave).
- Existing unit tests (54 tests) must continue to pass 100%.
- File edits must strictly preserve unchanged comments and docstrings.
- Python code edits via `edit_file` must validate syntax via `ast.parse` to prevent file corruption.

---

### Task 1: Path Resolution Engine (`resolve_agent_path`)

**Files:**
- Modify: `core/agent_town.py:155-171`
- Test: `tests/test_agent_town_developer.py`

**Interfaces:**
- Produces: `resolve_agent_path(raw_path: str, must_exist: bool = False) -> Path`

- [ ] **Step 1: Write the failing test for `resolve_agent_path`**

```python
import unittest
from pathlib import Path
from core.agent_town import resolve_agent_path

class TestAgentTownDeveloper(unittest.TestCase):
    def test_resolve_project_relative_path(self):
        p = resolve_agent_path("ui.py")
        self.assertEqual(p.resolve(), (Path.cwd() / "ui.py").resolve())

    def test_resolve_desktop_path(self):
        p = resolve_agent_path("Desktop/my_notes.txt")
        self.assertEqual(p.resolve(), (Path.home() / "Desktop" / "my_notes.txt").resolve())

    def test_resolve_absolute_path(self):
        target = Path.cwd() / "main.py"
        p = resolve_agent_path(str(target))
        self.assertEqual(p.resolve(), target.resolve())
```

- [ ] **Step 2: Run test to verify it fails**
`python -m unittest tests/test_agent_town_developer.py -v`

- [ ] **Step 3: Implement `resolve_agent_path` in `core/agent_town.py`**
Replace `_clean_agent_path` with `resolve_agent_path`:
1. If empty, return `Path.home() / "Desktop" / "JarvisProjects"`.
2. Expand user and normalize separators.
3. If absolute, return resolved path.
4. If starts with `desktop` (case-insensitive), resolve against `Path.home() / "Desktop"`.
5. Check if `Path.cwd() / p` exists. If so, return `Path.cwd() / p`.
6. Fallback to `Path.cwd() / p` for relative paths in current project.

- [ ] **Step 4: Run test to verify it passes**
`python -m unittest tests/test_agent_town_developer.py -v`

- [ ] **Step 5: Commit changes**
`git add core/agent_town.py tests/test_agent_town_developer.py && git commit -m "feat(agent_town): add intelligent resolve_agent_path"`

---

### Task 2: Developer Tools (`search_code`, `read_file`, `edit_file`, `run_command`)

**Files:**
- Modify: `core/agent_town.py:270-386`
- Test: `tests/test_agent_town_developer.py`

**Interfaces:**
- Produces:
  - `search_code(query: str, path: str = ".", file_pattern: str = "") -> str`
  - `read_file(path: str, start_line: int = 1, end_line: int = -1) -> str`
  - `edit_file(path: str, target: str, replacement: str) -> str`
  - `run_command(command: str, cwd: str = ".") -> str`

- [ ] **Step 1: Write failing tests for developer tools**
Add test methods to `tests/test_agent_town_developer.py`:
- `test_search_code_finds_occurrence`
- `test_read_file_line_range`
- `test_edit_file_surgical_replacement`
- `test_edit_file_rejects_ambiguity`
- `test_edit_file_rejects_syntax_error`
- `test_run_command_cwd`

- [ ] **Step 2: Run test to verify it fails**
`python -m unittest tests/test_agent_town_developer.py -v`

- [ ] **Step 3: Implement tools in `AgentTownManager.execute_agent_tool`**
Implement `search_code`, enhance `read_file` with line numbers and slice ranges, implement `edit_file` with `ast.parse` validation for `.py` files, and update `run_command` to accept and resolve `cwd`.

- [ ] **Step 4: Run tests to verify all developer tool tests pass**
`python -m unittest tests/test_agent_town_developer.py -v`

- [ ] **Step 5: Commit changes**
`git add core/agent_town.py tests/test_agent_town_developer.py && git commit -m "feat(agent_town): implement search_code, line-ranged read_file, and surgical edit_file"`

---

### Task 3: ReAct Engine Upgrades & Bob's System Persona

**Files:**
- Modify: `core/agent_town.py:388-545`
- Modify: `config/agents.json:12-20`
- Test: `tests/test_agent_town_developer.py`

**Interfaces:**
- Produces: Updated ReAct prompt with new tool definitions, `max_iterations = 10`, and truthful status reporting on loop exhaustion.

- [ ] **Step 1: Write test for ReAct loop integrity**
Add test in `tests/test_agent_town_developer.py` verifying that step exhaustion does not report `AgentState.COMPLETED`.

- [ ] **Step 2: Run test to verify failure**
`python -m unittest tests/test_agent_town_developer.py -v`

- [ ] **Step 3: Implement ReAct engine improvements & update `config/agents.json`**
1. In `core/agent_town.py`, increase `max_iterations = 10` for developer tasks.
2. Update tool definitions prompt string with `search_code`, `edit_file`, and line-ranged `read_file`.
3. If loop ends without `final_answer`, set `agent.state = AgentState.ERROR` or `INCOMPLETE`, with message `"Task stopped: step limit reached before resolution."`
4. In `config/agents.json`, update Bob's `system_instruction` with professional developer standards (investigate first, surgical edits, verify with tests, report evidence).

- [ ] **Step 4: Run full test suite to verify 100% pass rate**
`python -m unittest discover -s tests -p "test_*.py" -v`

- [ ] **Step 5: Commit changes**
`git add core/agent_town.py config/agents.json tests/test_agent_town_developer.py && git commit -m "feat(agent_town): upgrade ReAct engine and Bob developer persona"`

---

### Task 4: End-to-End Verification & Git Push

**Files:**
- Verify: Full test suite & git tree

- [ ] **Step 1: Run complete test suite**
`python -m unittest discover -s tests -p "test_*.py" -v`

- [ ] **Step 2: Push to GitHub**
`git push origin main`

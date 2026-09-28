# Design Specification: Bob (Resident Software Developer) Professional Upgrade

- **Date:** 2026-09-28
- **Author:** Antigravity (Google DeepMind) & User (Nibir)
- **Status:** Approved / Specifying
- **Target Subsystem:** `core/agent_town.py`, `config/agents.json`, `tests/`

---

## 1. Problem Statement & Background

In Mark-LIV's Agent Town living office, **Bob** is designated as the *Software Architect & Lead Developer* agent. However, when assigned real-world codebase tasks (such as updating configuration or modifying UI display labels across `ui.py` or `config/api_keys.json`), Bob consistently failed to apply changes and prematurely reported "Task completed with observations."

Investigation of [core/agent_town.py](file:///e:/github-projects/Mark-LIV/core/agent_town.py) and [memory/agent_town_memory.json](file:///e:/github-projects/Mark-LIV/memory/agent_town_memory.json) revealed five critical architectural bottlenecks:
1. **Sandboxed Path Resolution:** `_clean_agent_path()` forcibly redirected all relative file paths (e.g. `ui.py`) to `Desktop/JarvisProjects/ui.py`, causing `File does not exist` errors for actual project files.
2. **Silent File Truncation:** `read_file()` hard-truncated output at 15,000 characters without line-slicing capability. In large files like `ui.py` (6,532 lines, ~277 KB), lines beyond line 350 were completely invisible to Bob.
3. **Absence of Surgical Editing Tools:** Bob only had `write_file()` (which overwrites entire files). Rewriting 6,500 lines was impossible, and no `edit_file` / `replace` tool existed.
4. **No Code Grep / Search:** Bob had no search tool (`grep`/`search_code`) to locate where variables or strings were defined.
5. **Short ReAct Loop & False Positives:** `max_iterations = 4` prematurely aborted tasks during investigation, and the engine automatically defaulted uncompleted tasks to `agent.state = AgentState.COMPLETED` with message `"Task completed with observations."`

---

## 2. Goals & Success Criteria

### Goals:
- Equip Bob with professional developer capabilities: project-root awareness, code searching, line-range reading, surgical code editing with AST syntax validation, command execution in workspace, and test-driven verification.
- Eliminate false completion reports when tasks are interrupted or unfinished.
- Increase autonomous execution depth to support multi-step developer workflows (Search ➔ Read ➔ Edit ➔ Test ➔ Verify).

### Success Criteria:
- **Path Resolution:** Bob can reference both project files (e.g. `ui.py`, `config/api_keys.json`) and desktop files correctly without path errors.
- **Code Search:** Bob can run `search_code()` across the workspace and retrieve file paths, line numbers, and matching lines.
- **Line-Ranged Read:** Bob can read arbitrary line ranges (e.g. `start_line=3900, end_line=3950`) of any file with 1-indexed line numbers.
- **Surgical Code Edit:** Bob can use `edit_file()` to replace targeted blocks with exact uniqueness matching and automatic Python AST syntax verification.
- **Execution & Test Verification:** Bob can run tests via `run_command(command, cwd)` in the project workspace.
- **ReAct Loop & Integrity:** Loop iterations expanded to 10 for coding tasks; incomplete loops are marked `INCOMPLETE` / `FAILED`, never `COMPLETED`.
- **Test Coverage:** 100% pass rate across existing unit tests plus comprehensive new unit tests for all developer tools in `tests/test_agent_town_developer.py`.

---

## 3. Architectural Design

```
+-----------------------------------------------------------------------------------+
|                           Mark-LIV Agent Town Engine                             |
+-----------------------------------------------------------------------------------+
                                        |
                   +--------------------+--------------------+
                   |                                         |
                   v                                         v
       [Resident Agent: Bob]                      [Agent Town ReAct Loop]
  Role: Software Architect & Dev                   - max_iterations: 10
  System Persona: Verified Dev Engine              - Strict completion check
                   |                               - Structured JSON parser
                   v                                         |
+------------------------------------------------------------+---------------------+
|                              Agent Town Tool Execution                           |
|                                                                                  |
|  - resolve_agent_path(): Project root first, Desktop fallback                    |
|  - search_code(): Recursive text/regex search with line numbers                  |
|  - read_file(): Line-ranged inspection (start_line, end_line)                    |
|  - edit_file(): Surgical block replacement + Python AST syntax validation        |
|  - run_command(): Shell command execution in workspace cwd                       |
|  - write_file(): File creation / scratch script generator                        |
|  - list_files(), web_search(), create_word_document(), delegate_subtask()        |
+----------------------------------------------------------------------------------+
                                        |
                                        v
                 [Workspace Files & Test Verification Engine]
```

### 3.1 Path Resolution (`resolve_agent_path`)
Replace `_clean_agent_path` with a dual-mode resolver:
- If path is absolute: verify and return resolved `Path`.
- If path starts with `desktop/` or `~/desktop`: resolve relative to `Path.home() / "Desktop"`.
- If path is relative:
  1. Check if it exists in the current project root (`Path.cwd()`). If so, resolve to project root.
  2. Check if it exists in `Path.home() / "Desktop" / "JarvisProjects"`.
  3. Default to `Path.cwd() / raw_path` for code tasks, creating parent directories safely when writing.

### 3.2 Professional Developer Tools in `core/agent_town.py`

#### A. `search_code(query: str, path: str = ".", file_pattern: str = "") -> str`
- Searches files recursively using case-insensitive string matching or regex.
- Skips `.git`, `__pycache__`, `.pytest_cache`, `node_modules`, and binary files.
- Returns matches formatted as: `filepath:line_number: code_line`.
- Limits results to 40 matches to prevent token overflow while giving complete locations.

#### B. `read_file(path: str, start_line: int = 1, end_line: int = -1) -> str`
- Resolves target path via `resolve_agent_path`.
- Reads file as lines with line numbers (`line_num: line_content`).
- Supports slicing `[start_line - 1 : end_line]`.
- If no range specified and file > 300 lines, outputs first 300 lines with an explicit note:
  `"File has {total} lines. Showing lines 1-300. Use start_line and end_line to inspect specific ranges."`

#### C. `edit_file(path: str, target: str, replacement: str) -> str`
- Resolves target path.
- Checks that `target` string appears exactly once in the file.
  - If 0 occurrences: returns `"Error: target string not found in file."`
  - If >1 occurrences: returns `"Error: target string appears {count} times. Please include more surrounding context to uniquely identify the block."`
- Replaces `target` with `replacement`.
- If file extension is `.py`, performs syntax validation:
  ```python
  try:
      ast.parse(new_content)
  except SyntaxError as e:
      return f"Error: Replacement introduces Python syntax error at line {e.lineno}: {e.msg}. Edit rejected."
  ```
- Writes updated content to disk and returns success confirmation with line count and diff summary.

#### D. `run_command(command: str, cwd: str = ".") -> str`
- Executes command with working directory `cwd` resolved against project workspace.
- Captures STDOUT and STDERR with timeout (30 seconds default).
- Returns exit code, stdout, and stderr.

### 3.3 Enhanced ReAct Loop & Quality Control (`dispatch_task`)
- **Iteration Limit:** For agents with developer specialty (or all agents), increase `max_iterations` from 4 to 10.
- **Prompt Specification:** Update tool definitions provided to the LLM to include `search_code`, `edit_file`, and line-ranged `read_file`.
- **Completion Integrity:**
  - If loop finishes `max_iterations` without emitting `final_answer`:
    - Mark `agent.state = AgentState.FAILED` (or `INCOMPLETE`).
    - Record status: `"Task aborted: maximum iteration limit (10) reached without complete resolution."`
    - Do NOT falsely mark as `COMPLETED`.
  - When code is modified, the system instruction prompts Bob to run tests (`run_command`) to verify changes before producing `final_answer`.

### 3.4 Bob's System Persona ([config/agents.json](file:///e:/github-projects/Mark-LIV/config/agents.json))
Update Bob's `system_instruction`:
```json
"system_instruction": "You are Bob, the Lead Software Architect & Developer in Mark-LIV Agent Town. You follow rigorous software engineering standards:\n1. INVESTIGATE FIRST: Use 'search_code' and 'read_file' (with start_line/end_line) to find the exact code lines and root causes before touching code.\n2. SURGICAL EDITS: Use 'edit_file' to make targeted replacements. Never rewrite entire files blindly.\n3. VERIFY BEFORE CLAIMING SUCCESS: Always run tests or validation commands via 'run_command' to confirm your changes work.\n4. EVIDENCE-BASED REPORTS: Your final answer must detail the exact files changed, lines edited, and test results."
```

---

## 4. Testing Strategy

1. **Unit Tests (`tests/test_agent_town_developer.py`):**
   - Test `resolve_agent_path` for project root paths, relative paths, and desktop paths.
   - Test `search_code` finds matching symbols across sample project files with correct line numbers.
   - Test `read_file` with `start_line` and `end_line` slicing.
   - Test `edit_file` successfully edits target content.
   - Test `edit_file` rejects ambiguous targets (multiple matches).
   - Test `edit_file` rejects Python syntax errors via AST parse.
   - Test `run_command` executes in specified `cwd` and returns exit code + output.
   - Test `dispatch_task` does not report success on iteration exhaustion.
2. **Regression Testing:**
   - Run full project test suite (`python -m unittest discover -s tests -p "test_*.py" -v`) to ensure existing 54 tests continue passing without regression.

---

## 5. Security & Safety Considerations
- Path traversal outside the workspace or Desktop is constrained.
- AST parsing prevents corrupting Python runtime files with syntax errors.
- Disambiguation enforcement in `edit_file` prevents accidental multi-location overwrites.

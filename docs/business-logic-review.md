# Business logic review — 2026-10-11

## Conclusion

The tested application workflows pass, but this is not evidence that every
desktop application, web application or voice request works correctly.
Ten regression cases cover concrete defects fixed during this review.

## Fixes

- Preserve clarification text when resuming a task and reject unknown task-control actions.
- Journal each background action before dispatch, retaining its verification check.
- Stop mutations when their checkpoint cannot be saved.
- Recheck the original failed step's postcondition before marking it resolved.
- Block foreground observations that can steal focus from an active task.
- Preflight browser form selectors for uniqueness; read back filled values and report partial failures as errors.
- Validate explicit Excel row counts against supplied rows.
- Treat formula-like Excel headers as literal text and reject oversized headers.
- Do not issue a second click after an uncertain UI Automation invoke.
- Do not repeat an uncertain text update through keyboard fallback; read back direct value updates.

## Verification

- `python -m pytest tests -q`: 156 passed, 1 skipped; two dependency deprecation warnings.
- The skipped test is opt-in native Excel automation. It passed separately using
  `ULTRON_TEST_EXCEL_COM=1` and `python -m unittest tests.test_excel_native_integration -v`.
  It owns an isolated Excel process and checks 12 month columns, five data rows,
  deletion, a blank three-row/five-column table and literal formula-like headers.
- `python -m pytest -q` at repository root failed collection in six vision-service
  test modules: `src` was not importable from that invocation and `grpc` was absent.
  The application-suite result above excludes that independently configured service.

## Remaining review risks and validation gaps

- Live microphone transcription, language stability under noisy audio, model
  selection and real provider tool-calling behavior were not exercised in this audit.
- Browser and UI Automation regression tests use mocks; arbitrary app versions,
  layouts, browser profiles and DPI settings still need real scenario testing.
- A window-title check alone is weak evidence for content changes. A planner can
  still select an insufficient postcondition; completion is not a semantic proof.
- Confirmation callbacks run on a separate worker outside the task coordinator's
  action lock. Cancellation and concurrent execution across that boundary need
  further integration work before treating all confirmed workflows as serialized.
- Browser session wrapper timeouts do not currently ensure the underlying
  coroutine has stopped. A delayed operation may outlive the reported timeout.
- A failed native operation with no saved explicit postcondition remains unresolved;
  the runtime deliberately cannot accept an unrelated observation as its repair.

These limits prevent a claim of universal or perfect control.

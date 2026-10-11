# Four-agent workflow review — 2026-10-11

Alice, Bob, Carol and Dave use the same tool-driven execution loop with different
specialty instructions. Their model choice follows the shared `REASONING` ladder.
They are task specialists, not independent desktop-control engines.

This change prevents replacing an agent's active assignment, makes delegation
report when an agent is busy, checks file writes and edits by reading them back,
and prevents an immediate success report after a failed tool. It also carries a
small amount of recent task context, bounds tool observations in subsequent
prompts, and asks agents to answer in the user's language with concise evidence.

The application suite passed with 160 tests and one skipped native Excel test;
the four-agent workflow tests also passed. These are mocked model interactions.
Actual Gemini response quality, API latency, live search, desktop control and
independent verification across arbitrary tasks still depend on external systems
and the assigned task. Tool observations cannot prove every kind of success.

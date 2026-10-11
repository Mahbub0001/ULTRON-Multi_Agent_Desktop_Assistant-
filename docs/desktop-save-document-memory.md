# Desktop saving and document memory — 2026-10-11

Excel `save` now accepts a destination (`path`) and workbook name (`filename`).
`path="Desktop"` uses Windows' actual Known Folder, including OneDrive
redirection. The file tool uses the same location. Excel reports success only
after the workbook's full path and the saved file agree. A new workbook without
a destination returns an error instead of opening an unobserved Save As dialog.

For an uploaded PDF CV, DOCX or text file, `file_processor(action="remember")`
stores the full readable text in ignored local `memory/documents/` files, checks
the stored text and adds an index to long-term memory. `recall_memory` can search
those files by topic, including common Bengali CV questions. A regular
`save_memory` fact now permits up to 16,000 characters and verifies read-back.
Memory writes use atomic replacement.

The application suite passed with 163 tests and one skipped opt-in Excel test.
The isolated native Excel test separately saved to a temporary fake Desktop and
checked the file exists there. Document tests checked text beyond 50,000
characters, Bengali recall and an empty scanned PDF.

Scanned PDFs with no selectable text still require OCR. This workflow reports
that limit rather than claiming the document was remembered. Previously uploaded
documents are not migrated automatically; the user must upload them again to
store their full text with this version.

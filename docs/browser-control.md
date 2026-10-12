# Browser navigation and scrolling

Run the assistant from the updated source and restart any existing assistant process after updating.

Desktop commands such as “YouTube open koro” navigate to the website before checking for an existing app window. A generic Chrome window no longer counts as YouTube being open. Navigation checks the actual address, including the requested path and search query. YouTube search uses the YouTube results URL and encodes special characters in the query.

Scrolling targets the foreground browser, even if a previous automation session exists. Native scrolling moves the pointer into the browser document using its window bounds, compares the central content before and after input, and requires visible movement in the requested direction. It makes at most one additional attempt. A page boundary, blocked popup, changed target or unavailable evidence produces an error instead of a success claim. Connected browser scrolling checks actual document and nested container scroll offsets.

Keep the intended page visible. A popup, unusual feed layout, fullscreen video or content without enough visual detail can prevent verification; the assistant should inspect the screen when the tool reports this. Continuous scrolling stops when the browser loses foreground focus or movement cannot be verified.

## App windows, hover menus and reactions

Desktop app opening matches the executable process rather than another app's window title. Unknown app names are no longer mapped to an unrelated alias by substring. Closing requests normal closure on one identified window handle and verifies that it disappeared. A save prompt is reported as pending; work is never discarded automatically.

Use `browser_control hover` or `computer_control hover` to reveal menus without clicking. Targets must be unique; identify a post by author and visible text when reaction buttons repeat. After hovering, inspect the newly visible menu and select its actual option once. Inspect the current selection first to avoid toggling off an existing reaction. `verify_state` checks the result: connected browsers can read a specific element's `aria-pressed`, `aria-checked`, `aria-expanded`, `value` or `text`; native windows use a fresh visual observation with an explicit expectation. Visual observations can be uncertain and need the configured Gemini key. A visible reaction menu or matching label alone does not prove a reaction was selected.

Mouse targeting rejects changes to the window or target appearance before clicking. Context-qualified targets use visual grounding rather than matching an unrelated short button label. Named input failures stop typing; Bengali and other Unicode text use clipboard input. These controls provide building blocks for app workflows; they cannot guarantee every app layout or account state will work.

Desktop verification includes failed focus, normal closure, save prompts, misleading browser titles, duplicate buttons, changed targets, missing fields and Unicode typing. A local Chrome page with two synthetic posts validates all six reaction choices, scoped hover and selected-state read-back. A Windows Calculator window was opened and closed through Computer Use as an independent OS check. No real LinkedIn post was changed in these checks.

Validation includes regression checks for website routing, wrong addresses, delayed navigation, foreground targeting, static pages and animation, large movement, scroll boundaries, and connected scroll read-back. A local headless Chrome feed also exercises down/up scrolling, visible translation, a page boundary and a nested scroll container. This synthetic feed does not establish that every live LinkedIn layout works.

# Browser navigation and scrolling

Run the assistant from the updated source and restart any existing assistant process after updating.

Desktop commands such as “YouTube open koro” navigate to the website before checking for an existing app window. A generic Chrome window no longer counts as YouTube being open. Navigation checks the actual address, including the requested path and search query. YouTube search uses the YouTube results URL and encodes special characters in the query.

Scrolling targets the foreground browser, even if a previous automation session exists. Native scrolling moves the pointer into the browser document using its window bounds, compares the central content before and after input, and requires visible movement in the requested direction. It makes at most one additional attempt. A page boundary, blocked popup, changed target or unavailable evidence produces an error instead of a success claim. Connected browser scrolling checks actual document and nested container scroll offsets.

Keep the intended page visible. A popup, unusual feed layout, fullscreen video or content without enough visual detail can prevent verification; the assistant should inspect the screen when the tool reports this. Continuous scrolling stops when the browser loses foreground focus or movement cannot be verified.

Validation includes regression checks for website routing, wrong addresses, delayed navigation, foreground targeting, static pages and animation, large movement, scroll boundaries, and connected scroll read-back. A local headless Chrome feed also exercises down/up scrolling, visible translation, a page boundary and a nested scroll container. This synthetic feed does not establish that every live LinkedIn layout works.

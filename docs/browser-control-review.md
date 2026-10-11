# Browser and volume control review — 2026-10-11

The browser regression log exposed separate navigation, tab and verification
problems. `new_tab` previously entered the native URL-launch branch when no
automation session existed. It now sends Ctrl+T only to the foreground browser,
or opens a tab in an existing connected session. It never launches a new Chrome
process for a tab request. Foreground `go_to` uses the current tab and reads its
address back. Short LinkedIn paths such as `/fee` or `/not` are rejected.

Visible site navigation items should use `smart_click` so the current page is
inspected when needed. Native scroll converts requested pixels to a bounded
number of wheel steps. `start_scroll` continues until `stop_scroll` and pauses
when the browser is not foreground.

`close_tabs(domain="linkedin.com")` closes matching tabs in a connected browser
session and preserves unrelated tabs. Native browser tabs are not reliably
enumerable by this process; there the action returns an error and leaves all
tabs and windows intact. Closing a whole browser requires an explicit `scope`
argument, so a site-tab request cannot accidentally close the window through
the browser tool.

`computer_settings(action="volume_get")` now reads the default output endpoint.
The installed pycaw version exposes it through `EndpointVolume`; this was checked
on the current Windows environment. `volume_set` writes a scalar percentage and
checks the read-back before reporting success. A Bluetooth device's independent
on-device volume may differ from the Windows default output level.

The application suite passed with 175 tests and one skipped opt-in Excel test.
Browser tests use mocks; no live LinkedIn session or arbitrary browser profile
was exercised in this change.

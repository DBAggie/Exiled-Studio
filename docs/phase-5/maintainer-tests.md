# Maintainer tests in the application (2026-11-24)

The Phase 5 gate items only a person at the application can check, planned with the maintainer on 2026-11-24: the passive tree's paint and frame rate and peak memory (gate 9; `performance.md`, "Exceptions for the maintainer" 3 and 5), the keyboard and screen reader checks (`performance.md`, "Accessibility"), and a real update (`updates.md`, "Not tested here"). The NSIS size exception was accepted the same day; the tree packs check is deferred.

Fill in the results table at the end and send it back (or paste it into the session); the agent records the numbers in `performance.md` and `updates.md`.

## What to get

1. **The build under test**: the Windows portable zip from PR #168's CI. In the PR's checks, open the latest **CI** run, scroll to **Artifacts**, download `package-Windows`, and unzip it twice (the artifact holds the portable zip). Or with the GitHub CLI: `gh run download <run-id> -n package-Windows`. It carries the calculation speed-up of PR #168.
2. **The installed baseline for the update test**: `Exiled Studio Experimental 0.9.0-1` from the repository's Releases (the setup `.exe`). Install it only after parts A and B, so the portable build and the installed one are not open together.
3. **A large build to load**: `tools/oracle/fixtures/community/forged-frostbearer-spectre-necromancer.xml` from branch `feat/open-items` (`git fetch origin` then `git checkout feat/open-items` and `git pull`, or open the file on GitHub and save it).
4. **Microsoft Edge** (for the WebView2 tools) and, for part B, NVDA (free) or Windows Narrator (built in, Ctrl+Win+Enter).

Game data: the application does not download it on its own, and the start page's **Add game data…** link opens the folder picker for unsigned development data (do not use it). Download `game-data.loombundle` (8 MB, signed, game build 26530219) from the **Exiled Studio 0.9.0** release's assets, then in the application open **About**, section **Game data**, and use **Install game data from a file…**; it should then show "Signed by the project" and "In use".

## A. Passive tree paint, frame rate, and memory (gate 9)

The release application has no DevTools of its own; WebView2 opens its debugging port when an environment variable asks.

1. In PowerShell, in the unzipped portable folder:
   ```powershell
   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
   & ".\Exiled Studio.exe"
   ```
   (If the executable has another name, run the one `.exe` in the folder.)
2. Import the large build (drop the `.xml` on the window, or Open a file).
3. In Edge, open `edge://inspect`, check that **Discover network targets** lists `localhost:9222` (Configure if not), then click **inspect** under the application's page.
4. **First paint:** in DevTools, Performance panel, tick **Screenshots**, click **Record**, then in the application open the **Passive tree** screen, wait until the tree is drawn, click **Stop**. Read the time from the click to the first frame that shows the tree (hover the screenshots strip; or the timing between the click's input event and the next paint). Do it three times; note each.
5. **Pan and zoom:** in DevTools, press Ctrl+Shift+P, run **Show frames per second (FPS) meter**. In the tree, drag to pan for about five seconds, then zoom in and out with the wheel for five seconds. Note the FPS the meter shows while moving (its typical and its lowest value). A Performance recording of the same gives the frames too.
6. **Memory:** keep the build open with the tree shown, then open Task Manager, **Processes** tab, expand **Exiled Studio**. Note the memory of the application's own process and of each `Microsoft Edge WebView2` process under it, and their total. Then use the application for a minute (change screens, open the tree, edit an item) and note the highest total you see.
7. Close the application.

If `edge://inspect` shows no target, say so: the agent then makes a build with DevTools on.

## B. Keyboard and screen reader

On the same portable build, with the large build open:

1. **Keyboard allocation on the tree:** with the mouse untouched, reach the Passive tree screen, move to a node, allocate it, and remove it again, with the keyboard alone. Note each key you use, and anything you cannot reach or do.
2. **Screen reader:** start NVDA or Narrator. Go through the start page, the build's summary, Skills, Equipment, Configuration, and the Passive tree. Note anything read wrongly, read twice, or not read at all (a button or field with no name, a calculation result not announced, focus lost).

## C. The update (after the next release)

1. Install **Exiled Studio Experimental 0.9.0-1** (setup). Start it once, let it fetch game data, close it.
2. **Strict offline:** in Settings, Updates, choose strict offline and press check for updates: it must refuse without connecting. Set it back to automatic.
3. **Wait for the next experimental release** (it needs PR #168 merged into `dev`, `dev` into `experimental`, and the `release` run; the agent can prepare the version change). Then start 0.9.0-1: with automatic checks it should offer the new version. Install it on approval, start it, and check the version in About.
4. Optional, if you have the time: the same with the MSI of 0.9.0-1, and on Linux the AppImage (installs on approval) and the `.deb` (notifies only).

## Results to send back

| Item | Result |
|---|---|
| Workstation | CPU, RAM, Windows version, display resolution and scaling |
| Build under test | the CI run ID or the zip's file name |
| Game data | fetched from the feed, or chosen by hand |
| A4 tree first paint | three times, ms |
| A5 pan | typical and lowest FPS |
| A5 zoom | typical and lowest FPS |
| A6 memory | application process MB; WebView2 processes MB; total; highest total |
| A, anything odd | |
| B1 keyboard on the tree | keys used; what could not be done |
| B2 screen reader | which reader; each problem with the screen it was on |
| C2 strict offline | refused, yes or no |
| C3 update | offered, installed, version after (when the release exists) |
| C4 (optional) | MSI, AppImage, `.deb` |

## Results, 2026-11-24 (parts A and B1)
Run by an agent session on the maintainer's second workstation through the WebView2 debugging port, with synthetic mouse, wheel, and keys. The numbers and what they mean are in `performance.md`, "The maintainer's application test".

| Item | Result |
|---|---|
| Workstation | i7-1255U (10 cores, 12 threads, Iris Xe), 15.7 GB, Windows 11 Home 26H2 build 26300, 1920×1080 at 150% |
| Build under test | the portable package of PR #168's CI |
| Game data | `game-data.loombundle` from the 0.9.0 release, installed from the file; the feed check had failed with `FEED_HTTP` |
| A4 tree first paint | 305, 197, 168 ms (budget 500) |
| A5 pan | 27 typical, 23 lowest (budget 50); fixed in PR #168, to measure again |
| A5 zoom | 52 typical, 42 lowest at 10 ticks a second; 28 and 13 at 20; fixed in PR #168, to measure again |
| A6 memory | application 321 MB; WebView2 295 MB; total 616 MB; highest 650 MB; the budget counts the application's process only (maintainer 2026-11-24): met |
| A, anything odd | the wheel also scrolled the page and the node card stayed up while dragging (both fixed); a drag seemed to move the tree about twice as far as the pointer (to check by hand) |
| B1 keyboard on the tree | works through the List view (Tab, typing, Enter, Ctrl+Z); not on the canvas; six problems, `performance.md`, "Accessibility" |
| B2 screen reader | not done (needs a person) |
| C2, C3, C4 | not done (after the next release) |

**Next run of part A:** on PR #168's next CI build, A5 again (pan and zoom), and the hover: the time from the pointer resting on a node to its path showing, and to its numbers showing. The maintainer checked dragging by hand (2026-11-24): no noticeable delay.

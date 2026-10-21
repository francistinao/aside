# Aside

A teleprompter for macOS and Windows that sits next to your mouse pointer.

When you record a software walkthrough, present a slide presentation in online conferences, or even in the middle of an online interview, the script is usually at the top of the screen and the button you need is somewhere else, and this can actually result into Cognitive and Visual overload where you can't handle switching your focus on different areas of your screen (especially when you're reading a script while presenting something XD as I personally experience this). 

Aside puts the script in a small bubble that follows the pointer, so the line "Now click the settings cog" arrives at the settings cog with you.

### Updates

Aside checks GitHub Releases shortly after launch and then once a day, never while the Prompter runs. Updates download in the background and install when you quit. Turn off automatic checks in Settings > Updates.

## Use

1. On first launch, choose a folder for your scripts. The default is `Documents/Aside`.
2. Write or open a script in the Library window.
3. Set highlight mode, speed, opacity and corner in the bar at the bottom, then click Start.
4. After a 3 second countdown, the bubble follows your pointer and moves through the script.

Changes to speed, opacity and corner apply while the Prompter runs.

### Scripts

Scripts are plain Markdown files in a folder you choose. Folders in Aside are real folders on disk. There is no database.

You can edit scripts in Aside or in any other editor. If a file changes on disk while it is open, Aside reloads it, or asks first if you have unsaved edits. Deleted scripts go to the system Trash.

Aside supports headings (H1 to H3), bold, italic, bulleted and numbered lists, paragraphs and pause markers:

```markdown
# Q3 Roadmap Demo

Open the dashboard and point at the revenue chart. {pause 2s}

Now click the settings cog.
```

`{pause 2s}` holds the Prompter for that many seconds, from 0.5s to 10s. Headings show as small labels above the next line and are not read as lines.

### Shortcuts

These work from any app while the Prompter runs. Change them in Settings > Shortcuts.

| Action | macOS | Windows |
| --- | --- | --- |
| Pause or Resume | ⌃⌥ Space | Ctrl+Alt+Space |
| Previous line | ⌃⌥ ← | Ctrl+Alt+Left |
| Next line | ⌃⌥ → | Ctrl+Alt+Right |
| Faster (+10 wpm) | ⌃⌥ ↑ | Ctrl+Alt+Up |
| Slower (-10 wpm) | ⌃⌥ ↓ | Ctrl+Alt+Down |
| Restart | ⌃⌥ R | Ctrl+Alt+R |
| Show or Hide Prompter | ⌃⌥ H | Ctrl+Alt+H |
| Stop | ⌃⌥ . | Ctrl+Alt+. |
| Peek (hold) | Control | Ctrl |

The bubble never takes a click. Clicks always go to the app under it. To see what the bubble covers, hold the peek key: the bubble fades out and comes back a moment after you let go. The script keeps running while you peek.

The same controls are in the menu bar extra on macOS and the tray icon on Windows.

Some Intel graphics drivers on Windows rotate the screen on Ctrl+Alt+arrow keys. If a shortcut can't be registered, Settings shows which one so you can choose another.

## Screen sharing and recording

Aside asks the operating system to leave the bubble out of screen shares and recordings. How well that works depends on the platform, and Aside shows the status in the bubble's footer and in Settings > Privacy.

| Platform | Status in Aside | What to expect |
| --- | --- | --- |
| Windows 10 (build 19041 and later) and Windows 11 | Hidden | Left out of the standard capture paths that Zoom, Teams, Meet in Chrome or Edge, OBS and the Snipping Tool use. Aside reads the setting back to confirm it took effect. |
| Windows over Remote Desktop | May be visible | Remote Desktop sessions can show the bubble. |
| macOS | May be visible | Usually included when you share or record the entire screen. Left out when you share a single window or tab, or record with OBS Window Capture or Application Capture. |

On macOS, share a single window or tab to keep the bubble out of the capture.

To test your own computer, open Settings > Privacy and click Run Invisibility Check. Aside shows a test pattern in the bubble for under a second, takes a capture of the display and reports whether the pattern appears. It tests one capture path, so a call app may still behave differently. On macOS the check needs Screen Recording permission. Because builds are not yet signed with a Developer ID, macOS may ask for it again after an update. Nothing else in Aside needs it.

Aside is a presenter tool. It uses public OS APIs only and does not try to get around exam, interview or monitoring software.

## Build from source

You need:

- Rust 1.90 or later
- Node.js LTS with npm
- The [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS

```sh
git clone https://github.com/francistinao/aside.git
cd aside
npm install
npm run tauri dev
```

To build an installer for your machine:

```sh
npm run tauri build
```

The output goes to `src-tauri/target/release/bundle/`.

Before you open a pull request, run:

```sh
npm run typecheck
npm run lint
npm run test
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

### Stack

Tauri 2 (Rust) for the core, React 19 and TipTap 3 for the Library window, and plain TypeScript for the Prompter overlay. Plain CSS on top of shared tokens.

### Layout

```
src/              Library window (React)
  editor/         TipTap setup, Markdown round trip, pause node
  library/        sidebar, file tree, search
  controls/       bottom control bar
  settings/       Settings window
overlay/          Prompter bubble (plain TypeScript, no React)
src-tauri/src/
  lib.rs          builder, plugins, windows
  vault.rs        read, write, watch and trash scripts
  session.rs      Prompter state and timing engine
  follow/         pointer polling, spring, placement, peek
  capture/        capture exclusion per OS and the self-test
  shortcuts.rs    global shortcuts
  tray.rs         menu bar extra and tray menu
assets/           fonts, tokens.css, sample script
spikes/           throwaway experiments, one folder each
```

## Contributing

- This personal project of mine is open source. Feel free to contribute.
- One concern per pull request. Small diffs.
- Say in the pull request description what changed, why, and how you tested it.
- Give every new dependency a one-line reason.

## License

MIT. See [LICENSE](LICENSE).

Aside bundles the Inter font on Windows under the SIL Open Font License. See [assets/fonts/OFL.txt](assets/fonts/OFL.txt).

Made with <3 by Francis Tin-ao

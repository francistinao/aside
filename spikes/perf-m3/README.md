# perf-m3

Timing pages for the M3 performance checks, run in a plain
WKWebView so the numbers come from the same engine Tauri uses on macOS.

- `index.html` types 100 characters into a 20,000 word script with the real editor extensions.
- `sidebar.html` renders the sidebar with 1,000 scripts in 20 folders.

Run `./run.sh`. A small window appears at the bottom right for a few seconds.

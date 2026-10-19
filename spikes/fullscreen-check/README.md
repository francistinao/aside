# fullscreen-check

Prints the on-screen window list front to back: order, window layer, alpha, window number,
owning app, title and bounds. The Prompter panel should appear with layer 1000 ahead of the
full screen app's window. `--aside` prints only Aside's windows. Read only: it never touches
the screen or the pointer.

```
clang -fobjc-arc -framework CoreGraphics -framework Foundation windows.m -o windows
./windows | head -20
./windows --aside
```

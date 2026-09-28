# External open (Finder / Open With / `open -a`) smoke test

Regression check for warm external opens doing nothing. Requires macOS;
run after a Release build:

```sh
xcodebuild -project Mac/Pitex.xcodeproj -scheme Pitex -configuration Release -derivedDataPath build build
APP="$PWD/build/Build/Products/Release/Pitex.app"
```

## Setup

```sh
mkdir -p /tmp/pitex-a /tmp/pitex-b /tmp/pitex-c
printf '\\documentclass{article}\n\\begin{document}\nA\n\\end{document}\n' > /tmp/pitex-a/main.tex
printf '# Notes B\n' > /tmp/pitex-b/notes.md
printf '\\documentclass{article}\n\\begin{document}\nC\n\\end{document}\n' > /tmp/pitex-c/main.tex
```

## Cases

1. **Warm .md, different folder.** Launch `open -a "$APP" /tmp/pitex-a/main.tex`,
   wait until the project loads, then `open -a "$APP" /tmp/pitex-b/notes.md`.
   Expect: exactly one additional window opens and loads project B; the
   window for project A is untouched (2 windows total).
2. **Warm .tex, different folder.** With A and B open, run
   `open -a "$APP" /tmp/pitex-c/main.tex`. Expect: exactly one additional
   window opens and loads project C (3 windows total).
3. **Same-project reuse.** With all three windows open, run
   `open -a "$APP" /tmp/pitex-a/main.tex` again. Expect: no new window;
   the existing window for project A comes forward and shows main.tex
   (still 3 windows).
4. **Cold open.** Quit Pitex, then `open -a "$APP" /tmp/pitex-a/main.tex`.
   Expect: the app launches and the file's project loads (session restore
   must not take precedence over the opened file).

Equivalent manual variants that must behave the same: Finder double-click
on a .tex/.md file, and Finder → Open With → Pitex.

Output demo.cast

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "Charm"
Set TypingSpeed 50ms

Type "cd projects/runb-tui && node dist/index.js"
Sleep 1s
Enter

# Show initial Containers view
Sleep 2s

# Tab to Layers view
Type "@Tab"
Sleep 1.5s

# Tab to Overlay view
Type "@Tab"
Sleep 1.5s

# Tab to System view
Type "@Tab"
Sleep 1.5s

# Back to Containers
Type "@Tab"
Sleep 1.5s

# Quit
Type "q"
Sleep 500ms

Screenshot

# Markdown context graph CLI demo (issue #56)
#
# Record: ./docs/videos/record-markdown-context-cli.sh

Output "docs/videos/markdown-context-cli-no-captions.gif"
Output "docs/videos/markdown-context-cli-no-captions.mp4"

Require rgctl
Require jq

Set Shell "bash"
Set FontSize 15
Set FontFamily "Menlo, Monaco, Consolas, monospace"
Set Width 1400
Set Height 800
Set Padding 14
Set Margin 20
Set MarginFill "#0d1117"
Set WindowBar Colorful
Set WindowBarSize 40
Set BorderRadius 8
Set Theme "Catppuccin Mocha"
Set TypingSpeed 22ms
Set Framerate 30
Set PlaybackSpeed 1.0
Set CursorBlink true

Hide
Type "cd tests/fixtures/markdown-context"
Enter
Sleep 200ms
Type "clear"
Enter
Show

Type "# Docs + code context graph"
Enter
Sleep 800ms

Type "rgctl discover . -l markdown,java"
Enter
Sleep 3s

Type "# GQL: checkout headings"
Enter
Type `rgctl -f json gql "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n LIMIT 5" | jq '.count'`
Enter
Sleep 2s

Type "# Doc → Java class (query 6)"
Enter
Type `rgctl -f json gql "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' RETURN h, f, c" | jq '.count'`
Enter
Sleep 2s

Type "# slice/CPG do not apply to .md (use GQL for docs)"
Enter
Sleep 1s

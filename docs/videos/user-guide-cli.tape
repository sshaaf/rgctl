# rgBuilder CLI demo — user-guide walkthrough (no captions)
#
# Commands follow docs/user-guide.md §16 + CoolStore dual API / hybrid CPG.
# Record: ./docs/videos/record-user-guide-cli.sh
#
# Note: VHS Type strings cannot contain "$" — we cd into the fixture
# instead of using -r REPO. Use backtick Type `...` for mixed quotes.

Output "docs/videos/user-guide-cli-no-captions.gif"
Output "docs/videos/user-guide-cli-no-captions.mp4"

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

# --- silent setup ---
Hide
Type "cd rgbuilder-tests/ecommerce-java"
Enter
Sleep 200ms
Type "clear"
Enter
Show

Sleep 1s

# 1. discover
Type "rgctl discover . -l java -e target --with-cfg --export-migration-hints"
Enter
Sleep 8s

# 2. gql inventory
Ctrl+L
Sleep 300ms
Type "rgctl -f json gql --macro-name all_functions unused | jq '.count'"
Enter
Sleep 2.5s

# 3. gql CALLS
Type `rgctl gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'clearCart' RETURN a,b"`
Enter
Sleep 3s

# 4. named communities (analysis overlay)
Ctrl+L
Sleep 300ms
Type "rgctl -f json gql --macro-name all_communities unused | jq '.rows[:3][][] | {id: .community_id, label, n: .member_count}'"
Enter
Sleep 3s
Type "rgctl communities list | head -10"
Enter
Sleep 3s

# 5. blast-radius (JWT /api cart)
Ctrl+L
Sleep 300ms
Type "rgctl blast-radius 'CartService::clearCart'"
Enter
Sleep 3s

Type "rgctl -f json blast-radius 'CartService::clearCart' | jq '{score: .metrics.score, callers: .topology.direct_callers}'"
Enter
Sleep 3s

# 6. CoolStore dual API + hybrid CPG
Ctrl+L
Sleep 300ms
Type "rgctl cpg status"
Enter
Sleep 2.5s

Type "rgctl cpg mutations --type ShoppingCart --exclude-ctors | head -12"
Enter
Sleep 3s

Type "rgctl blast-radius 'ShoppingCartService::priceShoppingCart'"
Enter
Sleep 3s

# 7. metrics
Ctrl+L
Sleep 300ms
Type "rgctl -f json metrics --pagerank | jq '.pagerank.top[:3]'"
Enter
Sleep 3s

# 8. inspect
Ctrl+L
Sleep 300ms
Type "rgctl -f json inspect checkout cfg | jq '{layer, blocks: (.nodes|length), edges: (.edges|length)}'"
Enter
Sleep 3s

# 9. slice
Type "rgctl slice src/main/java/com/example/ecommerce/service/CartService.java --line 53 --variable item --function addItem"
Enter
Sleep 3s

# 10. semantic
Ctrl+L
Sleep 300ms
Type "rgctl semantic index --embedder vocab --dimensions 256"
Enter
Sleep 4s

Type "rgctl -f json semantic query 'shopping cart checkout' --limit 5 | jq '.hits[:3] | map({name, score})'"
Enter
Sleep 3s

Type "rgctl -f json semantic query 'shopping cart' --scope community --limit 3 | jq '.hits | map({name, ranking, score})'"
Enter
Sleep 3s

# 11. export
Ctrl+L
Sleep 300ms
Type "rgctl export --export-format mermaid --export-output /tmp/clearCart.mmd --query 'name:clearCart'"
Enter
Sleep 2s
Type "head -20 /tmp/clearCart.mmd"
Enter
Sleep 2.5s

# 12. check
Type "rgctl -f json check --policy-file ../../rgbuilder-tests/rgbuilder-policy.json | jq '{schema_version, violations: (.violations|length)}'"
Enter
Sleep 3s

Sleep 2s

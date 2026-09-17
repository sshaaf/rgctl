## Advanced patterns

### HTTP session for many queries

**User intent:** *"I need to run many queries interactively"*

```bash
rgctl -r "$REPO" serve --open
# POST http://127.0.0.1:8080/api/query
# {"query":"MATCH (n:Function) RETURN n LIMIT 5"}
```

See [docs/http-api.md](../../docs/http-api.md). For IDE agents spawn `rgctl -f json` subprocesses; optional `rgctl serve` for repeated HTTP queries on one repo.

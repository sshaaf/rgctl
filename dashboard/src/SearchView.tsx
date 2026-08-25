import { useCallback, useEffect, useState } from "preact/hooks";
import type { DashboardManifest } from "./types";
import {
  fetchSemanticStatus,
  formatSemanticScore,
  hitLabel,
  semanticQuery,
  shortPath,
  type SemanticHit,
  type SemanticStatus,
} from "./semanticSearch";

export interface SearchViewProps {
  manifest: DashboardManifest | null;
}

export function SearchView({ manifest }: SearchViewProps) {
  const [status, setStatus] = useState<SemanticStatus | null>(null);
  const [query, setQuery] = useState("");
  const [fusion, setFusion] = useState(true);
  const [keywordAnd, setKeywordAnd] = useState(false);
  const [limit, setLimit] = useState(20);
  const [hits, setHits] = useState<SemanticHit[]>([]);
  const [lastQuery, setLastQuery] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const manifestSemantic = manifest?.semantic;

  useEffect(() => {
    void fetchSemanticStatus()
      .then(setStatus)
      .catch(() =>
        setStatus({
          available: false,
          message:
            "Could not reach /api/semantic/status — start with `rgctl serve --open` (not python -m http.server).",
        }),
      );
  }, []);

  const runSearch = useCallback(async () => {
    const q = query.trim();
    if (!q) return;
    setLoading(true);
    setError(null);
    try {
      const response = await semanticQuery(q, { limit, fusion, keywordAnd });
      setHits(response.hits);
      setLastQuery(response.query);
    } catch (e) {
      setHits([]);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [query, limit, fusion, keywordAnd]);

  const available = status?.available ?? manifestSemantic?.available ?? false;
  const modelId = status?.model_id ?? manifestSemantic?.model_id;
  const indexed =
    status?.functions_indexed ?? manifestSemantic?.functions_indexed ?? undefined;

  return (
    <div class="search-view h-100 d-flex flex-column p-3">
      <div class="mb-3">
        <h2 class="h5 mb-1">Semantic search</h2>
        <p class="text-muted small mb-0">
          Natural-language and keyword search over indexed functions (default vocab; optional
          code-daemon or hash → Hamming retrieval, optional fusion re-ranking). Build with
          <code>rgctl semantic index</code>.
        </p>
      </div>

      {!available && (
        <div class="alert alert-warning py-2 small" role="status">
          {status?.message ??
            "Semantic index not built yet. Run `rgctl semantic index`, then `rgctl serve --open`."}
        </div>
      )}

      {available && (
        <div class="alert alert-light border py-2 small mb-3" role="status">
          <span class="badge text-bg-success me-2">Ready</span>
          {modelId && <code class="me-2">{modelId}</code>}
          {indexed != null && <span>{indexed.toLocaleString()} functions indexed</span>}
        </div>
      )}

      <form
        class="search-form mb-3"
        onSubmit={(e) => {
          e.preventDefault();
          void runSearch();
        }}
      >
        <div class="input-group mb-2">
          <input
            type="search"
            class="form-control"
            placeholder="e.g. shopping cart checkout, OrderService, security login…"
            value={query}
            onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
            aria-label="Semantic search query"
            disabled={!available || loading}
          />
          <button
            type="submit"
            class="btn btn-primary"
            disabled={!available || loading || !query.trim()}
          >
            {loading ? "Searching…" : "Search"}
          </button>
        </div>

        <div class="d-flex flex-wrap align-items-center gap-3 small">
          <label class="form-check form-check-inline mb-0">
            <input
              type="checkbox"
              class="form-check-input"
              checked={fusion}
              onChange={(e) => setFusion((e.target as HTMLInputElement).checked)}
              disabled={loading}
            />
            <span class="form-check-label">Late fusion</span>
          </label>
          <label class="form-check form-check-inline mb-0">
            <input
              type="checkbox"
              class="form-check-input"
              checked={keywordAnd}
              onChange={(e) => setKeywordAnd((e.target as HTMLInputElement).checked)}
              disabled={loading}
            />
            <span class="form-check-label">Keyword AND</span>
          </label>
          <label class="d-inline-flex align-items-center gap-1 mb-0">
            <span class="text-muted">Limit</span>
            <select
              class="form-select form-select-sm w-auto"
              value={String(limit)}
              onChange={(e) => setLimit(Number((e.target as HTMLSelectElement).value))}
              disabled={loading}
            >
              {[5, 10, 20, 50].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </label>
        </div>
      </form>

      {error && <div class="alert alert-danger py-2 small">{error}</div>}

      {lastQuery && !loading && (
        <p class="small text-muted mb-2">
          {hits.length} result{hits.length === 1 ? "" : "s"} for{" "}
          <strong>{lastQuery}</strong>
          {fusion ? " · fusion ranking" : " · Hamming only"}
          {keywordAnd ? " · keyword AND" : ""}
        </p>
      )}

      <div class="search-results border rounded flex-grow-1 min-h-0 overflow-auto">
        <table class="table table-sm table-hover mb-0">
          <thead class="table-light sticky-top">
            <tr>
              <th scope="col">Function</th>
              <th scope="col">File</th>
              <th scope="col" class="text-end">
                Score
              </th>
              <th scope="col" class="text-end">
                Distance
              </th>
            </tr>
          </thead>
          <tbody>
            {loading && hits.length === 0 ? (
              <tr>
                <td colSpan={4} class="text-muted small">
                  Searching…
                </td>
              </tr>
            ) : hits.length === 0 ? (
              <tr>
                <td colSpan={4} class="text-muted small">
                  {lastQuery ? "No matches." : "Enter a query and press Search."}
                </td>
              </tr>
            ) : (
              hits.map((hit) => (
                <tr key={hit.node_id}>
                  <td class="small fn-name" title={hitLabel(hit)}>
                    {hitLabel(hit)}
                    {hit.ranking === "fusion" && (
                      <span class="badge text-bg-secondary ms-1">fusion</span>
                    )}
                  </td>
                  <td class="small text-muted" title={hit.file_path ?? undefined}>
                    {shortPath(hit.file_path)}
                  </td>
                  <td class="small text-end font-monospace">
                    {formatSemanticScore(hit)}
                  </td>
                  <td class="small text-end text-muted">{hit.distance}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

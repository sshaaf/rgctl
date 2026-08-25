export interface SemanticStatus {
  available: boolean;
  model_id?: string;
  dimensions?: number;
  functions_indexed?: number;
  graph_digest?: string;
  message?: string;
}

export interface SemanticHit {
  node_id: string;
  name: string;
  qualified_name?: string | null;
  file_path?: string | null;
  distance: number;
  score: number;
  fused_score?: number | null;
  ranking?: string | null;
}

export interface SemanticQueryResponse {
  schema_version: number;
  query: string;
  model_id: string;
  dimensions: number;
  hits: SemanticHit[];
}

export interface SemanticQueryOptions {
  limit?: number;
  fusion?: boolean;
  keywordAnd?: boolean;
  candidatePool?: number;
}

export async function fetchSemanticStatus(): Promise<SemanticStatus> {
  const res = await fetch("/api/semantic/status");
  if (!res.ok) {
    return {
      available: false,
      message: `Semantic API unavailable (HTTP ${res.status}). Use \`rgctl serve\`, not a static file server.`,
    };
  }
  return (await res.json()) as SemanticStatus;
}

export async function semanticQuery(
  query: string,
  opts: SemanticQueryOptions = {},
): Promise<SemanticQueryResponse> {
  const res = await fetch("/api/semantic/query", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      query,
      limit: opts.limit ?? 20,
      fusion: opts.fusion ?? true,
      keyword_and: opts.keywordAnd ?? false,
      candidate_pool: opts.candidatePool ?? 256,
    }),
  });
  const body = (await res.json()) as SemanticQueryResponse & { message?: string };
  if (!res.ok) {
    const detail =
      typeof body === "object" && body && "message" in body && body.message
        ? body.message
        : `HTTP ${res.status}`;
    throw new Error(detail);
  }
  return body;
}

export function formatSemanticScore(hit: SemanticHit): string {
  return hit.score.toFixed(3);
}

export function hitLabel(hit: SemanticHit): string {
  return hit.qualified_name?.trim() || hit.name;
}

export function shortPath(filePath?: string | null): string {
  if (!filePath) return "—";
  const parts = filePath.split(/[/\\]/);
  if (parts.length <= 3) return filePath;
  return parts.slice(-3).join("/");
}

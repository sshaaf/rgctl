import { useEffect, useMemo, useState } from "preact/hooks";
import { bundleDataUrl } from "./bundleUrl";
import { ColumnHelp } from "./ColumnHelp";
import { FunctionListLayout } from "./FunctionListSidebar";
import { shortPath } from "./functionListUtils";
import { KantraSnippetEditor } from "./kantraSourceEditor";
import { loadSourceText } from "./sourceResolver";
import type {
  DashboardManifest,
  KantraFileBundle,
  KantraFileEntry,
  KantraIndexPayload,
  KantraRuleEntry,
  KantraTargetEntry,
  KantraViolation,
} from "./types";

const FILE_PAGE_SIZE = 30;
const SNIPPET_CONTEXT = 4;

const KANTRA_FILTER_CATEGORIES = [
  "mandatory",
  "potential",
  "optional",
  "uncategorized",
] as const;

type KantraFilterCategory = (typeof KANTRA_FILTER_CATEGORIES)[number];

const ALL_KANTRA_CATEGORIES: ReadonlySet<KantraFilterCategory> = new Set(
  KANTRA_FILTER_CATEGORIES,
);

const COLUMN_TOOLTIPS = {
  file: "Source file path (relative to repo root).",
  rule: "Kantra rule identifier from the catalog.",
  line: "1-based line number in the file.",
  category: "Rule category from the Konveyor catalog.",
  matched_by: "Evaluator that produced the match.",
  symbol: "Matched import or symbol when resolved by a referenced rule.",
} as const;

function normalizeCategory(category: string | null | undefined): KantraFilterCategory {
  if (
    category === "mandatory" ||
    category === "potential" ||
    category === "optional"
  ) {
    return category;
  }
  return "uncategorized";
}

function categoryBadge(category: string | null | undefined): string {
  switch (normalizeCategory(category)) {
    case "mandatory":
      return "danger";
    case "potential":
      return "warning";
    case "optional":
      return "info";
    default:
      return "secondary";
  }
}

function basename(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

function fileCategoryCount(
  entry: KantraFileEntry,
  categories: ReadonlySet<KantraFilterCategory>,
  targetFilter: string | null,
): number {
  if (targetFilter) {
    const perTarget = entry.target_categories?.[targetFilter];
    if (!perTarget) return 0;
    let count = 0;
    for (const cat of categories) {
      count += perTarget[cat] ?? 0;
    }
    return count;
  }
  let count = 0;
  for (const cat of categories) {
    count += entry.categories[cat] ?? 0;
  }
  return count;
}

function fileMatchesFilters(
  entry: KantraFileEntry,
  categories: ReadonlySet<KantraFilterCategory>,
  targetFilter: string | null,
): boolean {
  if (categories.size === 0) {
    return false;
  }
  return fileCategoryCount(entry, categories, targetFilter) > 0;
}

function violationMatchesTarget(
  violation: KantraViolation,
  rule: KantraRuleEntry | undefined,
  targetFilter: string | null,
): boolean {
  if (!targetFilter) return true;
  return (rule?.targets ?? []).includes(targetFilter);
}

function violationMatchesFilters(
  violation: KantraViolation,
  rule: KantraRuleEntry | undefined,
  categories: ReadonlySet<KantraFilterCategory>,
  targetFilter: string | null,
): boolean {
  return (
    violationMatchesCategory(violation, categories) &&
    violationMatchesTarget(violation, rule, targetFilter)
  );
}

function violationMatchesCategory(
  violation: KantraViolation,
  categories: ReadonlySet<KantraFilterCategory>,
): boolean {
  return categories.has(normalizeCategory(violation.category));
}

function violationKey(v: KantraViolation, index: number): string {
  return `${v.rule_id}:${v.line}:${index}`;
}

function snippetRange(
  source: string,
  line: number,
  context = SNIPPET_CONTEXT,
): { text: string; firstLine: number } | null {
  const lines = source.split("\n");
  if (lines.length === 0) return null;
  const start = Math.max(0, line - 1 - context);
  const end = Math.min(lines.length, line + context);
  return {
    text: lines.slice(start, end).join("\n"),
    firstLine: start + 1,
  };
}

export interface KantraViewProps {
  manifest: DashboardManifest | null;
}

export function KantraView({ manifest }: KantraViewProps) {
  const [index, setIndex] = useState<KantraIndexPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedFileId, setSelectedFileId] = useState<string | null>(null);
  const [fileBundle, setFileBundle] = useState<KantraFileBundle | null>(null);
  const [fileSearch, setFileSearch] = useState("");
  const [filePage, setFilePage] = useState(0);
  const [loadingFile, setLoadingFile] = useState(false);
  const [selectedViolationKey, setSelectedViolationKey] = useState<string | null>(null);
  const [sourceText, setSourceText] = useState("");
  const [loadingSource, setLoadingSource] = useState(false);
  const [categoryFilters, setCategoryFilters] = useState<Set<KantraFilterCategory>>(
    () => new Set(ALL_KANTRA_CATEGORIES),
  );
  const [targetFilter, setTargetFilter] = useState<string | null>(null);
  const [targetSearch, setTargetSearch] = useState("");
  const kantra = manifest?.kantra;

  useEffect(() => {
    if (!kantra?.available) {
      setIndex(null);
      return;
    }
    let cancelled = false;
    fetch(bundleDataUrl(kantra.index_path ?? "kantra_index.json"))
      .then((r) => {
        if (!r.ok) throw new Error(`kantra_index.json HTTP ${r.status}`);
        return r.json() as Promise<KantraIndexPayload>;
      })
      .then((data) => {
        if (!cancelled) setIndex(data);
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [kantra?.available, kantra?.index_path]);

  useEffect(() => {
    if (!selectedFileId || !index?.available) {
      setFileBundle(null);
      setSelectedViolationKey(null);
      setSourceText("");
      return;
    }
    let cancelled = false;
    setLoadingFile(true);
    setSelectedViolationKey(null);
    const detailDir = index.detail_dir || "kantra";
    fetch(bundleDataUrl(`${detailDir}/files/${selectedFileId}.json`))
      .then((r) => {
        if (!r.ok) throw new Error(`kantra file bundle HTTP ${r.status}`);
        return r.json() as Promise<KantraFileBundle>;
      })
      .then((data) => {
        if (!cancelled) setFileBundle(data);
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (!cancelled) setLoadingFile(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedFileId, index?.available, index?.detail_dir]);

  const selectedFile = useMemo(
    () => index?.files.find((f) => f.id === selectedFileId) ?? null,
    [index, selectedFileId],
  );

  const sourceId = fileBundle?.source_id ?? selectedFile?.source_id ?? null;

  useEffect(() => {
    if (!sourceId) {
      setSourceText("");
      return;
    }
    let cancelled = false;
    setLoadingSource(true);
    loadSourceText(sourceId)
      .then((text) => {
        if (!cancelled) setSourceText(text);
      })
      .catch(() => {
        if (!cancelled) setSourceText("");
      })
      .finally(() => {
        if (!cancelled) setLoadingSource(false);
      });
    return () => {
      cancelled = true;
    };
  }, [sourceId]);

  const showTargetFilter = !index?.target_filter && (index?.available_targets?.length ?? 0) > 0;

  const filteredTargets = useMemo(() => {
    const targets = index?.available_targets ?? [];
    const q = targetSearch.trim().toLowerCase();
    if (!q) return targets;
    return targets.filter((t) => t.target.toLowerCase().includes(q));
  }, [index?.available_targets, targetSearch]);

  const rulesById = useMemo(() => {
    const map = new Map<string, KantraRuleEntry>();
    for (const rule of index?.rules ?? []) {
      map.set(rule.rule_id, rule);
    }
    return map;
  }, [index?.rules]);

  const filteredFiles = useMemo(() => {
    if (!index) return [];
    const q = fileSearch.trim().toLowerCase();
    return index.files.filter((f) => {
      if (!fileMatchesFilters(f, categoryFilters, targetFilter)) {
        return false;
      }
      if (!q) {
        return true;
      }
      return (
        f.path.toLowerCase().includes(q) ||
        basename(f.path).toLowerCase().includes(q)
      );
    });
  }, [index, fileSearch, categoryFilters, targetFilter]);

  const filteredViolations = useMemo(() => {
    if (!fileBundle) return [];
    return fileBundle.violations.filter((v) =>
      violationMatchesFilters(v, rulesById.get(v.rule_id), categoryFilters, targetFilter),
    );
  }, [fileBundle, categoryFilters, targetFilter, rulesById]);

  const selectedViolation = useMemo(() => {
    if (!selectedViolationKey) return null;
    return (
      filteredViolations.find((v, i) => violationKey(v, i) === selectedViolationKey) ??
      null
    );
  }, [filteredViolations, selectedViolationKey]);

  const filePageCount = Math.max(1, Math.ceil(filteredFiles.length / FILE_PAGE_SIZE));
  const safeFilePage = Math.min(filePage, filePageCount - 1);
  const pageFiles = filteredFiles.slice(
    safeFilePage * FILE_PAGE_SIZE,
    safeFilePage * FILE_PAGE_SIZE + FILE_PAGE_SIZE,
  );

  useEffect(() => {
    setFilePage(0);
  }, [fileSearch, categoryFilters, targetFilter]);

  useEffect(() => {
    if (selectedFileId && !filteredFiles.some((f) => f.id === selectedFileId)) {
      setSelectedFileId(null);
    }
  }, [filteredFiles, selectedFileId]);

  useEffect(() => {
    if (
      selectedViolationKey &&
      !filteredViolations.some((v, i) => violationKey(v, i) === selectedViolationKey)
    ) {
      setSelectedViolationKey(null);
    }
  }, [filteredViolations, selectedViolationKey]);

  const toggleCategory = (category: KantraFilterCategory) => {
    setCategoryFilters((prev) => {
      const next = new Set(prev);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return next;
    });
  };

  const setAllCategories = (enabled: boolean) => {
    setCategoryFilters(enabled ? new Set(ALL_KANTRA_CATEGORIES) : new Set());
  };

  if (!kantra?.available) {
    return (
      <div class="p-4">
        <p class="text-muted mb-2">
          Konveyor Kantra migration rules are not in this bundle. Run discover with Kantra enabled:
        </p>
        <pre class="small bg-light p-2 rounded mb-0">
          rgctl discover . --with-kantra{"\n"}
          rgctl discover . --with-kantra --kantra-target quarkus
        </pre>
      </div>
    );
  }

  if (error && !index) {
    return <div class="alert alert-danger small mb-0 m-3">{error}</div>;
  }

  if (!index) {
    return <p class="text-muted small mb-0 p-4">Loading migration rules…</p>;
  }

  return (
    <FunctionListLayout
      sidebar={
        <KantraFileSidebar
          files={pageFiles}
          totalCount={filteredFiles.length}
          page={safeFilePage}
          pageCount={filePageCount}
          search={fileSearch}
          onSearchChange={setFileSearch}
          onPrevPage={() => setFilePage((p) => Math.max(0, p - 1))}
          onNextPage={() => setFilePage((p) => Math.min(filePageCount - 1, p + 1))}
          selectedId={selectedFileId}
          categoryFilters={categoryFilters}
          targetFilter={targetFilter}
          onSelect={(id) => {
            setSelectedFileId(id);
            setSelectedViolationKey(null);
          }}
        />
      }
    >
      <div class="kantra-view d-flex flex-column flex-grow-1 min-h-0 p-3 gap-2">
        <div class="border rounded bg-white p-3 flex-shrink-0">
          <div class="d-flex flex-wrap gap-3 align-items-center mb-2">
            <div>
              <div class="fw-semibold">Migration rules</div>
              <div class="text-muted small">
                {index.violation_count.toLocaleString()} violations ·{" "}
                {index.evaluated_rules.toLocaleString()} rules · {index.file_count.toLocaleString()}{" "}
                files
              </div>
            </div>
            {index.catalog_id && (
              <span class="badge text-bg-light border font-monospace">{index.catalog_id}</span>
            )}
            {index.target_filter && (
              <span class="badge text-bg-primary">target: {index.target_filter}</span>
            )}
          </div>
        </div>

        <CategoryFilterBar
          index={index}
          categoryFilters={categoryFilters}
          onToggle={toggleCategory}
          onAll={() => setAllCategories(true)}
          onNone={() => setAllCategories(false)}
        />

        {showTargetFilter && (
          <TargetFilterBar
            targets={filteredTargets}
            totalTargets={(index.available_targets ?? []).length}
            selectedTarget={targetFilter}
            targetSearch={targetSearch}
            onTargetSearchChange={setTargetSearch}
            onSelectTarget={setTargetFilter}
            violationCount={index.violation_count}
          />
        )}

        {error && <div class="alert alert-warning py-2 small mb-0 flex-shrink-0">{error}</div>}
        {loadingFile && (
          <p class="text-muted small mb-0 flex-shrink-0">Loading file violations…</p>
        )}

        {!selectedFileId && (
          <p class="text-muted small mb-0">
            Select a file in the sidebar to review its rule violations.
          </p>
        )}

        {selectedFileId && fileBundle && !loadingFile && (
          <div class="d-flex flex-column flex-grow-1 min-h-0 gap-2">
            <div
              class="border rounded bg-white d-flex flex-column flex-shrink-0"
              style={{ maxHeight: "42%" }}
            >
              <div class="border-bottom py-2 px-3 small flex-shrink-0 d-flex flex-wrap align-items-center gap-2">
                <span class="fw-semibold text-break">{fileBundle.path}</span>
                <span class="text-muted">
                  {filteredViolations.length.toLocaleString()} violation
                  {filteredViolations.length === 1 ? "" : "s"}
                </span>
              </div>
              <div class="flex-grow-1 min-h-0 overflow-auto">
                {filteredViolations.length === 0 ? (
                  <p class="text-muted small px-3 py-2 mb-0">
                    No violations match the selected categories for this file.
                  </p>
                ) : (
                  <table class="table table-sm table-hover mb-0">
                    <thead class="table-light sticky-top">
                      <tr>
                        <th scope="col">
                          Rule
                          <ColumnHelp text={COLUMN_TOOLTIPS.rule} />
                        </th>
                        <th scope="col">
                          Line
                          <ColumnHelp text={COLUMN_TOOLTIPS.line} />
                        </th>
                        <th scope="col">
                          Category
                          <ColumnHelp text={COLUMN_TOOLTIPS.category} />
                        </th>
                        <th scope="col">
                          Matcher
                          <ColumnHelp text={COLUMN_TOOLTIPS.matched_by} />
                        </th>
                        <th scope="col">Symbol</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredViolations.map((v, i) => {
                        const key = violationKey(v, i);
                        const active = key === selectedViolationKey;
                        return (
                          <tr
                            key={key}
                            class={active ? "table-primary" : undefined}
                            style={{ cursor: "pointer" }}
                            onClick={() => setSelectedViolationKey(key)}
                          >
                            <td class="font-monospace small">{v.rule_id}</td>
                            <td>{v.line}</td>
                            <td>
                              <span class={`badge text-bg-${categoryBadge(v.category)}`}>
                                {normalizeCategory(v.category)}
                              </span>
                            </td>
                            <td class="small text-muted">{v.matched_by}</td>
                            <td class="small text-break">
                              {v.symbol ? <code>{v.symbol}</code> : "—"}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                )}
              </div>
            </div>

            {selectedViolation && (
              <ViolationDetailPanel
                violation={selectedViolation}
                rule={rulesById.get(selectedViolation.rule_id) ?? null}
                sourceText={sourceText}
                loadingSource={loadingSource}
                filePath={fileBundle.path}
              />
            )}
          </div>
        )}
      </div>
    </FunctionListLayout>
  );
}

function CategoryFilterBar({
  index,
  categoryFilters,
  onToggle,
  onAll,
  onNone,
}: {
  index: KantraIndexPayload;
  categoryFilters: ReadonlySet<KantraFilterCategory>;
  onToggle: (category: KantraFilterCategory) => void;
  onAll: () => void;
  onNone: () => void;
}) {
  return (
    <div
      class="kantra-category-filters border rounded bg-white px-3 py-2 flex-shrink-0 d-flex flex-wrap align-items-center gap-3"
      data-testid="kantra-category-filters"
    >
      <span class="small fw-semibold">Filter files</span>
      {KANTRA_FILTER_CATEGORIES.map((cat) => (
        <label
          key={cat}
          class="form-check form-check-inline d-flex align-items-center gap-1 mb-0"
        >
          <input
            type="checkbox"
            class="form-check-input mt-0"
            checked={categoryFilters.has(cat)}
            onChange={() => onToggle(cat)}
          />
          <span class={`badge text-bg-${categoryBadge(cat)}`}>
            {cat}: {(index.by_category[cat] ?? 0).toLocaleString()}
          </span>
        </label>
      ))}
      <div class="btn-group btn-group-sm" role="group" aria-label="Category presets">
        <button type="button" class="btn btn-outline-secondary" onClick={onAll}>
          All
        </button>
        <button type="button" class="btn btn-outline-secondary" onClick={onNone}>
          None
        </button>
      </div>
      {categoryFilters.size === 0 && (
        <span class="text-warning small mb-0">Select at least one category.</span>
      )}
    </div>
  );
}

function TargetFilterBar({
  targets,
  totalTargets,
  selectedTarget,
  targetSearch,
  onTargetSearchChange,
  onSelectTarget,
  violationCount,
}: {
  targets: KantraTargetEntry[];
  totalTargets: number;
  selectedTarget: string | null;
  targetSearch: string;
  onTargetSearchChange: (value: string) => void;
  onSelectTarget: (target: string | null) => void;
  violationCount: number;
}) {
  return (
    <div
      class="kantra-target-filters border rounded bg-white px-3 py-2 flex-shrink-0 d-flex flex-wrap align-items-center gap-3"
      data-testid="kantra-target-filters"
    >
      <span class="small fw-semibold">Konveyor target</span>
      <select
        class="form-select form-select-sm"
        style={{ maxWidth: "18rem" }}
        value={selectedTarget ?? ""}
        aria-label="Filter by Konveyor target"
        onChange={(e) => {
          const value = (e.target as HTMLSelectElement).value;
          onSelectTarget(value || null);
        }}
      >
        <option value="">All targets ({violationCount.toLocaleString()})</option>
        {targets.map((entry) => (
          <option key={entry.target} value={entry.target}>
            {entry.target} ({entry.violation_count.toLocaleString()})
          </option>
        ))}
      </select>
      {totalTargets > 12 && (
        <input
          type="search"
          class="form-control form-control-sm"
          style={{ maxWidth: "14rem" }}
          placeholder="Search targets…"
          value={targetSearch}
          aria-label="Search Konveyor targets"
          onInput={(e) => onTargetSearchChange((e.target as HTMLInputElement).value)}
        />
      )}
      {selectedTarget && (
        <button
          type="button"
          class="btn btn-sm btn-outline-secondary"
          onClick={() => onSelectTarget(null)}
        >
          Clear target
        </button>
      )}
      {targetSearch && targets.length === 0 && (
        <span class="text-muted small mb-0">No targets match your search.</span>
      )}
    </div>
  );
}

function KantraFileSidebar({
  files,
  totalCount,
  page,
  pageCount,
  search,
  onSearchChange,
  onPrevPage,
  onNextPage,
  selectedId,
  categoryFilters,
  targetFilter,
  onSelect,
}: {
  files: KantraFileEntry[];
  totalCount: number;
  page: number;
  pageCount: number;
  search: string;
  onSearchChange: (value: string) => void;
  onPrevPage: () => void;
  onNextPage: () => void;
  selectedId: string | null;
  categoryFilters: ReadonlySet<KantraFilterCategory>;
  targetFilter: string | null;
  onSelect: (id: string) => void;
}) {
  const rangeStart = totalCount === 0 ? 0 : page * FILE_PAGE_SIZE + 1;
  const rangeEnd = Math.min((page + 1) * FILE_PAGE_SIZE, totalCount);

  return (
    <aside class="function-list-sidebar border-end bg-white" aria-label="File list">
      <div class="function-list-sidebar-inner">
        <div class="function-list-sidebar-header px-3 py-2 border-bottom d-flex align-items-center gap-2">
          <h2 class="function-list-sidebar-heading mb-0 flex-grow-1">Files</h2>
          <span class="text-muted small">{totalCount.toLocaleString()}</span>
        </div>
        <div class="px-3 py-2 border-bottom flex-shrink-0">
          <input
            type="search"
            class="form-control form-control-sm"
            placeholder="Search files…"
            value={search}
            onInput={(e) => onSearchChange((e.target as HTMLInputElement).value)}
            aria-label="Search files"
          />
        </div>
        <div class="function-list-scroll flex-grow-1 min-h-0 overflow-auto">
          {files.length === 0 ? (
            <p class="text-muted small px-3 py-2 mb-0">No files match the current filters.</p>
          ) : (
            files.map((file) => {
              const active = file.id === selectedId;
              const visibleCount = fileCategoryCount(file, categoryFilters, targetFilter);
              return (
                <button
                  key={file.id}
                  type="button"
                  class={`function-list-item w-100 text-start ${active ? "active" : ""}`}
                  onClick={() => onSelect(file.id)}
                  title={file.path}
                >
                  <div class="function-list-item-name text-truncate">{basename(file.path)}</div>
                  <div class="function-list-item-meta text-truncate">{shortPath(file.path)}</div>
                  <div class="d-flex flex-wrap gap-1 mt-1">
                    {KANTRA_FILTER_CATEGORIES.filter((cat) => {
                      const count = targetFilter
                        ? (file.target_categories?.[targetFilter]?.[cat] ?? 0)
                        : (file.categories[cat] ?? 0);
                      return count > 0;
                    }).map((cat) => (
                      <span
                        key={cat}
                        class={`badge text-bg-${categoryBadge(cat)}`}
                        style={{ fontSize: "0.65rem" }}
                      >
                        {cat}
                      </span>
                    ))}
                  </div>
                  <span class="badge function-list-item-badge bg-secondary">
                    {visibleCount}
                  </span>
                </button>
              );
            })
          )}
        </div>
        {totalCount > 0 && (
          <div class="function-list-pagination px-3 py-2 border-top flex-shrink-0 d-flex align-items-center justify-content-between gap-2">
            <button
              type="button"
              class="btn btn-sm btn-outline-secondary"
              disabled={page <= 0}
              onClick={onPrevPage}
            >
              Prev
            </button>
            <span class="small text-muted text-center flex-grow-1">
              {rangeStart}–{rangeEnd} of {totalCount.toLocaleString()}
              {pageCount > 1 && ` · ${page + 1}/${pageCount}`}
            </span>
            <button
              type="button"
              class="btn btn-sm btn-outline-secondary"
              disabled={page + 1 >= pageCount}
              onClick={onNextPage}
            >
              Next
            </button>
          </div>
        )}
      </div>
    </aside>
  );
}

function ViolationDetailPanel({
  violation,
  rule,
  sourceText,
  loadingSource,
  filePath,
}: {
  violation: KantraViolation;
  rule: KantraRuleEntry | null;
  sourceText: string;
  loadingSource: boolean;
  filePath: string;
}) {
  const snippet =
    sourceText.length > 0 ? snippetRange(sourceText, violation.line) : null;

  return (
    <div class="kantra-violation-detail border rounded bg-white flex-grow-1 min-h-0 d-flex flex-column">
      <div class="border-bottom py-2 px-3 small flex-shrink-0">
        <div class="fw-semibold font-monospace">{violation.rule_id}</div>
        <div class="text-muted">
          Line {violation.line} · {violation.matched_by} ·{" "}
          <span class={`badge text-bg-${categoryBadge(violation.category)}`}>
            {normalizeCategory(violation.category)}
          </span>
        </div>
      </div>
      <div class="px-3 py-2 flex-grow-1 min-h-0 overflow-auto">
        <div class="mb-3">
          <div class="small fw-semibold mb-1">Rule</div>
          <p class="small mb-1">
            {violation.message ?? rule?.message ?? "No rule message in the bundle."}
          </p>
          {violation.symbol && (
            <p class="small mb-0">
              Matched symbol: <code>{violation.symbol}</code>
            </p>
          )}
        </div>
        <div>
          <div class="small fw-semibold mb-1">Code at line {violation.line}</div>
          {loadingSource && <p class="text-muted small mb-0">Loading source…</p>}
          {!loadingSource && !snippet && (
            <p class="text-muted small mb-0">
              Source text is not available for this file in the dashboard bundle.
            </p>
          )}
          {!loadingSource && snippet && (
            <KantraSnippetEditor
              source={snippet.text}
              highlightLine={violation.line}
              firstLine={snippet.firstLine}
              category={violation.category}
              filePath={filePath}
            />
          )}
        </div>
      </div>
    </div>
  );
}

import { useEffect, useRef } from "preact/hooks";
import {
  EditorState,
  Compartment,
  StateField,
  RangeSetBuilder,
} from "@codemirror/state";
import {
  EditorView,
  lineNumbers,
  Decoration,
  type DecorationSet,
} from "@codemirror/view";
import { java } from "@codemirror/lang-java";
import { basicSetup } from "codemirror";

export type KantraViolationCategory =
  | "mandatory"
  | "potential"
  | "optional"
  | "uncategorized";

export interface KantraSnippetEditorProps {
  source: string;
  /** 1-based line number in the full file. */
  highlightLine: number;
  /** 1-based line number of the first line shown in `source`. */
  firstLine: number;
  category: string | null | undefined;
  filePath?: string | null;
}

function normalizeCategory(category: string | null | undefined): KantraViolationCategory {
  if (
    category === "mandatory" ||
    category === "potential" ||
    category === "optional"
  ) {
    return category;
  }
  return "uncategorized";
}

function categoryLineClass(category: KantraViolationCategory): string {
  switch (category) {
    case "mandatory":
      return "cm-kantra-line-mandatory";
    case "potential":
      return "cm-kantra-line-potential";
    case "optional":
      return "cm-kantra-line-optional";
    default:
      return "cm-kantra-line-uncategorized";
  }
}

function languageExtension(filePath?: string | null) {
  if (filePath?.toLowerCase().endsWith(".java")) {
    return java();
  }
  return [];
}

export function KantraSnippetEditor({
  source,
  highlightLine,
  firstLine,
  category,
  filePath,
}: KantraSnippetEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const highlightCompartment = useRef(new Compartment());
  const lineOffset = Math.max(0, firstLine - 1);
  const relativeHighlightLine = highlightLine - lineOffset;
  const normalizedCategory = normalizeCategory(category);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const highlightExt = buildViolationHighlight(
      relativeHighlightLine,
      normalizedCategory,
    );
    const state = EditorState.create({
      doc: source,
      extensions: [
        basicSetup,
        lineNumbers({
          formatNumber: (lineNo) => String(lineNo + lineOffset),
        }),
        languageExtension(filePath),
        EditorView.editable.of(false),
        EditorState.readOnly.of(true),
        highlightCompartment.current.of(highlightExt),
      ],
    });

    const view = new EditorView({ state, parent: host });
    viewRef.current = view;
    scrollToHighlight(view, relativeHighlightLine);

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, [source, filePath, lineOffset]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: highlightCompartment.current.reconfigure(
        buildViolationHighlight(relativeHighlightLine, normalizedCategory),
      ),
    });
    scrollToHighlight(view, relativeHighlightLine);
  }, [relativeHighlightLine, normalizedCategory]);

  return (
    <div
      ref={hostRef}
      class="kantra-snippet-editor-host border rounded overflow-auto"
      data-testid="kantra-snippet-editor"
    />
  );
}

function scrollToHighlight(view: EditorView, relativeLine: number) {
  if (relativeLine < 1 || relativeLine > view.state.doc.lines) return;
  const line = view.state.doc.line(relativeLine);
  view.dispatch({
    effects: EditorView.scrollIntoView(line.from, { y: "center" }),
  });
}

function buildViolationHighlight(
  relativeLine: number,
  category: KantraViolationCategory,
) {
  const field = StateField.define<DecorationSet>({
    create(state) {
      return violationLineDecoration(state.doc, relativeLine, category);
    },
    update(_deco, tr) {
      return violationLineDecoration(tr.state.doc, relativeLine, category);
    },
    provide: (f) => EditorView.decorations.from(f),
  });
  return field;
}

function violationLineDecoration(
  doc: EditorState["doc"],
  relativeLine: number,
  category: KantraViolationCategory,
): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  if (relativeLine < 1 || relativeLine > doc.lines) {
    return builder.finish();
  }
  const line = doc.line(relativeLine);
  builder.add(
    line.from,
    line.from,
    Decoration.line({ class: categoryLineClass(category) }),
  );
  return builder.finish();
}

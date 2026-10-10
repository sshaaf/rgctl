"use client";

import { Children, isValidElement, type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { MermaidDiagram } from "@/components/mermaid-diagram";

function extractText(node: ReactNode): string {
  if (node == null || typeof node === "boolean") return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(extractText).join("");
  if (isValidElement<{ children?: ReactNode }>(node)) {
    return extractText(node.props.children);
  }
  return "";
}

function isMermaidCode(node: ReactNode): node is React.ReactElement<{
  className?: string;
  children?: ReactNode;
}> {
  if (!isValidElement(node)) return false;
  const className = (node.props as { className?: string }).className ?? "";
  return typeof className === "string" && className.split(/\s+/).includes("language-mermaid");
}

export function DocMarkdown({ children }: { children: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        pre({ children: preChildren }) {
          const only = Children.toArray(preChildren)[0];
          if (isMermaidCode(only)) {
            return <MermaidDiagram chart={extractText(only.props.children)} />;
          }
          return <pre>{preChildren}</pre>;
        },
      }}
    >
      {children}
    </ReactMarkdown>
  );
}

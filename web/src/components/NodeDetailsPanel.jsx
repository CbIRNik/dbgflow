import { useState } from "react"
import { ChevronDown, ChevronRight, X } from "lucide-react"
import { Badge, Button, Card, ScrollArea, Separator } from "./ui"

function highlightRust(code) {
  if (!code) return null
  const lines = code.split("\n")

  const escapeHTML = (str) =>
    str.replace(
      /[&<>'"]/g,
      (tag) =>
        ({
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          "'": "&#39;",
          '"': "&quot;",
        })[tag] || tag,
    )

  return lines.map((line, i) => {
    let highlight = escapeHTML(line)
    highlight = highlight
      .replace(/(&quot;.*?&quot;)/g, '<span class="text-green-400">$1</span>')
      .replace(/(\/\/.*)/g, '<span class="text-neutral-500">$1</span>')
      .replace(
        /\b(pub|fn|struct|enum|impl|for|let|mut|match|if|else|return|const|static|use|mod|crate|super)\b/g,
        '<span class="text-orange-400">$1</span>',
      )
      .replace(
        /\b(String|Vec|Option|Result|bool|i32|i64|u32|u64|usize|f32|f64)\b/g,
        '<span class="text-blue-400">$1</span>',
      )

    return (
      <span
        key={i}
        className="block min-h-[1em]"
        dangerouslySetInnerHTML={{ __html: highlight }}
      />
    )
  })
}

const KIND_CONFIG = {
  function: {
    label: "Function",
    className: "details-panel__badge details-panel__badge--neutral",
  },
  type: {
    label: "Data",
    className: "details-panel__badge details-panel__badge--neutral",
  },
  test: {
    label: "Test",
    className: "details-panel__badge details-panel__badge--neutral",
  },
}

function sectionTitle(title) {
  return (
    <div className="text-[11px] font-semibold uppercase tracking-[0.18em] text-muted-foreground">
      {title}
    </div>
  )
}

function recordList(items, emptyLabel) {
  if (!items.length) {
    return <div className="details-panel__empty">{emptyLabel}</div>
  }

  return items.map((item, index) => {
    let highlighted = item.preview
    if (window.Prism && item.preview) {
      try {
        // Try to parse as JSON first
        JSON.parse(item.preview)
        highlighted = window.Prism.highlight(
          item.preview,
          window.Prism.languages.javascript,
          "javascript"
        )
      } catch {
        // If not valid JSON, just use plain text
        highlighted = item.preview
      }
    }
    
    return (
      <div
        className="details-panel__record"
        key={`${item.name}-${item.title}-${index}`}
      >
        <div className="details-panel__record-head">
          <span>{item.name}</span>
          {item.title ? <span>{item.title}</span> : null}
        </div>
        {typeof highlighted === 'string' && highlighted !== item.preview ? (
          <pre className="details-panel__record-preview">
            <code dangerouslySetInnerHTML={{ __html: highlighted }} />
          </pre>
        ) : (
          <pre className="details-panel__record-preview">{item.preview}</pre>
        )}
      </div>
    )
  })
}

export default function NodeDetailsPanel({ nodeData, onClose }) {
  const [isCodeOpen, setIsCodeOpen] = useState(false)
  const kind = KIND_CONFIG[nodeData.node.kind] ?? KIND_CONFIG.function
  const statusClassName =
    nodeData.executionState === "failure"
      ? "details-panel__badge details-panel__badge--danger"
      : nodeData.executionState === "success"
        ? "details-panel__badge details-panel__badge--success"
        : nodeData.executionState === "running"
          ? "details-panel__badge details-panel__badge--running"
          : "details-panel__badge details-panel__badge--neutral"

  return (
    <aside className="details-panel">
      <Card className="details-panel__shell">
        <div className="details-panel__header">
          <div className="min-w-0">
            <div className="details-panel__eyebrow">node details</div>
            <div className="details-panel__title">{nodeData.node.label}</div>
          </div>

          <Button onClick={onClose} size="icon" type="button" variant="ghost">
            <X className="h-4 w-4" />
          </Button>
        </div>

        <Separator />

        <ScrollArea className="details-panel__scroll">
          <div className="details-panel__content">
            <div className="details-panel__badges">
              <Badge className={kind.className} variant="outline">
                {kind.label}
              </Badge>

              <Badge className={statusClassName} variant="outline">
                {nodeData.status?.label ??
                  (nodeData.hasExecuted ? "ran" : "idle")}
              </Badge>
            </div>

            {nodeData.node.source && (
              <section className="details-panel__section">
                <button
                  className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-muted-foreground hover:text-foreground transition-colors"
                  onClick={() => setIsCodeOpen(!isCodeOpen)}
                  type="button"
                >
                  {isCodeOpen ? (
                    <ChevronDown className="h-3.5 w-3.5" />
                  ) : (
                    <ChevronRight className="h-3.5 w-3.5" />
                  )}
                  Source Code
                </button>
                {isCodeOpen && (
                  <div className="details-panel__records">
                    <pre className="details-panel__record-preview language-rust bg-neutral-900 border border-neutral-800 p-3 rounded-md whitespace-pre-wrap break-all text-[11px]">
                      <code
                        className="language-rust"
                        dangerouslySetInnerHTML={{
                          __html: window.Prism
                            ? window.Prism.highlight(
                                nodeData.node.source,
                                window.Prism.languages.rust,
                                "rust",
                              )
                            : highlightRust(nodeData.node.source),
                        }}
                      />
                    </pre>
                  </div>
                )}
              </section>
            )}

            <section className="details-panel__section">
              {sectionTitle("Input")}
              <div className="details-panel__records">
                {recordList(
                  nodeData.inputData,
                  "No captured input for this node in the selected step range.",
                )}
              </div>
            </section>

            <section className="details-panel__section">
              {sectionTitle("Output")}
              <div className="details-panel__records">
                {recordList(
                  nodeData.outputData,
                  "No captured output for this node in the selected step range.",
                )}
              </div>
            </section>
          </div>
        </ScrollArea>
      </Card>
    </aside>
  )
}

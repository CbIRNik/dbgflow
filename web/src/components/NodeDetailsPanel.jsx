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

function highlightData(content, preferredLanguage = null) {
  if (!content || !window.Prism) return null

  const trimmed = content.trim()

  if (preferredLanguage === "rust" && window.Prism.languages.rust) {
    return window.Prism.highlight(content, window.Prism.languages.rust, "rust")
  }

  if (preferredLanguage === "javascript") {
    return window.Prism.highlight(
      content,
      window.Prism.languages.javascript,
      "javascript",
    )
  }

  // Try JSON first (valid JSON objects/arrays)
  try {
    JSON.parse(content)
    return window.Prism.highlight(
      content,
      window.Prism.languages.javascript,
      "javascript"
    )
  } catch {
    // Not valid JSON, continue
  }

  // Check for Rust-like data structures (structs, enums, tuples)
  // Patterns: SomeStruct { ... }, SomeEnum::Variant, (a, b, c)
  const hasRustPatterns = /^[A-Z][a-zA-Z0-9]*\s*[{(]|::|^\(.*\)$/.test(trimmed)
  if (hasRustPatterns && window.Prism.languages.rust) {
    return window.Prism.highlight(content, window.Prism.languages.rust, "rust")
  }

  if (/^fn\s+[a-zA-Z_][a-zA-Z0-9_]*\s*\(/.test(trimmed) && window.Prism.languages.rust) {
    return window.Prism.highlight(content, window.Prism.languages.rust, "rust")
  }

  // Check for simple key-value or object-like structures (even without valid JSON)
  // Patterns: { key: value }, key: value, [items]
  const hasStructurePatterns = /^[\[{]|:\s*["\d\[{]|=>\s*/.test(trimmed)
  if (hasStructurePatterns) {
    return window.Prism.highlight(
      content,
      window.Prism.languages.javascript,
      "javascript"
    )
  }

  // Check for Rust type names (primitives, generics, references)
  // Patterns: i32, String, Vec<T>, Option<T>, Result<T, E>, &str, &mut T, Box<T>
  const isRustType = /^(&\s*(mut\s+)?)?([iu](8|16|32|64|128|size)|f(32|64)|bool|char|str|String|Vec|Option|Result|Box|Rc|Arc|Cell|RefCell|HashMap|HashSet|BTreeMap|BTreeSet|Cow|Pin|PhantomData)(<.*>)?$/.test(trimmed)
    || /^[A-Z][a-zA-Z0-9]*(<.*>)?$/.test(trimmed)  // Custom type names like MyStruct<T>
    || /^&(mut\s+)?[a-zA-Z]/.test(trimmed)  // References like &str, &mut T
    || /^\(.*\)$/.test(trimmed)  // Tuples like (i32, String)
  if (isRustType && window.Prism.languages.rust) {
    return window.Prism.highlight(content, window.Prism.languages.rust, "rust")
  }

  // For anything else with potential code-like content, use javascript highlighting
  // This catches numbers, strings, booleans, etc.
  if (/^[\d"'\-\[{(]|true|false|null|None|Some/.test(trimmed)) {
    return window.Prism.highlight(
      content,
      window.Prism.languages.javascript,
      "javascript"
    )
  }

  // No highlighting for plain text
  return null
}

function recordList(items, emptyLabel) {
  if (!items.length) {
    return <div className="details-panel__empty">{emptyLabel}</div>
  }

  return items.map((item, index) => {
    const highlighted = highlightData(item.preview, item.language)

    return (
      <div
        className="details-panel__record"
        key={`${item.name}-${item.title}-${index}`}
      >
        <div className="details-panel__record-head">
          <span>{item.name}</span>
          {item.title ? <span>{item.title}</span> : null}
        </div>
        {highlighted ? (
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
                    <pre className="details-panel__source-code language-rust bg-neutral-900 border border-neutral-800 p-3 rounded-md text-[11px]">
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

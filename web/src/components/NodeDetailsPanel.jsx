import { useEffect, useRef, useState } from "react"
import { ChevronDown, ChevronRight, X } from "lucide-react"
import { Badge, Button, Card, ScrollArea, Separator } from "./ui"

function escapeHTML(str) {
  return str.replace(
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
}

function highlightRustHTML(code) {
  if (!code) {
    return ""
  }

  return code
    .split("\n")
    .map((line) =>
      escapeHTML(line)
        .replace(/(&quot;.*?&quot;)/g, '<span class="text-green-400">$1</span>')
        .replace(/(\/\/.*)/g, '<span class="text-neutral-500">$1</span>')
        .replace(
          /\b(pub|fn|struct|enum|impl|for|let|mut|match|if|else|return|const|static|use|mod|crate|super)\b/g,
          '<span class="text-orange-400">$1</span>',
        )
        .replace(
          /\b(String|Vec|Option|Result|bool|i32|i64|u32|u64|usize|f32|f64)\b/g,
          '<span class="text-blue-400">$1</span>',
        ),
    )
    .join("\n")
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

function highlightLine(content, preferredLanguage) {
  return highlightData(content, preferredLanguage) ?? escapeHTML(content)
}

function renderHighlightedLines(content, preferredLanguage, keyPrefix) {
  return content.split("\n").map((line, index) => (
    <span
      key={`${keyPrefix}-${index}`}
      className="details-panel__preview-line"
      dangerouslySetInnerHTML={{
        __html: line ? highlightLine(line, preferredLanguage) : "&nbsp;",
      }}
    />
  ))
}

function countBracketDelta(line) {
  const opens = [...line].filter((char) => char === "[").length
  const closes = [...line].filter((char) => char === "]").length
  return opens - closes
}

function collectCollapsibleArrays(content) {
  const lines = content.split("\n")
  const sections = []
  const plainBuffer = []
  const flushPlainBuffer = () => {
    if (!plainBuffer.length) {
      return
    }

    sections.push({
      type: "text",
      content: plainBuffer.join("\n"),
    })
    plainBuffer.length = 0
  }

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index]
    const trimmed = line.trim()
    const looksLikeArrayStart =
      trimmed.endsWith("[") && !trimmed.startsWith("//") && !trimmed.startsWith("#")

    if (!looksLikeArrayStart) {
      plainBuffer.push(line)
      continue
    }

    let depth = countBracketDelta(line)
    if (depth <= 0) {
      plainBuffer.push(line)
      continue
    }

    const arrayLines = [line]
    let endIndex = index
    while (depth > 0 && endIndex + 1 < lines.length) {
      endIndex += 1
      arrayLines.push(lines[endIndex])
      depth += countBracketDelta(lines[endIndex])
    }

    const itemCount = arrayLines.slice(1, -1).filter((entry) => entry.trim()).length
    if (depth !== 0 || itemCount < 10) {
      plainBuffer.push(...arrayLines)
      index = endIndex
      continue
    }

    flushPlainBuffer()

    const startLine = arrayLines[0]
    const endLine = arrayLines[arrayLines.length - 1]
    const collapsedPrefix = startLine.slice(0, startLine.lastIndexOf("[")).trimEnd()
    const collapsedSuffix = endLine.trim().endsWith(",") ? "," : ""

    sections.push({
      type: "array",
      collapsedLine: `${collapsedPrefix} [ … ${itemCount} items ]${collapsedSuffix}`,
      expandedContent: arrayLines.join("\n"),
      key: `${index}-${itemCount}`,
    })

    index = endIndex
  }

  flushPlainBuffer()

  return sections.some((section) => section.type === "array") ? sections : null
}

function formatSourceCode(content) {
  return content
    .split("\n")
    .flatMap((line) => {
      const trimmed = line.trim()
      const shouldWrap =
        trimmed.length > 72 &&
        (trimmed.includes(".await") || (trimmed.match(/\.[A-Za-z_][A-Za-z0-9_]*\s*\(/g) ?? []).length >= 2)

      if (!shouldWrap) {
        return [line]
      }

      const segments = line.split(/(?=\.[A-Za-z_][A-Za-z0-9_]*\s*\(|\.await\b)/g)
      if (segments.length < 3) {
        return [line]
      }

      const indent = line.match(/^\s*/)?.[0] ?? ""
      return [segments[0], ...segments.slice(1).map((segment) => `${indent}    ${segment.trimStart()}`)]
    })
    .join("\n")
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

function CollapsibleArrayBlock({ section, preferredLanguage }) {
  const [isExpanded, setIsExpanded] = useState(false)

  return (
    <div className="details-panel__array-section">
      <button
        className="details-panel__array-toggle"
        onClick={() => setIsExpanded((current) => !current)}
        type="button"
      >
        {isExpanded ? "Hide array" : "Show array"}
      </button>
      {isExpanded
        ? renderHighlightedLines(
            section.expandedContent,
            preferredLanguage,
            `array-expanded-${section.key}`,
          )
        : renderHighlightedLines(
            section.collapsedLine,
            preferredLanguage,
            `array-collapsed-${section.key}`,
          )}
    </div>
  )
}

function PreviewBlock({ content, preferredLanguage = null }) {
  const sections = collectCollapsibleArrays(content)

  if (!sections) {
    const highlighted = highlightData(content, preferredLanguage)
    return highlighted ? (
      <pre className="details-panel__record-preview">
        <code dangerouslySetInnerHTML={{ __html: highlighted }} />
      </pre>
    ) : (
      <pre className="details-panel__record-preview">{content}</pre>
    )
  }

  return (
    <div className="details-panel__record-preview details-panel__record-preview--structured">
      {sections.map((section, index) =>
        section.type === "array" ? (
          <CollapsibleArrayBlock
            key={section.key}
            preferredLanguage={preferredLanguage}
            section={section}
          />
        ) : (
          <div className="details-panel__text-section" key={`text-${index}`}>
            {renderHighlightedLines(section.content, preferredLanguage, `text-${index}`)}
          </div>
        ),
      )}
    </div>
  )
}

function recordList(items, emptyLabel) {
  if (!items.length) {
    return <div className="details-panel__empty">{emptyLabel}</div>
  }

  return items.map((item, index) => {
    return (
      <div
        className="details-panel__record"
        key={`${item.name}-${item.title}-${index}`}
        >
          <div className="details-panel__record-head">
            <span>{item.name}</span>
            {item.title ? <span>{item.title}</span> : null}
          </div>
          <PreviewBlock content={item.preview} preferredLanguage={item.language} />
      </div>
    )
  })
}

export default function NodeDetailsPanel({
  nodeData,
  onClose,
  onResize,
  width,
  minWidth,
  maxWidth,
}) {
  const [isCodeOpen, setIsCodeOpen] = useState(false)
  const dragStateRef = useRef(null)
  const formattedSource = nodeData.node.source
    ? formatSourceCode(nodeData.node.source)
    : ""
  const kind = KIND_CONFIG[nodeData.node.kind] ?? KIND_CONFIG.function
  const statusClassName =
    nodeData.executionState === "failure"
      ? "details-panel__badge details-panel__badge--danger"
      : nodeData.executionState === "success"
        ? "details-panel__badge details-panel__badge--success"
        : nodeData.executionState === "running"
          ? "details-panel__badge details-panel__badge--running"
          : "details-panel__badge details-panel__badge--neutral"

  useEffect(() => {
    const stopResize = () => {
      dragStateRef.current = null
      document.body.style.cursor = ""
      document.body.style.userSelect = ""
    }

    const handlePointerMove = (event) => {
      if (!dragStateRef.current) {
        return
      }

      const nextWidth = dragStateRef.current.startWidth + event.clientX - dragStateRef.current.startX
      const clampedWidth = Math.max(minWidth, Math.min(maxWidth, nextWidth))
      onResize(clampedWidth)
    }

    const handlePointerUp = () => {
      stopResize()
    }

    window.addEventListener("pointermove", handlePointerMove)
    window.addEventListener("pointerup", handlePointerUp)

    return () => {
      window.removeEventListener("pointermove", handlePointerMove)
      window.removeEventListener("pointerup", handlePointerUp)
      stopResize()
    }
  }, [maxWidth, minWidth, onResize])

  return (
    <aside className="details-panel" style={{ width }}>
      <div
        aria-hidden="true"
        className="details-panel__resize-handle"
        onPointerDown={(event) => {
          dragStateRef.current = {
            startWidth: width,
            startX: event.clientX,
          }
          document.body.style.cursor = "col-resize"
          document.body.style.userSelect = "none"
        }}
      />
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
                                formattedSource,
                                window.Prism.languages.rust,
                                "rust",
                              )
                            : highlightRustHTML(formattedSource),
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

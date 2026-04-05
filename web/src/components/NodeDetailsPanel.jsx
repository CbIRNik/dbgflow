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

function detectPreviewBlockKind(trimmedLine) {
  if (trimmedLine.endsWith("{")) {
    return "object"
  }

  if (trimmedLine.endsWith("[")) {
    return "array"
  }

  return null
}

function isPreviewBlockClosingLine(trimmedLine, kind) {
  if (kind === "object") {
    return trimmedLine === "}" || trimmedLine === "},"
  }

  if (kind === "array") {
    return trimmedLine === "]" || trimmedLine === "],"
  }

  return false
}

function countVisiblePreviewEntries(entries) {
  return entries.reduce((count, entry) => {
    if (entry.type === "line") {
      return entry.content.trim() ? count + 1 : count
    }

    return count + 1
  }, 0)
}

function isPreviewBlockCollapsible(block) {
  const visibleEntryCount = countVisiblePreviewEntries(block.children)
  const hasNestedBlocks = block.children.some((entry) => entry.type === "block")

  return visibleEntryCount >= 3 || hasNestedBlocks
}

function containsCollapsiblePreviewBlock(entries) {
  return entries.some((entry) => {
    if (entry.type === "line") {
      return false
    }

    return (
      isPreviewBlockCollapsible(entry) ||
      containsCollapsiblePreviewBlock(entry.children)
    )
  })
}

function buildCollapsedBlockLine(block) {
  const bracket = block.kind === "array" ? "[" : "{"
  const summaryLabel = block.kind === "array" ? "items" : "fields"
  const prefix = block.openLine.slice(0, block.openLine.lastIndexOf(bracket)).trimEnd()
  const indent = block.openLine.match(/^\s*/)?.[0] ?? ""
  const suffix = block.closeLine.trimEnd().endsWith(",") ? "," : ""
  const visibleEntryCount = countVisiblePreviewEntries(block.children)

  if (block.kind === "array") {
    return prefix
      ? `${prefix} [ … ${visibleEntryCount} ${summaryLabel} ]${suffix}`
      : `${indent}[ … ${visibleEntryCount} ${summaryLabel} ]${suffix}`
  }

  return prefix
    ? `${prefix} { … ${visibleEntryCount} ${summaryLabel} }${suffix}`
    : `${indent}{ … ${visibleEntryCount} ${summaryLabel} }${suffix}`
}

function buildStructuredPreviewTree(content) {
  const lines = content.split("\n")
  if (lines.length < 2) {
    return null
  }

  const root = { children: [] }
  const stack = [root]

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index]
    const trimmedLine = line.trim()
    const blockKind = detectPreviewBlockKind(trimmedLine)

    if (blockKind) {
      const block = {
        type: "block",
        kind: blockKind,
        key: `block-${index}-${blockKind}`,
        openLine: line,
        closeLine: "",
        children: [],
      }
      stack[stack.length - 1].children.push(block)
      stack.push(block)
      continue
    }

    const currentBlock = stack[stack.length - 1]
    if (
      stack.length > 1 &&
      isPreviewBlockClosingLine(trimmedLine, currentBlock.kind)
    ) {
      currentBlock.closeLine = line
      stack.pop()
      continue
    }

    stack[stack.length - 1].children.push({
      type: "line",
      key: `line-${index}`,
      content: line,
    })
  }

  if (stack.length !== 1 || !containsCollapsiblePreviewBlock(root.children)) {
    return null
  }

  return root.children
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

function renderPreviewEntries(entries, preferredLanguage, keyPrefix) {
  return entries.map((entry, index) =>
    entry.type === "line" ? (
      <span
        key={`${keyPrefix}-${entry.key}-${index}`}
        className="details-panel__preview-line"
        dangerouslySetInnerHTML={{
          __html: entry.content
            ? highlightLine(entry.content, preferredLanguage)
            : "&nbsp;",
        }}
      />
    ) : (
      <FoldablePreviewBlock
        key={`${keyPrefix}-${entry.key}-${index}`}
        block={entry}
        preferredLanguage={preferredLanguage}
      />
    ),
  )
}

function FoldablePreviewBlock({ block, preferredLanguage }) {
  const isCollapsible = isPreviewBlockCollapsible(block)
  const [isExpanded, setIsExpanded] = useState(!isCollapsible)

  return (
    <div className="details-panel__fold-block">
      {isCollapsible ? (
        <button
          className="details-panel__fold-button"
          onClick={() => setIsExpanded((current) => !current)}
          type="button"
        >
          {isExpanded ? (
            <ChevronDown className="details-panel__fold-indicator" />
          ) : (
            <ChevronRight className="details-panel__fold-indicator" />
          )}
          <span
            className="details-panel__preview-line"
            dangerouslySetInnerHTML={{
              __html: highlightLine(
                isExpanded ? block.openLine : buildCollapsedBlockLine(block),
                preferredLanguage,
              ),
            }}
          />
        </button>
      ) : (
        renderHighlightedLines(
          block.openLine,
          preferredLanguage,
          `block-open-${block.key}`,
        )
      )}

      {isExpanded ? (
        <div className="details-panel__fold-body">
          {renderPreviewEntries(block.children, preferredLanguage, block.key)}
          {block.closeLine
            ? renderHighlightedLines(
                block.closeLine,
                preferredLanguage,
                `block-close-${block.key}`,
              )
            : null}
        </div>
      ) : null}
    </div>
  )
}

function PreviewBlock({ content, preferredLanguage = null }) {
  const sections = buildStructuredPreviewTree(content)

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
      {renderPreviewEntries(sections, preferredLanguage, "preview")}
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

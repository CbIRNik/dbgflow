import { useRef } from "react"
import { Handle, Position } from "@xyflow/react"
import { Play } from "lucide-react"
import { Button } from "./ui"

const KIND_CONFIG = {
  function: {
    icon: "fn",
  },
  type: {
    icon: "db",
  },
  test: {
    icon: "t",
  },
}

function formatNodeLabel(node) {
  if (node.kind === "function") {
    return `${node.label}() {}`
  }
  return node.label
}

export default function GraphNode({ data, selected }) {
  const pointerStateRef = useRef(null)
  const node = data.node
  const kind = KIND_CONFIG[node.kind] ?? KIND_CONFIG.function
  const isSelected = data.isSelected || selected
  const executionStateClassName =
    data.executionState === "running"
      ? "workflow-node--running-static"
      : `workflow-node--${data.executionState}`
  const animationClassName = data.isAnimating
    ? "workflow-node--running-animated"
    : ""
  const handoffClassName =
    data.handoffState === "source"
      ? "workflow-node--handoff-source"
      : data.handoffState === "target"
        ? "workflow-node--handoff-target"
        : ""
  const interactionModeClassName =
    data.canvasMode === "move-nodes" ? "workflow-node--draggable" : ""
  const statusClassName =
    data.executionState === "running"
      ? [
          "workflow-node__status",
          "workflow-node__status--running-static",
          data.isAnimating ? "workflow-node__status--running-animated" : "",
        ]
          .filter(Boolean)
          .join(" ")
      : `workflow-node__status workflow-node__status--${data.executionState}`
  const openNodeDetails = () => {
    data.onOpenDetails(node.id)
  }

  return (
    <div
      aria-pressed={isSelected}
      className={[
        "workflow-node",
        isSelected ? "is-selected" : "",
        executionStateClassName,
        animationClassName,
        handoffClassName,
        interactionModeClassName,
      ]
        .filter(Boolean)
        .join(" ")}
      onPointerDown={(event) => {
        if (event.button !== 0) {
          return
        }

        pointerStateRef.current = {
          moved: false,
          x: event.clientX,
          y: event.clientY,
        }
        event.currentTarget.focus()
        if (data.canvasMode !== "move-nodes") {
          openNodeDetails()
        }
      }}
      onPointerMove={(event) => {
        if (!pointerStateRef.current || pointerStateRef.current.moved) {
          return
        }

        const deltaX = Math.abs(event.clientX - pointerStateRef.current.x)
        const deltaY = Math.abs(event.clientY - pointerStateRef.current.y)
        if (deltaX > 6 || deltaY > 6) {
          pointerStateRef.current.moved = true
        }
      }}
      onClick={() => {
        if (data.canvasMode !== "move-nodes") {
          pointerStateRef.current = null
          return
        }

        if (!pointerStateRef.current?.moved) {
          openNodeDetails()
        }
        pointerStateRef.current = null
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault()
          openNodeDetails()
        }
      }}
      onPointerCancel={() => {
        pointerStateRef.current = null
      }}
      role="button"
      style={{ width: data.dimensions.width }}
      tabIndex={0}
    >
      <Handle
        className="workflow-node__handle"
        position={Position.Left}
        type="target"
      />

      <div className="workflow-node__body">
        <span className="workflow-node__kind">{kind.icon}</span>
        <span className="workflow-node__name">{formatNodeLabel(node)}</span>
        <span className={statusClassName} />
      </div>

      {data.canRunChain ? (
        <Button
          className="workflow-node__action gap-2"
          onClick={(event) => {
            event.stopPropagation()
            data.onRunChain(node.id)
          }}
          onPointerDown={(event) => event.stopPropagation()}
          size="sm"
          type="button"
        >
          <Play className="h-3.5 w-3.5" />
          Run chain
        </Button>
      ) : null}

      <Handle
        className="workflow-node__handle"
        position={Position.Right}
        type="source"
      />
    </div>
  )
}

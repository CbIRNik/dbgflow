import { useEffect, useRef } from "react"
import {
  Background,
  ReactFlow,
  useNodesInitialized,
  useReactFlow,
} from "@xyflow/react"
import "@xyflow/react/dist/style.css"
import GraphNode from "./GraphNode"

const nodeTypes = {
  inspector: GraphNode,
}
const MIN_CANVAS_ZOOM = 0.16
const MAX_CANVAS_ZOOM = 1.8
const FIT_VIEW_PADDING = 0.16

function ViewportSync({ fitViewKey }) {
  const reactFlow = useReactFlow()
  const nodesInitialized = useNodesInitialized()
  const lastFitViewKeyRef = useRef("")

  useEffect(() => {
    if (!nodesInitialized || !fitViewKey) {
      return
    }

    if (lastFitViewKeyRef.current === fitViewKey) {
      return
    }

    lastFitViewKeyRef.current = fitViewKey

    const frameId = window.requestAnimationFrame(() => {
      void reactFlow.fitView({
        duration: 480,
        maxZoom: 1.18,
        minZoom: MIN_CANVAS_ZOOM,
        padding: FIT_VIEW_PADDING,
      })
    })

    return () => {
      window.cancelAnimationFrame(frameId)
    }
  }, [fitViewKey, nodesInitialized, reactFlow])

  return null
}

export default function WorkflowCanvas({
  canvasMode,
  edges,
  fitViewKey,
  nodes,
  onNodesChange,
  onPaneClick,
}) {
  return (
    <section className={`workflow-stage workflow-stage--full workflow-stage--${canvasMode}`}>
      <ReactFlow
        edges={edges}
        elementsSelectable={false}
        maxZoom={MAX_CANVAS_ZOOM}
        minZoom={MIN_CANVAS_ZOOM}
        nodeTypes={nodeTypes}
        nodes={nodes}
        nodesConnectable={false}
        nodesDraggable={canvasMode === "move-nodes"}
        onNodesChange={onNodesChange}
        onPaneClick={onPaneClick}
        onlyRenderVisibleElements
        panOnDrag={canvasMode === "pan-canvas"}
        proOptions={{ hideAttribution: true }}
        selectNodesOnDrag={false}
      >
        <ViewportSync fitViewKey={fitViewKey} />
        <Background color="#1e1e1e" gap={22} size={1.1} variant="dots" />
      </ReactFlow>
    </section>
  )
}

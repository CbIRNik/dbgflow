import { useEffect, useRef, useState, useCallback } from "react"
import { Background, ReactFlow, useNodesInitialized, useReactFlow, applyNodeChanges } from "@xyflow/react"
import "@xyflow/react/dist/style.css"
import GraphNode from "./GraphNode"
import { useUIStore } from "../store"

const nodeTypes = {
  inspector: GraphNode,
}
const MIN_CANVAS_ZOOM = 0.16
const MAX_CANVAS_ZOOM = 1.8
const FIT_VIEW_PADDING = 0.16

function ViewportSync({ fitViewKey, nodeCount }) {
  const reactFlow = useReactFlow()
  const nodesInitialized = useNodesInitialized()
  const fittedKeyRef = useRef("")

  useEffect(() => {
    if (!fitViewKey || nodeCount === 0 || !nodesInitialized) {
      return
    }

    if (fittedKeyRef.current === fitViewKey) {
      return
    }

    let frameId = 0
    let attempts = 0

    const fit = () => {
      attempts += 1
      void reactFlow.fitView({
        duration: attempts === 1 ? 0 : 360,
        maxZoom: 1.18,
        minZoom: MIN_CANVAS_ZOOM,
        padding: FIT_VIEW_PADDING,
      })

      if (attempts >= 6) {
        fittedKeyRef.current = fitViewKey
        return
      }

      frameId = window.requestAnimationFrame(fit)
    }

    frameId = window.requestAnimationFrame(fit)
    return () => {
      window.cancelAnimationFrame(frameId)
    }
  }, [fitViewKey, nodeCount, nodesInitialized, reactFlow])

  return null
}

export default function WorkflowCanvas({
  edges,
  fitViewKey,
  nodes: initialNodes,
  onNodesChange: externalOnNodesChange,
  onNodeSelect,
  onPaneClick,
}) {
  const canvasMode = useUIStore((state) => state.canvasMode)
  const [localNodes, setLocalNodes] = useState(initialNodes)

  // Sync external nodes when they change significantly (e.g. graph reload)
  useEffect(() => {
    setLocalNodes(initialNodes)
  }, [initialNodes])

  const onNodesChange = useCallback((changes) => {
    setLocalNodes((nds) => applyNodeChanges(changes, nds))
    // Pass changes upstream so positions can be saved (without causing heavy re-renders)
    externalOnNodesChange?.(changes)
  }, [externalOnNodesChange])
  
  return (
    <section className={`workflow-stage workflow-stage--full workflow-stage--${canvasMode}`}>
      <ReactFlow
        key={fitViewKey || "workflow-canvas"}
        edges={edges}
        edgesFocusable={false}
        edgesReconnectable={false}
        elementsSelectable={false}
        maxZoom={MAX_CANVAS_ZOOM}
        minZoom={MIN_CANVAS_ZOOM}
        nodeTypes={nodeTypes}
        nodes={localNodes}
        nodesConnectable={false}
        nodesDraggable={canvasMode === "move-nodes"}
        onNodeClick={(_, node) => {
          onNodeSelect?.(node.id)
        }}
        onNodesChange={onNodesChange}
        onEdgeClick={() => {
          onPaneClick?.()
        }}
        onPaneClick={onPaneClick}
        panOnDrag={canvasMode === "pan-canvas"}
        selectNodesOnDrag={false}
      >
        <ViewportSync fitViewKey={fitViewKey} nodeCount={localNodes.length} />
        <Background color="#1e1e1e" gap={22} size={1.1} variant="dots" />
      </ReactFlow>
    </section>
  )
}

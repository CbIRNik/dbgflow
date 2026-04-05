import { useEffect, useRef } from "react"
import {
  Background,
  ReactFlow,
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

function ViewportSync({ fitViewKey, nodeCount }) {
  const reactFlow = useReactFlow()
  const lastFitViewKeyRef = useRef("")

  useEffect(() => {
    if (!fitViewKey || nodeCount === 0) {
      return
    }

    if (lastFitViewKeyRef.current === fitViewKey) {
      return
    }

    lastFitViewKeyRef.current = fitViewKey

    let firstFrameId = 0
    let secondFrameId = 0

    firstFrameId = window.requestAnimationFrame(() => {
      secondFrameId = window.requestAnimationFrame(() => {
        void reactFlow.fitView({
          duration: 480,
          maxZoom: 1.18,
          minZoom: MIN_CANVAS_ZOOM,
          padding: FIT_VIEW_PADDING,
        })
      })
    })

    return () => {
      window.cancelAnimationFrame(firstFrameId)
      window.cancelAnimationFrame(secondFrameId)
    }
  }, [fitViewKey, nodeCount, reactFlow])

  return null
}

export default function WorkflowCanvas({
  canvasMode,
  edges,
  fitViewKey,
  nodes,
  onNodesChange,
  onNodeSelect,
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
        onNodeClick={(_, node) => {
          onNodeSelect?.(node.id)
        }}
        onNodesChange={onNodesChange}
        onPaneClick={onPaneClick}
        panOnDrag={canvasMode === "pan-canvas"}
        proOptions={{ hideAttribution: true }}
        selectNodesOnDrag={false}
      >
        <ViewportSync fitViewKey={fitViewKey} nodeCount={nodes.length} />
        <Background color="#1e1e1e" gap={22} size={1.1} variant="dots" />
      </ReactFlow>
    </section>
  )
}

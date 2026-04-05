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
        duration: 240,
        maxZoom: 1.18,
        minZoom: 0.42,
        padding: 0.22,
      })
    })

    return () => {
      window.cancelAnimationFrame(frameId)
    }
  }, [fitViewKey, nodesInitialized, reactFlow])

  return null
}

export default function WorkflowCanvas({
  edges,
  fitViewKey,
  nodes,
  onNodeSelect,
  onNodesChange,
  onPaneClick,
}) {
  return (
    <section className="workflow-stage workflow-stage--full">
      <ReactFlow
        edges={edges}
        elementsSelectable={false}
        maxZoom={1.8}
        minZoom={0.42}
        nodeTypes={nodeTypes}
        nodes={nodes}
        nodesConnectable={false}
        nodesDraggable={false}
        onNodeClick={(_, node) => {
          onNodeSelect(node.id)
        }}
        onNodesChange={onNodesChange}
        onPaneClick={onPaneClick}
        proOptions={{ hideAttribution: true }}
        selectNodesOnDrag={false}
      >
        <ViewportSync fitViewKey={fitViewKey} />
        <Background color="#1e1e1e" gap={22} size={1.1} variant="dots" />
      </ReactFlow>
    </section>
  )
}

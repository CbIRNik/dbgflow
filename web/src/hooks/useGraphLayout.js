import { useCallback, useEffect, useMemo, useState } from "react"
import { MarkerType, Position, applyNodeChanges } from "@xyflow/react"
import { NODE_DIMENSIONS, EDGE_COLORS } from "../utils/constants.js"
import { applyLayout } from "../utils/graphUtils.js"

function roundPosition(position) {
  return {
    x: Number(position.x.toFixed(2)),
    y: Number(position.y.toFixed(2)),
  }
}

function buildEdges(graphModel, activePlaybackNodeId, activeEdgeIds) {
  return graphModel.session.edges.map((edge, index) => {
    const isVisible =
      graphModel.visitedNodeIds.size === 0 ||
      graphModel.visitedNodeIds.has(edge.from) ||
      graphModel.visitedNodeIds.has(edge.to)
    const edgeId = `${edge.from}::${edge.to}`
    const touchesActiveNode = Boolean(
      activePlaybackNodeId &&
        (edge.from === activePlaybackNodeId || edge.to === activePlaybackNodeId),
    )
    const isActive =
      activeEdgeIds.has(edgeId) || (touchesActiveNode && edge.kind === "test_link")
    const stroke = isActive
      ? "#f5f5f5"
      : EDGE_COLORS[edge.kind] ?? EDGE_COLORS.control_flow

    return {
      id: `${edge.from}::${edge.to}::${index}`,
      source: edge.from,
      target: edge.to,
      type: "smoothstep",
      className: `workflow-edge ${isActive ? "is-active" : ""}`,
      pathOptions: {
        borderRadius: 14,
        offset: 18,
      },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 9,
        height: 9,
        color: stroke,
      },
      style: {
        stroke,
        opacity: isVisible ? (isActive ? 1 : 0.8) : 0.2,
        strokeWidth: isActive ? 3.1 : 1.9,
      },
    }
  })
}

function buildPositionMap(nodes, layout, savedNodePositions) {
  const positions = {}

  for (const node of nodes) {
    positions[node.id] = roundPosition(
      savedNodePositions?.[node.id] ?? layout.get(node.id) ?? { x: 0, y: 0 },
    )
  }

  return positions
}

function buildNodesForChangeSet(nodes, nodePositions, layout) {
  return nodes.map((node) => ({
    id: node.id,
    position: nodePositions[node.id] ?? layout.get(node.id) ?? { x: 0, y: 0 },
  }))
}

export function useGraphLayout({
  graphModel,
  selectedRun,
  selectedNodeId,
  canvasMode,
  activePlaybackNodeId,
  activeEdgeIds,
  onRunChain,
  onOpenDetails,
  buildNodeData,
  savedNodePositions,
}) {
  const graphNodes = graphModel?.session.nodes ?? []
  const graphEdges = graphModel?.session.edges ?? []
  const firstSeenSeqByNode = graphModel?.firstSeenSeqByNode ?? null
  const layoutKey = selectedRun
    ? `${selectedRun.id}:${graphNodes.length}:${graphEdges.length}`
    : ""

  const layout = useMemo(() => {
    if (!graphModel) {
      return new Map()
    }

    return applyLayout(
      graphNodes,
      graphEdges,
      firstSeenSeqByNode,
    )
  }, [firstSeenSeqByNode, graphEdges, graphNodes])
  const [nodePositions, setNodePositions] = useState({})

  useEffect(() => {
    if (!graphModel) {
      setNodePositions({})
      return
    }

    setNodePositions(buildPositionMap(graphNodes, layout, savedNodePositions))
  }, [graphNodes, layout, layoutKey, savedNodePositions])

  const edges = useMemo(() => {
    if (!graphModel) {
      return []
    }

    return buildEdges(graphModel, activePlaybackNodeId, activeEdgeIds)
  }, [graphModel, activePlaybackNodeId, activeEdgeIds])

  const nodes = useMemo(() => {
    if (!graphModel) {
      return []
    }

    return graphNodes.map((node) => ({
      id: node.id,
      type: "inspector",
      draggable: canvasMode === "move-nodes",
      selected: selectedNodeId === node.id,
      position: nodePositions[node.id] ?? layout.get(node.id) ?? { x: 0, y: 0 },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
      data: {
        ...buildNodeData(
          graphModel,
          selectedRun,
          node.id,
          selectedNodeId,
          activePlaybackNodeId,
          onRunChain,
          onOpenDetails,
        ),
        canvasMode,
      },
    }))
  }, [
    graphModel,
    graphNodes,
    selectedRun,
    selectedNodeId,
    canvasMode,
    nodePositions,
    layout,
    activePlaybackNodeId,
    onRunChain,
    onOpenDetails,
    buildNodeData,
  ])

  const nodePositionSnapshot = useMemo(() => {
    if (!graphNodes.length) {
      return null
    }

    return Object.fromEntries(
      graphNodes.map((node) => [
        node.id,
        roundPosition(nodePositions[node.id] ?? layout.get(node.id) ?? { x: 0, y: 0 }),
      ]),
    )
  }, [graphNodes, layout, nodePositions])

  const onNodesChange = useCallback((changes) => {
    if (!graphNodes.length || canvasMode !== "move-nodes") {
      return
    }

    setNodePositions((currentPositions) => {
      const currentNodes = buildNodesForChangeSet(graphNodes, currentPositions, layout)
      const nextNodes = applyNodeChanges(changes, currentNodes)

      return Object.fromEntries(
        nextNodes.map((node) => [node.id, roundPosition(node.position)]),
      )
    })
  }, [canvasMode, graphNodes, layout])

  return { nodes, edges, nodePositionSnapshot, onNodesChange }
}

export { NODE_DIMENSIONS, EDGE_COLORS }

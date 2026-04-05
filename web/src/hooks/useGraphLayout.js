import { useEffect, useState } from "react";
import { MarkerType, Position, applyNodeChanges } from "@xyflow/react";
import { NODE_DIMENSIONS, EDGE_COLORS } from "../utils/constants.js";
import { applyLayout } from "../utils/graphUtils.js";

function buildEdges(graphModel, activePlaybackNodeId, activeEdgeIds, handoff, isPlaying) {
  return graphModel.session.edges.map((edge, index) => {
    const isVisible = graphModel.visitedNodeIds.size === 0 || graphModel.visitedNodeIds.has(edge.from) || graphModel.visitedNodeIds.has(edge.to);
    const edgeId = `${edge.from}::${edge.to}`;
    const isHandoff = Boolean(handoff?.edgeIds?.has(edgeId));
    const touchesActiveNode = Boolean(
      activePlaybackNodeId && (edge.from === activePlaybackNodeId || edge.to === activePlaybackNodeId)
    );
    const isActive = activeEdgeIds.has(edgeId) || (touchesActiveNode && edge.kind === "test_link");
    const stroke = isActive ? "#f5f5f5" : EDGE_COLORS[edge.kind] ?? EDGE_COLORS.control_flow;

    return {
      id: `${edge.from}::${edge.to}::${index}`,
      source: edge.from,
      target: edge.to,
      type: "smoothstep",
      animated: isHandoff,
      className: `workflow-edge ${isActive ? "is-active" : ""} ${isHandoff ? "is-handoff" : ""}`,
      pathOptions: {
        borderRadius: 14,
        offset: 18
      },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 9,
        height: 9,
        color: stroke
      },
      style: {
        stroke,
        opacity: isVisible ? (isActive || isHandoff ? 1 : 0.8) : 0.2,
        strokeWidth: isActive ? 3.1 : isHandoff ? 2.6 : 1.9
      }
    };
  });
}

/**
 * Hook for dagre graph layout calculation.
 * @param {object} options
 * @param {object|null} options.graphModel - The graph model containing session data
 * @param {object|null} options.selectedRun - The currently selected run
 * @param {string} options.selectedNodeId - ID of the currently selected node
 * @param {string} options.detailsNodeId - ID of the node with open details panel
 * @param {string|null} options.activePlaybackNodeId - ID of the active playback node
 * @param {Set} options.activeEdgeIds - Set of active edge IDs
 * @param {boolean} options.isPlaying - Whether playback is active
 * @param {Function} options.onRunChain - Callback when running chain from a start node
 * @param {Function} options.onOpenDetails - Callback when opening node details
 * @param {Function} options.buildNodeData - Function to build node data
 * @returns {{ nodes: Array, edges: Array, onNodesChange: Function, setNodes: Function }}
 */
export function useGraphLayout({
  graphModel,
  selectedRun,
  selectedNodeId,
  detailsNodeId,
  activePlaybackNodeId,
  activeEdgeIds,
  handoff,
  isPlaying,
  onRunChain,
  onOpenDetails,
  buildNodeData
}) {
  const [nodes, setNodes] = useState([]);
  const [edges, setEdges] = useState([]);

  useEffect(() => {
    if (!graphModel) {
      return;
    }

    const layout = applyLayout(
      graphModel.session.nodes,
      graphModel.session.edges,
      graphModel.firstSeenSeqByNode
    );

    setEdges(buildEdges(graphModel, activePlaybackNodeId, activeEdgeIds, handoff, isPlaying));
    setNodes((currentNodes) => {
      const currentById = new Map(currentNodes.map((node) => [node.id, node]));

      return graphModel.session.nodes.map((node) => ({
        id: node.id,
        type: "inspector",
        draggable: false,
        selected: selectedNodeId === node.id,
        position: currentById.get(node.id)?.position ?? layout.get(node.id) ?? { x: 0, y: 0 },
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
        data: {
          ...buildNodeData(
            graphModel,
            selectedRun,
            node.id,
            selectedNodeId,
            detailsNodeId,
            activePlaybackNodeId,
            isPlaying,
            onRunChain,
            onOpenDetails
          ),
          handoffState:
            handoff?.from === node.id
              ? "source"
              : handoff?.to === node.id
                ? "target"
                : null
        }
      }));
    });
  }, [graphModel, selectedRun, selectedNodeId, detailsNodeId, activePlaybackNodeId, activeEdgeIds, handoff, isPlaying, onRunChain, onOpenDetails, buildNodeData]);

  const onNodesChange = (changes) => {
    setNodes((currentNodes) => applyNodeChanges(changes, currentNodes));
  };

  return { nodes, edges, onNodesChange, setNodes };
}

export { NODE_DIMENSIONS, EDGE_COLORS };

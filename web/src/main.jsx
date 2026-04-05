import "prismjs"
import "prismjs/components/prism-rust"
import "prismjs/themes/prism-tomorrow.css"
import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from "react"
import { createRoot } from "react-dom/client"
import "./styles.css"
import { usePipelineStore } from "./store"
import { triggerRerun as triggerServerRerun } from "./utils/api.js"
import { deriveChainRuns } from "./utils/chainUtils.js"
import {
  useSession,
  usePlayback,
  useGraphLayout,
  useWorkflowModel,
  NODE_DIMENSIONS,
} from "./hooks"
import NodeDetailsPanel from "./components/NodeDetailsPanel"
import PlaybackControls from "./components/PlaybackControls"
import WorkflowCanvas from "./components/WorkflowCanvas"
import WorkflowEmptyState from "./components/WorkflowEmptyState"
import { buildGraphModel, buildNodeData } from "./utils/graphUtils.js"
import { shortenPreview } from "./utils/formatUtils.js"

const RUN_ROUTE_PREFIX = "#/pipelines/"
const DEFAULT_PANEL_WIDTH = 420
const MIN_PANEL_WIDTH = 320
const MAX_PANEL_WIDTH = 680
const DEFAULT_CANVAS_MODE = "pan-canvas"

function readRunRouteId() {
  if (typeof window === "undefined") {
    return ""
  }

  const { hash } = window.location
  if (hash.startsWith(RUN_ROUTE_PREFIX)) {
    try {
      const parsed = decodeURIComponent(hash.slice(RUN_ROUTE_PREFIX.length))
      localStorage.setItem("dbgflow_last_run_id", parsed)
      return parsed
    } catch {
      // Ignored
    }
  }

  return localStorage.getItem("dbgflow_last_run_id") || ""
}

function buildRunRoute(runId) {
  return runId ? `${RUN_ROUTE_PREFIX}${encodeURIComponent(runId)}` : ""
}

function App() {
  const { session, serverStatus, error, setServerStatus } = useSession()
  const { getPipelineState, setPipelineState } = usePipelineStore()
  const [routeRunId, setRouteRunId] = useState(() => readRunRouteId())
  const [selectedNodeId, setSelectedNodeId] = useState("")
  const [detailsNodeId, setDetailsNodeId] = useState("")
  const [isDetailsOpen, setIsDetailsOpen] = useState(false)
  const [detailsPanelWidth, setDetailsPanelWidth] = useState(DEFAULT_PANEL_WIDTH)
  const [canvasMode, setCanvasMode] = useState(DEFAULT_CANVAS_MODE)
  const [savedNodePositions, setSavedNodePositions] = useState(null)
  const [savedNodePositionsPipelineId, setSavedNodePositionsPipelineId] = useState("")
  const chainRuns = useMemo(() => deriveChainRuns(session), [session])
  const isRestoringState = useRef(false)

  useEffect(() => {
    if (typeof window === "undefined") {
      return undefined
    }

    const syncRouteRunId = () => {
      setRouteRunId(readRunRouteId())
    }

    window.addEventListener("hashchange", syncRouteRunId)
    return () => {
      window.removeEventListener("hashchange", syncRouteRunId)
    }
  }, [])

  const navigateToRun = useCallback((runId, replace = false) => {
    localStorage.setItem("dbgflow_last_run_id", runId)
    const nextHash = buildRunRoute(runId)

    if (typeof window === "undefined") {
      setRouteRunId(runId)
      return
    }

    if (window.location.hash === nextHash) {
      setRouteRunId(runId)
      return
    }

    const nextUrl = `${window.location.pathname}${window.location.search}${nextHash}`
    if (replace) {
      window.history.replaceState(null, "", nextUrl)
      setRouteRunId(runId)
      return
    }

    window.location.hash = nextHash
  }, [])

  const selectedRun = useMemo(() => {
    if (!chainRuns.length) {
      return null
    }

    return chainRuns.find((run) => run.id === routeRunId) ?? chainRuns[0]
  }, [chainRuns, routeRunId])

  const sortedEvents = useMemo(() => {
    if (!selectedRun) {
      return []
    }

    return [...selectedRun.events].sort((left, right) => left.seq - right.seq)
  }, [selectedRun])

  const fullGraphModel = useMemo(
    () => buildGraphModel(session, selectedRun, selectedRun?.events ?? []),
    [session, selectedRun],
  )

  useEffect(() => {
    if (!chainRuns.length) {
      return
    }

    if (!chainRuns.some((run) => run.id === routeRunId)) {
      navigateToRun(chainRuns[0].id, true)
    }
  }, [chainRuns, navigateToRun, routeRunId])

  const {
    isPlaying,
    playbackIndex,
    playbackSpeed,
    pause,
    play,
    stepBackward,
    stepForward,
    setSpeed,
    setPlaybackIndex,
    setRequestedStartNodeId,
  } = usePlayback({
    sortedEvents,
    fullGraphModel,
  })

  const {
    activeEdgeIds,
    activePlaybackNodeId,
    activeStepLabel,
    effectivePlaybackIndex,
    graphModel,
    handoff,
    stepOptions,
  } = useWorkflowModel({
    fullGraphModel,
    isPlaying,
    playbackIndex,
    selectedRun,
    session,
    sortedEvents,
  })

  const handleOpenDetails = useCallback((nodeId) => {
    setSelectedNodeId(nodeId)
    setDetailsNodeId(nodeId)
    setIsDetailsOpen(true)
  }, [])

  const handleCloseDetails = useCallback(() => {
    setIsDetailsOpen(false)
  }, [])

  useEffect(() => {
    if (!activePlaybackNodeId || isRestoringState.current) {
      return
    }

    setSelectedNodeId(activePlaybackNodeId)
    if (isDetailsOpen) {
      setDetailsNodeId(activePlaybackNodeId)
    }
  }, [activePlaybackNodeId, isDetailsOpen])

  const triggerRun = useCallback(
    async (startNodeId) => {
      if (!sortedEvents.length && !serverStatus.can_rerun) {
        return
      }

      setRequestedStartNodeId(startNodeId)

      if (!serverStatus.can_rerun) {
        setPlaybackIndex(-1)
        play()
        return
      }

      pause()
      setPlaybackIndex(-1)
      setServerStatus((current) => ({
        ...current,
        running: true,
        last_error: null,
      }))

      try {
        const nextStatus = await triggerServerRerun()
        setServerStatus(nextStatus)
      } catch (requestError) {
        setServerStatus((current) => ({
          ...current,
          running: false,
          last_error: requestError.message,
        }))
      }
    },
    [
      sortedEvents.length,
      serverStatus.can_rerun,
      play,
      pause,
      setRequestedStartNodeId,
      setPlaybackIndex,
      setServerStatus,
    ],
  )

  const runChainFromNode = useCallback(
    async (nodeId) => {
      await triggerRun(nodeId)
    },
    [triggerRun],
  )

  const buildNodeDataCallback = useCallback(
    (
      currentGraphModel,
      currentRun,
      nodeId,
      currentSelectedNodeId,
      detailsNodeId,
      currentPlaybackNodeId,
      currentIsPlaying,
      onRunChain,
      onOpenDetails,
    ) =>
      buildNodeData(
        currentGraphModel,
        currentRun,
        nodeId,
        currentSelectedNodeId,
        detailsNodeId,
        currentPlaybackNodeId,
        currentIsPlaying,
        onRunChain,
        onOpenDetails,
        NODE_DIMENSIONS,
        shortenPreview,
      ),
    [],
  )

  const activeSavedNodePositions = useMemo(() => {
    if (!selectedRun) {
      return null
    }

    if (savedNodePositionsPipelineId === selectedRun.id) {
      return savedNodePositions
    }

    return getPipelineState(selectedRun.id).nodePositions ?? null
  }, [getPipelineState, savedNodePositions, savedNodePositionsPipelineId, selectedRun])

  const { nodes, edges, onNodesChange, setNodes } = useGraphLayout({
    graphModel,
    selectedRun,
    selectedNodeId,
    detailsNodeId: isDetailsOpen ? detailsNodeId : "",
    canvasMode,
    activePlaybackNodeId,
    activeEdgeIds,
    handoff,
    isPlaying,
    onRunChain: runChainFromNode,
    onOpenDetails: handleOpenDetails,
    buildNodeData: buildNodeDataCallback,
    savedNodePositions: activeSavedNodePositions,
  })

  const persistedNodePositions = useMemo(() => {
    if (!nodes.length) {
      return null
    }

    return Object.fromEntries(
      nodes.map((node) => [
        node.id,
        {
          x: Number(node.position.x.toFixed(2)),
          y: Number(node.position.y.toFixed(2)),
        },
      ]),
    )
  }, [nodes])

  const handleNodeDragStop = useCallback((draggedNode) => {
    const roundedPosition = {
      x: Number(draggedNode.position.x.toFixed(2)),
      y: Number(draggedNode.position.y.toFixed(2)),
    }

    setNodes((currentNodes) => {
      const nextNodes = currentNodes.map((node) =>
        node.id === draggedNode.id
          ? { ...node, position: roundedPosition }
          : node,
      )

      setSavedNodePositions(
        Object.fromEntries(
          nextNodes.map((node) => [
            node.id,
            {
              x: Number(node.position.x.toFixed(2)),
              y: Number(node.position.y.toFixed(2)),
            },
          ]),
        ),
      )
      setSavedNodePositionsPipelineId(selectedRun?.id ?? "")

      return nextNodes
    })
  }, [selectedRun?.id, setNodes])

  const fitViewKey = useMemo(() => {
    if (!selectedRun) {
      return ""
    }

    return `${selectedRun.id}:${graphModel?.session.nodes.length ?? 0}`
  }, [graphModel, selectedRun])

  useEffect(() => {
    if (!selectedRun) {
      return
    }

    // Restore saved state for this pipeline
    isRestoringState.current = true
    const savedState = getPipelineState(selectedRun.id)
    // Don't restore selectedNodeId - let it be driven by activePlaybackNodeId
    // Only restore isDetailsOpen when switching pipelines
    setIsDetailsOpen(savedState.isDetailsOpen)
    setDetailsPanelWidth(
      Math.max(
        MIN_PANEL_WIDTH,
        Math.min(MAX_PANEL_WIDTH, savedState.panelWidth ?? DEFAULT_PANEL_WIDTH),
      ),
    )
    setCanvasMode(savedState.canvasMode ?? DEFAULT_CANVAS_MODE)
    setSavedNodePositions(savedState.nodePositions ?? null)
    setSavedNodePositionsPipelineId(selectedRun.id)
    setPlaybackIndex(savedState.playbackIndex)
    setSpeed(savedState.playbackSpeed)
    setRequestedStartNodeId("")
    // Reset transient node focus when switching pipelines - active playback can repopulate it.
    setSelectedNodeId("")
    setDetailsNodeId("")
    // Use setTimeout to ensure state is set before we start saving again
    setTimeout(() => {
      isRestoringState.current = false
    }, 0)
  }, [getPipelineState, selectedRun?.id, setPlaybackIndex, setSpeed, setRequestedStartNodeId])

  // Save state when it changes
  // Note: Don't save selectedNodeId - it's transient and driven by activePlaybackNodeId
  // Only save isDetailsOpen, playbackIndex, playbackSpeed which are user preferences
  useEffect(() => {
    const nodePositionsToPersist = activeSavedNodePositions ?? persistedNodePositions

    if (!selectedRun || isRestoringState.current || !nodePositionsToPersist) return
    setPipelineState(selectedRun.id, {
      playbackIndex: playbackIndex,
      isDetailsOpen: isDetailsOpen,
      panelWidth: detailsPanelWidth,
      playbackSpeed: playbackSpeed,
      canvasMode,
      nodePositions: nodePositionsToPersist,
    })
  }, [selectedRun?.id, playbackIndex, isDetailsOpen, detailsPanelWidth, playbackSpeed, canvasMode, activeSavedNodePositions, persistedNodePositions, setPipelineState])

  const handleSelectRun = useCallback(
    (runId) => {
      if (!runId || runId === selectedRun?.id) {
        return
      }

      navigateToRun(runId)
    },
    [navigateToRun, selectedRun?.id],
  )

  const handleStepChange = (newIndex) => {
    setPlaybackIndex(newIndex)
    pause()
  }

  const currentNodeData =
    detailsNodeId && graphModel
      ? buildNodeData(
          graphModel,
          selectedRun,
          detailsNodeId,
          selectedNodeId,
          isDetailsOpen ? detailsNodeId : "",
          activePlaybackNodeId,
          isPlaying,
          (nodeId) => {
            void runChainFromNode(nodeId)
          },
          handleOpenDetails,
          NODE_DIMENSIONS,
          shortenPreview,
        )
      : null

  if (error) {
    return <WorkflowEmptyState description={error} title="UI failed to load" />
  }

  if (!graphModel) {
    return (
      <WorkflowEmptyState
        description="Reading the latest captured chain and preparing the execution graph."
        title="Loading workflow"
      />
    )
  }

  return (
    <main
      className={`screen screen--workflow ${isDetailsOpen && detailsNodeId ? "has-details-panel" : ""}`}
      style={{ "--sidebar-width": `${detailsPanelWidth}px` }}
    >
      <Fragment key={selectedRun?.id ?? "empty"}>
        {isDetailsOpen && currentNodeData ? (
          <NodeDetailsPanel
            nodeData={currentNodeData}
            onClose={handleCloseDetails}
            onResize={setDetailsPanelWidth}
            width={detailsPanelWidth}
            minWidth={MIN_PANEL_WIDTH}
            maxWidth={MAX_PANEL_WIDTH}
          />
        ) : null}

        <WorkflowCanvas
          canvasMode={canvasMode}
          edges={edges}
          fitViewKey={fitViewKey}
          nodes={nodes}
          onNodeDragStop={handleNodeDragStop}
          onNodesChange={onNodesChange}
          onNodeSelect={handleOpenDetails}
          onPaneClick={() => {
            if (!isDetailsOpen) {
              setSelectedNodeId("")
            }
          }}
        />

        {sortedEvents.length > 0 ? (
          <PlaybackControls
            canvasMode={canvasMode}
            currentStepLabel={activeStepLabel}
            hasDetailsPanel={isDetailsOpen && Boolean(detailsNodeId)}
            isPlaying={isPlaying}
            onPause={pause}
            onPlay={play}
            onCanvasModeChange={setCanvasMode}
            onRunSelect={handleSelectRun}
            onSkipEnd={stepForward}
            onSkipStart={stepBackward}
            onSpeedChange={setSpeed}
            onStepChange={handleStepChange}
            playbackIndex={effectivePlaybackIndex}
            playbackSpeed={playbackSpeed}
            runs={chainRuns}
            selectedRun={selectedRun}
            stepOptions={stepOptions}
            totalEvents={sortedEvents.length}
          />
        ) : null}
      </Fragment>
    </main>
  )
}

createRoot(document.getElementById("root")).render(<App />)

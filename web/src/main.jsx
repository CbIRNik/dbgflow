import "prismjs"
import "prismjs/components/prism-rust"
import "prismjs/themes/prism-tomorrow.css"
import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from "react"
import { createRoot } from "react-dom/client"
import "./styles.css"
import { usePipelineStore, useUIStore, useSessionStore, usePlaybackStore, DEFAULT_PANEL_WIDTH, MIN_PANEL_WIDTH } from "./store"
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
import {
  buildCanvasNodeData,
  buildGraphModel,
  buildNodeData,
} from "./utils/graphUtils.js"
import { shortenPreview } from "./utils/formatUtils.js"

const RUN_ROUTE_PREFIX = "#/pipelines/"
const PANEL_MAX_WIDTH_RATIO = 0.7

function getMaxPanelWidth(viewportWidth) {
  return Math.max(MIN_PANEL_WIDTH + 120, Math.floor(viewportWidth * PANEL_MAX_WIDTH_RATIO))
}

function clampPanelWidth(width, viewportWidth) {
  return Math.max(MIN_PANEL_WIDTH, Math.min(getMaxPanelWidth(viewportWidth), width))
}

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
  // Session state from zustand
  const { session, serverStatus, error } = useSession()
  const setServerStatus = useSessionStore((state) => state.setServerStatus)
  
  // UI state from zustand
  const selectedNodeId = useUIStore((state) => state.selectedNodeId)
  const detailsNodeId = useUIStore((state) => state.detailsNodeId)
  const detailsPanelWidth = useUIStore((state) => state.detailsPanelWidth)
  const canvasMode = useUIStore((state) => state.canvasMode)
  const viewportWidth = useUIStore((state) => state.viewportWidth)
  const isDetailsOpen = useUIStore((state) => state.isDetailsOpen)
  
  const setSelectedNodeId = useUIStore((state) => state.setSelectedNodeId)
  const setDetailsNodeId = useUIStore((state) => state.setDetailsNodeId)
  const setDetailsPanelWidth = useUIStore((state) => state.setDetailsPanelWidth)
  const setViewportWidth = useUIStore((state) => state.setViewportWidth)
  const openDetails = useUIStore((state) => state.openDetails)
  const dismissDetails = useUIStore((state) => state.dismissDetails)
  
  // Playback state from zustand (via usePlayback hook)
  const setRequestedStartNodeId = usePlaybackStore((state) => state.setRequestedStartNodeId)
  const setPlaybackIndex = usePlaybackStore((state) => state.setPlaybackIndex)
  const pause = usePlaybackStore((state) => state.pause)
  const play = usePlaybackStore((state) => state.play)
  
  // Pipeline store
  const getPipelineState = usePipelineStore((state) => state.getPipelineState)
  const setPipelineState = usePipelineStore((state) => state.setPipelineState)
  
  // Local state
  const [routeRunId, setRouteRunId] = useState(() => readRunRouteId())
  const [savedNodePositions, setSavedNodePositions] = useState(null)
  
  const chainRuns = useMemo(() => deriveChainRuns(session), [session])
  const isRestoringState = useRef(false)
  const maxPanelWidth = useMemo(() => getMaxPanelWidth(viewportWidth), [viewportWidth])

  useEffect(() => {
    if (typeof window === "undefined") {
      return undefined
    }

    const syncRouteRunId = () => {
      setRouteRunId(readRunRouteId())
    }
    const syncViewportWidth = () => {
      setViewportWidth(window.innerWidth)
    }

    window.addEventListener("hashchange", syncRouteRunId)
    window.addEventListener("resize", syncViewportWidth)
    return () => {
      window.removeEventListener("hashchange", syncRouteRunId)
      window.removeEventListener("resize", syncViewportWidth)
    }
  }, [setViewportWidth])

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
    stepBackward,
    stepForward,
    setSpeed,
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
    openDetails(nodeId)
    if (selectedRun?.id) {
      setPipelineState(selectedRun.id, {
        isDetailsOpen: true,
      })
    }
  }, [selectedRun?.id, setPipelineState, openDetails])

  const handleDismissDetails = useCallback(() => {
    dismissDetails()
    if (selectedRun?.id) {
      setPipelineState(selectedRun.id, {
        isDetailsOpen: false,
      })
    }
  }, [selectedRun?.id, setPipelineState, dismissDetails])

  useEffect(() => {
    if (!isDetailsOpen && !selectedNodeId) {
      return undefined
    }

    const handlePointerDown = (event) => {
      if (!(event.target instanceof Element)) {
        return
      }

      if (!event.target.closest(".react-flow")) {
        return
      }

      if (
        event.target.closest(".react-flow__node") ||
        event.target.closest(".details-panel") ||
        event.target.closest(".playback-bar") ||
        event.target.closest("[data-radix-popper-content-wrapper]") ||
        event.target.closest("[role='listbox']") ||
        event.target.closest("[role='menu']")
      ) {
        return
      }

      handleDismissDetails()
    }

    document.addEventListener("pointerdown", handlePointerDown, true)
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true)
    }
  }, [handleDismissDetails, isDetailsOpen, selectedNodeId])

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

  const buildCanvasNodeDataCallback = useCallback(
    (
      currentGraphModel,
      currentRun,
      nodeId,
      currentSelectedNodeId,
      currentPlaybackNodeId,
      onRunChain,
      onOpenDetails,
    ) =>
      buildCanvasNodeData(
        currentGraphModel,
        currentRun,
        nodeId,
        currentSelectedNodeId,
        currentPlaybackNodeId,
        onRunChain,
        onOpenDetails,
        NODE_DIMENSIONS,
      ),
    [],
  )

  // Node focus effect only shows when panel is open
  const visualSelectedNodeId = isDetailsOpen ? selectedNodeId : ""
  const { nodes, edges, nodePositionSnapshot, onNodesChange } = useGraphLayout({
    graphModel,
    selectedRun,
    selectedNodeId: visualSelectedNodeId,
    canvasMode,
    activePlaybackNodeId,
    activeEdgeIds,
    onRunChain: runChainFromNode,
    onOpenDetails: handleOpenDetails,
    buildNodeData: buildCanvasNodeDataCallback,
    savedNodePositions,
  })

  const fitViewKey = useMemo(() => {
    if (!selectedRun) {
      return ""
    }

    return `${selectedRun.id}:${graphModel?.session.nodes.length ?? 0}`
  }, [graphModel, selectedRun])

  const resolvedDetailsNodeId = useMemo(() => {
    if (!isDetailsOpen || !graphModel) {
      return ""
    }

    // Only show details for explicitly selected nodes - never auto-select
    if (detailsNodeId && graphModel.nodeById.has(detailsNodeId)) {
      return detailsNodeId
    }

    // If panel is open but no valid detailsNodeId, close it
    // This handles restoring state where isDetailsOpen was true but node was reset
    return ""
  }, [detailsNodeId, graphModel, isDetailsOpen])

  // Close panel in store if it's open but has no valid node to display
  useEffect(() => {
    if (!selectedRun?.id || isRestoringState.current) {
      return
    }

    if (isDetailsOpen && !resolvedDetailsNodeId) {
      setPipelineState(selectedRun.id, { isDetailsOpen: false })
    }
  }, [isDetailsOpen, resolvedDetailsNodeId, selectedRun?.id, setPipelineState])

  const playbackBarWidth = useMemo(() => {
    if (!isDetailsOpen) {
      return viewportWidth - 24
    }

    return Math.max(320, viewportWidth - detailsPanelWidth - 36)
  }, [detailsPanelWidth, isDetailsOpen, viewportWidth])

  useEffect(() => {
    setDetailsPanelWidth(clampPanelWidth(detailsPanelWidth, viewportWidth))
  }, [viewportWidth, detailsPanelWidth, setDetailsPanelWidth])

  useEffect(() => {
    if (!selectedRun) {
      return
    }

    // Restore saved state for this pipeline
    isRestoringState.current = true
    const savedState = getPipelineState(selectedRun.id)
    setDetailsPanelWidth(
      clampPanelWidth(savedState.panelWidth ?? DEFAULT_PANEL_WIDTH, viewportWidth),
    )
    useUIStore.setState({ canvasMode: savedState.canvasMode ?? "pan-canvas" })
    setSavedNodePositions(savedState.nodePositions ?? null)
    setPlaybackIndex(
      Number.isFinite(savedState.playbackIndex) ? savedState.playbackIndex : -1,
    )
    setSpeed(
      Number.isFinite(savedState.playbackSpeed) ? savedState.playbackSpeed : 1,
    )
    setRequestedStartNodeId("")
    // Reset transient node focus when switching pipelines - active playback can repopulate it.
    setSelectedNodeId("")
    setDetailsNodeId("")
    // Use setTimeout to ensure state is set before we start saving again
    setTimeout(() => {
      isRestoringState.current = false
    }, 0)
  }, [getPipelineState, selectedRun?.id, setPlaybackIndex, setSpeed, setRequestedStartNodeId, viewportWidth, setDetailsPanelWidth, setSelectedNodeId, setDetailsNodeId])

  useEffect(() => {
    if (!selectedRun || isRestoringState.current || !nodePositionSnapshot) {
      return undefined
    }

    const timeoutId = window.setTimeout(() => {
      setPipelineState(selectedRun.id, {
        panelWidth: detailsPanelWidth,
        playbackSpeed,
        canvasMode,
        nodePositions: nodePositionSnapshot,
      })
    }, 80)

    return () => {
      window.clearTimeout(timeoutId)
    }
  }, [
    selectedRun?.id,
    detailsPanelWidth,
    playbackSpeed,
    canvasMode,
    nodePositionSnapshot,
    setPipelineState,
  ])

  useEffect(() => {
    if (!selectedRun || isRestoringState.current) {
      return undefined
    }

    const timeoutId = window.setTimeout(() => {
      setPipelineState(selectedRun.id, {
        playbackIndex,
      })
    }, isPlaying ? 180 : 0)

    return () => {
      window.clearTimeout(timeoutId)
    }
  }, [selectedRun?.id, playbackIndex, isPlaying, setPipelineState])

  const handleSelectRun = useCallback(
    (runId) => {
      if (!runId || runId === selectedRun?.id) {
        return
      }

      navigateToRun(runId)
    },
    [navigateToRun, selectedRun?.id],
  )

  const handleStepChange = useCallback((newIndex) => {
    setPlaybackIndex(newIndex)
    pause()
  }, [setPlaybackIndex, pause])

  const currentNodeData = useMemo(() => {
    if (!resolvedDetailsNodeId || !graphModel) {
      return null
    }

    return buildNodeData(
      graphModel,
      selectedRun,
      resolvedDetailsNodeId,
      selectedNodeId,
      isDetailsOpen ? resolvedDetailsNodeId : "",
      activePlaybackNodeId,
      isPlaying,
      (nodeId) => {
        void runChainFromNode(nodeId)
      },
      handleOpenDetails,
      NODE_DIMENSIONS,
      shortenPreview,
    )
  }, [
    activePlaybackNodeId,
    graphModel,
    handleOpenDetails,
    isDetailsOpen,
    isPlaying,
    resolvedDetailsNodeId,
    runChainFromNode,
    selectedNodeId,
    selectedRun,
  ])

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
      className={`screen screen--workflow ${isDetailsOpen ? "has-details-panel" : ""}`}
      style={{ "--sidebar-width": `${detailsPanelWidth}px` }}
    >
      <Fragment key={selectedRun?.id ?? "empty"}>
        {isDetailsOpen && currentNodeData ? (
          <NodeDetailsPanel
            nodeData={currentNodeData}
            onClose={handleDismissDetails}
            onResize={setDetailsPanelWidth}
            width={detailsPanelWidth}
            minWidth={MIN_PANEL_WIDTH}
            maxWidth={maxPanelWidth}
          />
        ) : null}

        <WorkflowCanvas
          edges={edges}
          fitViewKey={fitViewKey}
          nodes={nodes}
          onNodesChange={onNodesChange}
          onNodeSelect={handleOpenDetails}
          onPaneClick={handleDismissDetails}
        />

        {sortedEvents.length > 0 ? (
          <PlaybackControls
            availableWidth={playbackBarWidth}
            currentStepLabel={activeStepLabel}
            onRunSelect={handleSelectRun}
            onStepChange={handleStepChange}
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

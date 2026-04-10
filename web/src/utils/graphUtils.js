import dagre from "@dagrejs/dagre"
import { NODE_DIMENSIONS } from "./constants.js"
import { buildReturnSignature, isTypePreview } from "./formatUtils.js"

export function buildGraphModel(session, selectedRun, visibleEvents) {
  const baseGraphModel = buildBaseGraphModel(session, selectedRun)
  return buildPlaybackGraphModel(baseGraphModel, visibleEvents)
}

function buildBaseGraphModel(session, selectedRun) {
  if (!session || !selectedRun) {
    return null
  }

  const runNodeIds = selectedRun.nodeIds
  const allRunNodes = session.nodes.filter((node) => runNodeIds.has(node.id))
  const allRunEdges = session.edges.filter(
    (edge) => runNodeIds.has(edge.from) && runNodeIds.has(edge.to),
  )
  const functionNodeById = new Map(
    allRunNodes
      .filter((node) => node.kind === "function")
      .map((node) => [node.id, node]),
  )
  const typeNodes = allRunNodes.filter((node) => node.kind === "type")
  const typeNodeById = new Map(typeNodes.map((node) => [node.id, node]))
  const testLinkByTestNode = new Map()
  const nodeIdByCallId = new Map()
  const latestCallNodeIdByFunctionId = new Map()
  const callOrdinalByFunctionId = new Map()
  const renderNodes = []
  let rootCallNodeId = null

  for (const edge of allRunEdges) {
    if (edge.kind === "test_link") {
      testLinkByTestNode.set(edge.from, edge.to)
    }
  }

  for (const event of selectedRun.events) {
    if (event.kind !== "function_enter" || event.call_id == null) {
      continue
    }

    const baseNode = functionNodeById.get(event.node_id)
    if (!baseNode) {
      continue
    }

    const ordinal = (callOrdinalByFunctionId.get(event.node_id) ?? 0) + 1
    callOrdinalByFunctionId.set(event.node_id, ordinal)

    const callNodeId = `${event.node_id}::call:${event.call_id}`
    nodeIdByCallId.set(event.call_id, callNodeId)
    latestCallNodeIdByFunctionId.set(event.node_id, callNodeId)

    if (!rootCallNodeId && event.parent_call_id == null) {
      rootCallNodeId = callNodeId
    }

    renderNodes.push({
      ...baseNode,
      id: callNodeId,
      label: ordinal > 1 ? `${baseNode.label} #${ordinal}` : baseNode.label,
      functionId: baseNode.id,
      callId: event.call_id,
    })
  }

  const resolvedTestLinkByTestNode = new Map()
  for (const [testNodeId, functionNodeId] of testLinkByTestNode.entries()) {
    resolvedTestLinkByTestNode.set(
      testNodeId,
      latestCallNodeIdByFunctionId.get(functionNodeId) ?? functionNodeId,
    )
  }

  const rootNodeId = rootCallNodeId ?? selectedRun.rootNodeId
  const nodeById = new Map(renderNodes.map((node) => [node.id, node]))
  const renderNodeIds = new Set(renderNodes.map((node) => node.id))
  const renderEdges = buildRenderEdges(
    selectedRun.events,
    allRunEdges,
    renderNodeIds,
    resolvedTestLinkByTestNode,
    nodeIdByCallId,
    rootNodeId,
  )

  const firstSeenSeqByNode = new Map()
  const previousFocusedNodeIdBySeq = new Map()
  let previousFunctionNodeId = null
  for (const event of selectedRun.events) {
    const focusNodeId = focusNodeIdForEvent(
      event,
      resolvedTestLinkByTestNode,
      nodeIdByCallId,
      rootNodeId,
    )

    if (!focusNodeId || !renderNodeIds.has(focusNodeId)) {
      continue
    }

    if (!firstSeenSeqByNode.has(focusNodeId)) {
      firstSeenSeqByNode.set(focusNodeId, event.seq)
    }

    if (event.kind === "function_enter") {
      if (previousFunctionNodeId && previousFunctionNodeId !== focusNodeId) {
        previousFocusedNodeIdBySeq.set(event.seq, previousFunctionNodeId)
      }
      previousFunctionNodeId = focusNodeId
    } else if (String(event.kind).startsWith("test_")) {
      // test links can just use what link they have
      const parentNodeId = resolvedTestLinkByTestNode.get(event.node_id)
      if (parentNodeId) {
        previousFocusedNodeIdBySeq.set(event.seq, parentNodeId)
      }
    }
  }

  return {
    session: {
      ...session,
      title: selectedRun.title,
      nodes: renderNodes,
      edges: renderEdges,
      events: selectedRun.events,
    },
    rootNodeId,
    allNodes: allRunNodes,
    allEdges: allRunEdges,
    nodeById,
    typeNodes,
    typeNodeById,
    renderNodeIds,
    testLinkByTestNode: resolvedTestLinkByTestNode,
    firstSeenSeqByNode,
    nodeIdByCallId,
    previousFocusedNodeIdBySeq,
  }
}

export function buildPlaybackGraphModel(
  baseGraphModel,
  visibleEvents,
  playbackContext = null,
) {
  if (!baseGraphModel) {
    return null
  }

  const eventsByNode = new Map()
  const latestEventByNode = new Map()
  const snapshotByNode = new Map()
  const linkedTestsByNode = new Map()
  const failingTargetIds = new Set()
  const visitedNodeIds = new Set()
  const inputDataByNode = new Map()
  const outputDataByNode = new Map()
  const latestSnapshotByTypeNode = new Map()
  const runningNodeIds = new Set()
  const openFunctionNodesByCallId = new Map()
  const openFunctionCountByNodeId = new Map()
  const openTestCountByNodeId = new Map()

  for (const event of visibleEvents) {
    const focusNodeId = focusNodeIdForEvent(
      event,
      baseGraphModel.testLinkByTestNode,
      baseGraphModel.nodeIdByCallId,
      baseGraphModel.rootNodeId,
    )

    if (focusNodeId && baseGraphModel.renderNodeIds.has(focusNodeId)) {
      pushMapList(eventsByNode, focusNodeId, event)
      latestEventByNode.set(focusNodeId, event)
      visitedNodeIds.add(focusNodeId)
    }

    const callNodeId =
      event.call_id != null
        ? baseGraphModel.nodeIdByCallId.get(event.call_id)
        : null

    if (
      event.kind === "function_enter" &&
      callNodeId &&
      baseGraphModel.renderNodeIds.has(callNodeId)
    ) {
      openFunctionNodesByCallId.set(event.call_id, callNodeId)
      bumpMapCount(openFunctionCountByNodeId, callNodeId, 1)
      runningNodeIds.add(callNodeId)
    }

    if (event.kind === "function_exit" && event.call_id != null) {
      const activeNodeId = openFunctionNodesByCallId.get(event.call_id)
      if (activeNodeId) {
        bumpMapCount(openFunctionCountByNodeId, activeNodeId, -1)
        if (
          !openFunctionCountByNodeId.get(activeNodeId) &&
          !openTestCountByNodeId.get(activeNodeId)
        ) {
          runningNodeIds.delete(activeNodeId)
        }
        openFunctionNodesByCallId.delete(event.call_id)
      }
    }

    if (
      event.kind === "function_enter" &&
      callNodeId &&
      baseGraphModel.renderNodeIds.has(callNodeId)
    ) {
      pushMapItems(
        inputDataByNode,
        callNodeId,
        resolveInputRecords(
          event,
          latestSnapshotByTypeNode,
          baseGraphModel.typeNodes,
        ),
      )
    }

    if (
      event.kind === "function_exit" &&
      callNodeId &&
      baseGraphModel.renderNodeIds.has(callNodeId)
    ) {
      pushMapItems(
        outputDataByNode,
        callNodeId,
        formatExitRecords(event, baseGraphModel.nodeById.get(callNodeId)),
      )
    }

    if (event.kind === "value_snapshot") {
      snapshotByNode.set(event.node_id, event)
      latestSnapshotByTypeNode.set(event.node_id, event)

      const sourceNodeId =
        event.call_id != null
          ? baseGraphModel.nodeIdByCallId.get(event.call_id)
          : null

      if (sourceNodeId && baseGraphModel.renderNodeIds.has(sourceNodeId)) {
        pushMapItems(
          outputDataByNode,
          sourceNodeId,
          formatSnapshotRecords(
            event,
            baseGraphModel.typeNodeById.get(event.node_id),
          ),
        )
      }
    }

    if (String(event.kind).startsWith("test_")) {
      const linkedNodeId = baseGraphModel.testLinkByTestNode.get(event.node_id)
      if (linkedNodeId && baseGraphModel.renderNodeIds.has(linkedNodeId)) {
        pushMapList(linkedTestsByNode, linkedNodeId, event)
        visitedNodeIds.add(linkedNodeId)

        if (event.kind === "test_started") {
          bumpMapCount(openTestCountByNodeId, linkedNodeId, 1)
          runningNodeIds.add(linkedNodeId)
        }

        if (event.kind === "test_failed" || event.kind === "test_passed") {
          bumpMapCount(openTestCountByNodeId, linkedNodeId, -1)
          if (
            !openTestCountByNodeId.get(linkedNodeId) &&
            !openFunctionCountByNodeId.get(linkedNodeId)
          ) {
            runningNodeIds.delete(linkedNodeId)
          }
        }

        if (event.kind === "test_failed") {
          failingTargetIds.add(linkedNodeId)
        }
      }

      if (baseGraphModel.renderNodeIds.has(event.node_id)) {
        pushMapItems(outputDataByNode, event.node_id, formatTestRecords(event))
      }
    }
  }

  // For parallel branches, treat contiguous sibling enters as a single "start" step
  // so all sibling nodes can be highlighted together.
  if (
    playbackContext?.sortedEvents &&
    Number.isInteger(playbackContext.currentIndex)
  ) {
    const parallelNodeIds = collectParallelEnterNodeIdsForStep(
      playbackContext.sortedEvents,
      playbackContext.currentIndex,
      baseGraphModel.renderNodeIds,
      baseGraphModel.nodeIdByCallId,
    )
    for (const nodeId of parallelNodeIds) {
      runningNodeIds.add(nodeId)
    }
  }

  return {
    ...baseGraphModel,
    eventsByNode,
    latestEventByNode,
    snapshotByNode,
    linkedTestsByNode,
    failingTargetIds,
    visitedNodeIds,
    inputDataByNode,
    outputDataByNode,
    runningNodeIds,
  }
}

function collectParallelEnterNodeIdsForStep(
  sortedEvents,
  currentIndex,
  renderNodeIds,
  nodeIdByCallId,
) {
  const nodeIds = new Set()
  const currentEvent = sortedEvents[currentIndex]

  if (
    !currentEvent ||
    currentEvent.kind !== "function_enter" ||
    currentEvent.parent_call_id == null
  ) {
    return nodeIds
  }

  const sharedParentCallId = currentEvent.parent_call_id

  let start = currentIndex
  while (start > 0) {
    const previous = sortedEvents[start - 1]
    if (
      previous?.kind === "function_enter" &&
      previous.parent_call_id === sharedParentCallId
    ) {
      start -= 1
      continue
    }
    break
  }

  let end = currentIndex
  while (end + 1 < sortedEvents.length) {
    const next = sortedEvents[end + 1]
    if (
      next?.kind === "function_enter" &&
      next.parent_call_id === sharedParentCallId
    ) {
      end += 1
      continue
    }
    break
  }

  for (let i = start; i <= end; i += 1) {
    const event = sortedEvents[i]
    if (event?.kind !== "function_enter") {
      continue
    }
    const callNodeId =
      event.call_id != null ? nodeIdByCallId.get(event.call_id) : null
    if (callNodeId && renderNodeIds.has(callNodeId)) {
      nodeIds.add(callNodeId)
    }
  }

  return nodeIds
}

export function applyLayout(nodes, edges, firstSeenSeqByNode) {
  const graph = new dagre.graphlib.Graph()
  graph.setDefaultEdgeLabel(() => ({}))
  graph.setGraph({
    rankdir: "LR",
    nodesep: 48,
    ranksep: 138,
    marginx: 48,
    marginy: 44,
  })
  ;[...nodes]
    .sort((left, right) => compareNodes(left, right, firstSeenSeqByNode))
    .forEach((node) => {
      graph.setNode(node.id, {
        width: NODE_DIMENSIONS.width,
        height: NODE_DIMENSIONS.height,
      })
    })

  edges.forEach((edge) => {
    graph.setEdge(edge.from, edge.to)
  })

  dagre.layout(graph)

  const positions = new Map()
  nodes.forEach((node) => {
    const positioned = graph.node(node.id)
    positions.set(node.id, {
      x: positioned.x - NODE_DIMENSIONS.width / 2,
      y: positioned.y - NODE_DIMENSIONS.height / 2,
    })
  })

  return positions
}

export function resolvePlaybackStartIndex(
  sortedEvents,
  requestedStartNodeId,
  graphModel,
) {
  if (!sortedEvents.length || !graphModel || !requestedStartNodeId) {
    return 0
  }

  const nextIndex = sortedEvents.findIndex((event) => {
    const focusNodeId = focusNodeIdForEvent(
      event,
      graphModel.testLinkByTestNode,
      graphModel.nodeIdByCallId,
      graphModel.rootNodeId,
    )
    return focusNodeId === requestedStartNodeId
  })

  return nextIndex >= 0 ? nextIndex : 0
}

export function focusNodeIdForEvent(
  event,
  testLinkByTestNode,
  nodeIdByCallId,
  rootNodeId = "",
) {
  if (!event) {
    return null
  }

  if (String(event.kind).startsWith("test_")) {
    return testLinkByTestNode.get(event.node_id) ?? event.node_id
  }

  if (event.kind === "value_snapshot") {
    if (event.call_id != null) {
      return nodeIdByCallId.get(event.call_id) ?? rootNodeId ?? event.node_id
    }
    return rootNodeId || event.node_id
  }

  if (event.kind === "function_enter" || event.kind === "function_exit") {
    if (event.call_id != null) {
      return nodeIdByCallId.get(event.call_id) ?? event.node_id
    }
    return event.node_id
  }

  return event.node_id
}

export function activeEdgesForEvent(event, graphModel) {
  const activeEdgeIds = new Set()
  if (!event) {
    return activeEdgeIds
  }

  if (String(event.kind).startsWith("test_")) {
    const linkedNodeId = graphModel.testLinkByTestNode.get(event.node_id)
    if (linkedNodeId) {
      activeEdgeIds.add(`${event.node_id}::${linkedNodeId}`)
    }
    return activeEdgeIds
  }

  if (event.kind === "value_snapshot") {
    return activeEdgeIds
  }

  const previousFocusedNodeId = graphModel.previousFocusedNodeIdBySeq.get(
    event.seq,
  )
  const focusedNodeId = focusNodeIdForEvent(
    event,
    graphModel.testLinkByTestNode,
    graphModel.nodeIdByCallId,
    graphModel.rootNodeId,
  )
  if (
    previousFocusedNodeId &&
    focusedNodeId &&
    previousFocusedNodeId !== focusedNodeId
  ) {
    activeEdgeIds.add(`${previousFocusedNodeId}::${focusedNodeId}`)
  }

  return activeEdgeIds
}

function buildRenderEdges(
  events,
  allRunEdges,
  renderNodeIds,
  testLinkByTestNode,
  nodeIdByCallId,
  rootNodeId,
) {
  const renderEdges = []
  const edgeKeys = new Set()

  for (const event of events) {
    if (event.kind !== "function_enter") {
      continue
    }

    const focusNodeId = focusNodeIdForEvent(
      event,
      testLinkByTestNode,
      nodeIdByCallId,
      rootNodeId,
    )

    if (!focusNodeId || !renderNodeIds.has(focusNodeId)) {
      continue
    }

    const parentNodeId =
      event.parent_call_id != null
        ? nodeIdByCallId.get(event.parent_call_id)
        : null
    if (
      parentNodeId &&
      parentNodeId !== focusNodeId &&
      renderNodeIds.has(parentNodeId)
    ) {
      const edgeKey = `${parentNodeId}::${focusNodeId}::control_flow`
      if (!edgeKeys.has(edgeKey)) {
        renderEdges.push({
          from: parentNodeId,
          to: focusNodeId,
          kind: "control_flow",
          label: null,
        })
        edgeKeys.add(edgeKey)
      }
    }
  }

  for (const edge of allRunEdges) {
    if (
      edge.kind === "test_link" &&
      renderNodeIds.has(edge.from) &&
      renderNodeIds.has(edge.to)
    ) {
      const edgeKey = `${edge.from}::${edge.to}::${edge.kind}`
      if (!edgeKeys.has(edgeKey)) {
        renderEdges.push(edge)
        edgeKeys.add(edgeKey)
      }
    }
  }

  return renderEdges
}

export function buildNodeData(
  graphModel,
  selectedRun,
  nodeId,
  selectedNodeId,
  detailsNodeId,
  activePlaybackNodeId,
  isPlaying,
  onRunChain,
  onOpenDetails,
  nodeDimensions,
  shortenPreviewFn,
) {
  const node = graphModel.nodeById.get(nodeId)
  if (!node) {
    return null
  }
  const events = graphModel.eventsByNode.get(nodeId) ?? []
  const tests = graphModel.linkedTestsByNode.get(nodeId) ?? []
  const latestEvent = graphModel.latestEventByNode.get(nodeId)
  const hasExecuted = graphModel.visitedNodeIds.has(nodeId)
  const isActiveStep = activePlaybackNodeId === nodeId
  const isFailingTarget = graphModel.failingTargetIds.has(nodeId)
  const isRunning = graphModel.runningNodeIds.has(nodeId)
  const status = summarizeStatus(
    node.kind,
    tests,
    latestEvent,
    isFailingTarget,
    isRunning,
    hasExecuted,
  )
  const inputData = graphModel.inputDataByNode.get(nodeId) ?? []
  const outputData = graphModel.outputDataByNode.get(nodeId) ?? []

  return {
    node,
    eventCount: events.length,
    hasExecuted,
    executionState: resolveExecutionState(status),
    isAnimating: isPlaying && isRunning,
    inputData,
    outputData,
    isSelected: selectedNodeId === nodeId,
    isCurrent: isActiveStep,
    isDetailsOpen: detailsNodeId === nodeId,
    canRunChain: graphModel.rootNodeId === nodeId,
    isFailingTarget,
    preview: summarizePreview(inputData, outputData, shortenPreviewFn),
    events,
    status,
    dimensions: nodeDimensions,
    onRunChain,
    onOpenDetails,
  }
}

export function buildCanvasNodeData(
  graphModel,
  selectedRun,
  nodeId,
  selectedNodeId,
  activePlaybackNodeId,
  onRunChain,
  onOpenDetails,
  nodeDimensions,
) {
  const node = graphModel.nodeById.get(nodeId)
  if (!node) {
    return null
  }

  const tests = graphModel.linkedTestsByNode.get(nodeId) ?? []
  const latestEvent = graphModel.latestEventByNode.get(nodeId)
  const hasExecuted = graphModel.visitedNodeIds.has(nodeId)
  const isActiveStep = activePlaybackNodeId === nodeId
  const isFailingTarget = graphModel.failingTargetIds.has(nodeId)
  const isRunning = graphModel.runningNodeIds.has(nodeId)
  const status = summarizeStatus(
    node.kind,
    tests,
    latestEvent,
    isFailingTarget,
    isRunning,
    hasExecuted,
  )

  return {
    node,
    executionState: resolveExecutionState(status),
    isSelected: selectedNodeId === nodeId,
    canRunChain: graphModel.rootNodeId === nodeId,
    dimensions: nodeDimensions,
    onRunChain,
    onOpenDetails,
  }
}

export function summarizeStatus(
  kind,
  tests,
  latestEvent,
  isFailingTarget,
  isRunning,
  hasExecuted,
) {
  if (kind === "test") {
    if (latestEvent?.kind === "test_failed") {
      return { label: "failed", className: "is-danger" }
    }
    if (latestEvent?.kind === "test_passed") {
      return { label: "passed", className: "is-success" }
    }
    if (isRunning) {
      return { label: "running", className: "is-running" }
    }
    if (latestEvent?.kind === "test_started") {
      return { label: "started", className: "is-neutral" }
    }
    return null
  }

  if (isFailingTarget) {
    return { label: "failed", className: "is-danger" }
  }

  if (isRunning) {
    return { label: "running", className: "is-running" }
  }

  if (hasExecuted || tests.some((event) => event.kind === "test_passed")) {
    return { label: "ran", className: "is-success" }
  }

  return null
}

export function summarizePreview(inputData, outputData, shortenPreviewFn) {
  const firstRecord = outputData[0] ?? inputData[0]

  if (!firstRecord?.preview) {
    return null
  }

  return {
    title: firstRecord.title,
    preview: shortenPreviewFn(firstRecord.preview, 240),
  }
}

function compareNodes(left, right, firstSeenSeqByNode) {
  const leftSeq = firstSeenSeqByNode.get(left.id) ?? Number.MAX_SAFE_INTEGER
  const rightSeq = firstSeenSeqByNode.get(right.id) ?? Number.MAX_SAFE_INTEGER

  if (leftSeq !== rightSeq) {
    return leftSeq - rightSeq
  }

  if (left.kind !== right.kind) {
    return left.kind.localeCompare(right.kind)
  }

  return left.label.localeCompare(right.label)
}

function resolveInputRecords(event, latestSnapshotByTypeNode, typeNodes) {
  const records = []

  for (const value of event.values ?? []) {
    const matchedTypeNode = findMatchingTypeNode(value.preview, typeNodes)
    const snapshot = matchedTypeNode
      ? latestSnapshotByTypeNode.get(matchedTypeNode.id)
      : null

    if (snapshot?.values?.length) {
      for (const snapshotValue of snapshot.values) {
        records.push({
          name: value.name,
          preview: snapshotValue.preview,
          title: snapshot.title,
          sourceLabel: matchedTypeNode.label,
        })
      }
      continue
    }

    records.push({
      name: value.name,
      preview: value.preview,
      title: event.title,
      sourceLabel: null,
    })
  }

  return records
}

function formatExitRecords(event, sourceNode) {
  return (event.values ?? []).map((value) => ({
    name: value.name,
    preview:
      sourceNode?.source && isTypePreview(value.preview)
        ? buildReturnSignature(sourceNode.source, value.preview)
        : value.preview,
    title: "return",
    sourceLabel: null,
    language:
      sourceNode?.source && isTypePreview(value.preview) ? "rust" : null,
  }))
}

function formatSnapshotRecords(event, snapshotNode) {
  return (event.values ?? []).map((value) => ({
    name: snapshotNode?.label ?? value.name,
    preview: value.preview,
    title: event.title,
    sourceLabel: snapshotNode?.label ?? null,
  }))
}

function formatTestRecords(event) {
  return (event.values ?? []).map((value) => ({
    name: value.name,
    preview: value.preview,
    title: event.kind.replace("test_", "test "),
    sourceLabel: null,
  }))
}

function findMatchingTypeNode(preview, typeNodes) {
  const normalizedPreview = normalizeTypeText(preview)

  return (
    typeNodes.find((node) =>
      normalizedPreview.includes(normalizeTypeText(node.id)),
    ) ??
    typeNodes.find((node) =>
      normalizedPreview.includes(normalizeTypeText(node.label)),
    ) ??
    null
  )
}

function normalizeTypeText(value) {
  return String(value)
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "")
}

function resolveExecutionState(status) {
  if (status?.className === "is-danger") {
    return "failure"
  }

  if (status?.className === "is-running") {
    return "running"
  }

  if (status?.className === "is-success") {
    return "success"
  }

  return "idle"
}

function pushMapItems(map, key, items) {
  if (!items.length) {
    return
  }

  const list = map.get(key) ?? []
  list.push(...items)
  map.set(key, list)
}

function pushMapList(map, key, value) {
  const list = map.get(key) ?? []
  list.push(value)
  map.set(key, list)
}

function bumpMapCount(map, key, delta) {
  const nextValue = (map.get(key) ?? 0) + delta

  if (nextValue <= 0) {
    map.delete(key)
    return
  }

  map.set(key, nextValue)
}

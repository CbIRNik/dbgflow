import { slugifyLabel, stripEventPrefix } from "./formatUtils.js";

export function deriveChainRuns(session) {
  if (!session) {
    return [];
  }

  const sortedEvents = [...session.events].sort((left, right) => left.seq - right.seq);
  if (!sortedEvents.length) {
    return [buildFallbackRun(session, [], "chain:1:session")];
  }

  const nodeById = new Map(session.nodes.map((node) => [node.id, node]));
  const rootByCallId = new Map();
  const rootRuns = [];
  const rootRunsByCallId = new Map();
  let lastOpenRootIndex = -1;
  let pendingTestEvents = [];

  for (const event of sortedEvents) {
    if (event.kind === "function_enter" && event.call_id != null) {
      const inheritedRootCallId =
        event.parent_call_id != null ? rootByCallId.get(event.parent_call_id) : undefined;
      const rootCallId = inheritedRootCallId ?? event.call_id;
      rootByCallId.set(event.call_id, rootCallId);

      let run = rootRunsByCallId.get(rootCallId);
      if (!run) {
        const rootIndex = rootRuns.length + 1;
        run = {
          id: `chain:${rootIndex}:${slugifyLabel(nodeById.get(event.node_id)?.label ?? event.node_id)}`,
          label: nodeById.get(event.node_id)?.label ?? stripEventPrefix(event.title),
          title: `${session.title} / ${nodeById.get(event.node_id)?.label ?? stripEventPrefix(event.title)}`,
          events: [],
          rootNodeId: event.node_id
        };
        if (pendingTestEvents.length) {
          run.events.push(...pendingTestEvents);
          pendingTestEvents = [];
        }
        rootRuns.push(run);
        rootRunsByCallId.set(rootCallId, run);
      }

      run.events.push(event);
      lastOpenRootIndex = rootRuns.indexOf(run);
      continue;
    }

    if (event.call_id != null) {
      const rootCallId = rootByCallId.get(event.call_id);
      const run = rootCallId != null ? rootRunsByCallId.get(rootCallId) : null;
      if (run) {
        run.events.push(event);
        lastOpenRootIndex = rootRuns.indexOf(run);
        continue;
      }
    }

    if (String(event.kind).startsWith("test_")) {
      if (lastOpenRootIndex >= 0) {
        rootRuns[lastOpenRootIndex].events.push(event);
      } else {
        pendingTestEvents.push(event);
      }
    }
  }

  const runs = rootRuns
    .map((run) => ({
      ...run,
      nodeIds: collectRunNodeIds(run.events, session)
    }))
    .filter((run) =>
      run.events.some((event) =>
        ["function_enter", "function_exit", "value_snapshot", "test_started", "test_failed", "test_passed"].includes(
          String(event.kind)
        )
      )
    );

  if (runs.length > 0) {
    return runs;
  }

  return [buildFallbackRun(session, sortedEvents, "chain:1:session")];
}

export function collectRunNodeIds(events, session) {
  const nodeIds = new Set();
  const testLinksByTestNode = new Map();

  for (const edge of session.edges) {
    if (edge.kind === "test_link") {
      testLinksByTestNode.set(edge.from, edge.to);
    }
  }

  for (const event of events) {
    nodeIds.add(event.node_id);

    if (String(event.kind).startsWith("test_")) {
      const linkedNodeId = testLinksByTestNode.get(event.node_id);
      if (linkedNodeId) {
        nodeIds.add(linkedNodeId);
      }
    }
  }

  return nodeIds;
}

function firstPlayableNodeId(events, session) {
  const testLinkByTestNode = new Map(
    session.edges
      .filter((edge) => edge.kind === "test_link")
      .map((edge) => [edge.from, edge.to])
  );

  for (const event of events) {
    const nodeId = focusNodeIdForEvent(event, testLinkByTestNode);
    const node = session.nodes.find((candidate) => candidate.id === nodeId);
    if (node && node.kind !== "test") {
      return node.id;
    }
  }

  return session.nodes.find((node) => node.kind !== "test")?.id ?? "";
}

function focusNodeIdForEvent(event, testLinkByTestNode) {
  if (!event) {
    return null;
  }
  if (String(event.kind).startsWith("test_")) {
    return testLinkByTestNode.get(event.node_id) ?? event.node_id;
  }
  return event.node_id;
}

function buildFallbackRun(session, events, id) {
  const rootNodeId =
    firstPlayableNodeId(events, session) ||
    session.nodes.find((node) => node.kind !== "test")?.id ||
    session.nodes[0]?.id ||
    "";
  const rootNode = session.nodes.find((node) => node.id === rootNodeId);

  return {
    id,
    label: rootNode?.label ?? "Chain 1",
    title: session.title,
    events,
    nodeIds: events.length ? collectRunNodeIds(events, session) : new Set(session.nodes.map((node) => node.id)),
    rootNodeId
  };
}

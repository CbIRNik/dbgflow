import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import dagre from "@dagrejs/dagre";
import {
  Background,
  Controls,
  Handle,
  MarkerType,
  MiniMap,
  Position,
  ReactFlow
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import "./styles.css";

const KIND_ORDER = {
  test: 0,
  function: 1,
  type: 2
};

const KIND_LABEL = {
  test: "test",
  function: "function",
  type: "data"
};

const EVENT_LABEL = {
  function_enter: "enter",
  function_exit: "exit",
  value_snapshot: "snapshot",
  test_started: "test start",
  test_passed: "test pass",
  test_failed: "test fail"
};

const NODE_WIDTH = 252;
const NODE_HEIGHT = 138;

const LAYOUTS = {
  LR: "left to right",
  TB: "top to bottom"
};

function GraphNode({ data, selected }) {
  const node = data.node;
  const classes = [
    "flow-node",
    `flow-node--${node.kind}`,
    selected ? "is-selected" : "",
    data.isFailingTarget ? "is-failing" : ""
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={classes}>
      <Handle type="target" position={Position.Left} />
      <div className="flow-node__topline">
        <span className="flow-node__kind">{KIND_LABEL[node.kind] ?? node.kind}</span>
        {data.isFailingTarget ? <span className="flow-node__flag">failing</span> : null}
      </div>
      <div className="flow-node__title">{node.label}</div>
      <div className="flow-node__meta">{node.module_path}</div>
      <div className="flow-node__meta">
        {node.file}:{node.line}
      </div>
      <div className="flow-node__stats">
        <span>{data.eventCount} events</span>
        <span>{data.valueCount} values</span>
      </div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

const nodeTypes = {
  inspector: GraphNode
};

function App() {
  const [session, setSession] = useState(null);
  const [error, setError] = useState("");
  const [selectedNodeId, setSelectedNodeId] = useState("");
  const [layoutDirection, setLayoutDirection] = useState("LR");

  useEffect(() => {
    fetch("/session.json")
      .then((response) => {
        if (!response.ok) {
          throw new Error(`Failed to load session: ${response.status}`);
        }
        return response.json();
      })
      .then((payload) => {
        setSession(payload);
      })
      .catch((fetchError) => {
        setError(fetchError.message);
      });
  }, []);

  const derived = useMemo(() => {
    if (!session) {
      return null;
    }

    const eventCountByNode = new Map();
    const valueCountByNode = new Map();
    const nodeById = new Map(session.nodes.map((node) => [node.id, node]));
    const testEvents = session.events.filter((event) => event.kind.startsWith("test_"));
    const failedTestIds = new Set(
      testEvents.filter((event) => event.kind === "test_failed").map((event) => event.node_id)
    );
    const failingTargets = new Set(
      session.edges
        .filter((edge) => failedTestIds.has(edge.from))
        .map((edge) => edge.to)
    );

    for (const event of session.events) {
      eventCountByNode.set(event.node_id, (eventCountByNode.get(event.node_id) ?? 0) + 1);
      valueCountByNode.set(
        event.node_id,
        (valueCountByNode.get(event.node_id) ?? 0) + event.values.length
      );
    }

    const horizontalLayout = layoutDirection === "LR";
    const layout = computeLayout(session.nodes, session.edges, layoutDirection);
    const nodes = session.nodes.map((node) => ({
      id: node.id,
      type: "inspector",
      data: {
        node,
        eventCount: eventCountByNode.get(node.id) ?? 0,
        valueCount: valueCountByNode.get(node.id) ?? 0,
        isFailingTarget: failingTargets.has(node.id)
      },
      position: layout.get(node.id) ?? { x: 0, y: 0 },
      sourcePosition: horizontalLayout ? Position.Right : Position.Bottom,
      targetPosition: horizontalLayout ? Position.Left : Position.Top
    }));

    const edges = session.edges.map((edge, index) => ({
      id: `${edge.from}::${edge.to}::${index}`,
      source: edge.from,
      target: edge.to,
      animated: edge.from.startsWith("test::"),
      type: "smoothstep",
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 18,
        height: 18,
        color: edge.from.startsWith("test::") ? "#f2552c" : "#52606d"
      },
      style: {
        stroke: edge.from.startsWith("test::") ? "#f2552c" : "#52606d",
        strokeWidth: edge.from.startsWith("test::") ? 2.4 : 1.8
      }
    }));

    const selectedId =
      selectedNodeId ||
      [...failingTargets][0] ||
      session.nodes.find((node) => node.kind === "function")?.id ||
      session.nodes[0]?.id ||
      "";

    const selectedNode = nodeById.get(selectedId) ?? null;
    const nodeEvents = session.events.filter((event) => event.node_id === selectedId);
    const failingTests = testEvents
      .map((event) => {
        const linkedNodeId = session.edges.find((edge) => edge.from === event.node_id)?.to ?? "";
        return {
          ...event,
          linkedNodeId,
          linkedNode: nodeById.get(linkedNodeId) ?? null
        };
      })
      .filter((event) => event.kind === "test_failed" || event.kind === "test_passed");

    return {
      nodes,
      edges,
      selectedId,
      selectedNode,
      nodeEvents,
      failingTests,
      counts: {
        nodes: session.nodes.length,
        edges: session.edges.length,
        events: session.events.length
      },
      layoutDirection
    };
  }, [layoutDirection, selectedNodeId, session]);

  useEffect(() => {
    if (derived?.selectedId && derived.selectedId !== selectedNodeId) {
      setSelectedNodeId(derived.selectedId);
    }
  }, [derived, selectedNodeId]);

  if (error) {
    return (
      <main className="screen">
        <section className="empty-state">
          <div className="eyebrow">session</div>
          <h1>UI failed to load</h1>
          <p>{error}</p>
        </section>
      </main>
    );
  }

  if (!derived || !session) {
    return (
      <main className="screen">
        <section className="empty-state">
          <div className="eyebrow">dbgflow</div>
          <h1>Loading graph session</h1>
          <p>Pulling nodes, events, and test state into React Flow.</p>
        </section>
      </main>
    );
  }

  return (
    <main className="screen">
      <section className="hero">
        <div>
          <div className="eyebrow">graph debugger</div>
          <h1>{session.title}</h1>
          <p>
            React Flow view for traced Rust functions, UI-debugged data nodes, and failing tests.
          </p>
        </div>
        <div className="hero__stats">
          <Stat label="Nodes" value={derived.counts.nodes} />
          <Stat label="Edges" value={derived.counts.edges} />
          <Stat label="Events" value={derived.counts.events} />
        </div>
      </section>

      <section className="workspace">
        <section className="flow-shell">
          <div className="flow-shell__header">
            <div>
              <div className="eyebrow">canvas</div>
              <h2>Execution graph</h2>
            </div>
            <div className="flow-legend">
              <div className="layout-toggle">
                {Object.entries(LAYOUTS).map(([direction, label]) => (
                  <button
                    key={direction}
                    className={`layout-toggle__button ${layoutDirection === direction ? "is-active" : ""}`}
                    onClick={() => setLayoutDirection(direction)}
                    type="button"
                  >
                    {label}
                  </button>
                ))}
              </div>
              <span className="flow-legend__item flow-legend__item--function">functions</span>
              <span className="flow-legend__item flow-legend__item--type">data</span>
              <span className="flow-legend__item flow-legend__item--test">tests</span>
            </div>
          </div>
          <div className="flow-stage">
            <ReactFlow
              key={derived.layoutDirection}
              nodes={derived.nodes}
              edges={derived.edges}
              nodeTypes={nodeTypes}
              fitView
              minZoom={0.2}
              maxZoom={1.8}
              onNodeClick={(_, node) => setSelectedNodeId(node.id)}
            >
              <Background color="#314254" gap={24} size={1.1} />
              <MiniMap
                pannable
                zoomable
                nodeColor={(node) => {
                  if (node.data?.isFailingTarget) {
                    return "#f2552c";
                  }
                  if (node.data?.node?.kind === "type") {
                    return "#42c7b4";
                  }
                  if (node.data?.node?.kind === "test") {
                    return "#ffcf5a";
                  }
                  return "#7c8ca2";
                }}
              />
              <Controls />
            </ReactFlow>
          </div>
        </section>

        <aside className="sidebar">
          <section className="panel">
            <div className="panel__header">
              <div className="eyebrow">selection</div>
              <h2>{derived.selectedNode?.label ?? "Node details"}</h2>
            </div>
            {derived.selectedNode ? (
              <div className="panel__body">
                <div className="detail-grid">
                  <Detail label="Kind" value={derived.selectedNode.kind} />
                  <Detail label="Module" value={derived.selectedNode.module_path} />
                  <Detail
                    label="Location"
                    value={`${derived.selectedNode.file}:${derived.selectedNode.line}`}
                  />
                </div>
                <div className="stack-list">
                  {derived.nodeEvents.length ? (
                    derived.nodeEvents.map((event) => (
                      <article className="event-card" key={`${event.seq}-${event.title}`}>
                        <div className="event-card__topline">
                          <strong>
                            #{event.seq} {event.title}
                          </strong>
                          <span>{EVENT_LABEL[event.kind] ?? event.kind}</span>
                        </div>
                        {event.values.map((value, index) => (
                          <div className="event-card__value" key={`${event.seq}-${index}`}>
                            <div className="event-card__value-name">{value.name}</div>
                            <pre>{value.preview}</pre>
                          </div>
                        ))}
                      </article>
                    ))
                  ) : (
                    <div className="empty-card">No events attached to this node yet.</div>
                  )}
                </div>
              </div>
            ) : null}
          </section>

          <section className="panel">
            <div className="panel__header">
              <div className="eyebrow">tests</div>
              <h2>Node failures</h2>
            </div>
            <div className="panel__body stack-list">
              {derived.failingTests.length ? (
                derived.failingTests.map((test) => (
                  <button
                    className={`test-card ${test.kind === "test_failed" ? "is-failed" : "is-passed"}`}
                    key={`${test.seq}-${test.title}`}
                    onClick={() => setSelectedNodeId(test.linkedNodeId)}
                    type="button"
                  >
                    <div className="test-card__topline">
                      <strong>{test.title}</strong>
                      <span>{EVENT_LABEL[test.kind] ?? test.kind}</span>
                    </div>
                    <div className="test-card__target">
                      {test.linkedNode ? `targets ${test.linkedNode.label}` : "no linked node"}
                    </div>
                    {test.values.map((value, index) => (
                      <pre key={`${test.seq}-${index}`}>{value.preview}</pre>
                    ))}
                  </button>
                ))
              ) : (
                <div className="empty-card">No test nodes recorded in this session.</div>
              )}
            </div>
          </section>
        </aside>
      </section>
    </main>
  );
}

function Stat({ label, value }) {
  return (
    <div className="stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function Detail({ label, value }) {
  return (
    <div className="detail">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function computeLayout(nodes, edges, direction) {
  const graph = new dagre.graphlib.Graph();
  graph.setDefaultEdgeLabel(() => ({}));
  graph.setGraph({
    rankdir: direction,
    nodesep: 42,
    ranksep: direction === "LR" ? 110 : 88,
    marginx: 40,
    marginy: 40
  });

  [...nodes].sort(compareNodes).forEach((node) => {
    graph.setNode(node.id, {
      width: NODE_WIDTH,
      height: NODE_HEIGHT
    });
  });

  edges.forEach((edge) => {
    graph.setEdge(edge.from, edge.to);
  });

  dagre.layout(graph);

  const positions = new Map();
  nodes.forEach((node) => {
    const positioned = graph.node(node.id);
    positions.set(node.id, {
      x: positioned.x - NODE_WIDTH / 2,
      y: positioned.y - NODE_HEIGHT / 2
    });
  });

  return positions;
}

function compareNodes(left, right) {
  const orderDiff = (KIND_ORDER[left.kind] ?? 99) - (KIND_ORDER[right.kind] ?? 99);
  if (orderDiff !== 0) {
    return orderDiff;
  }
  return left.label.localeCompare(right.label);
}

createRoot(document.getElementById("root")).render(<App />);

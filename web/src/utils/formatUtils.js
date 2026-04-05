import { EVENT_LABEL } from "./constants.js";

export function shortenPreview(preview, limit) {
  const compact = preview.replace(/\s+/g, " ").trim();
  return compact.length > limit ? `${compact.slice(0, limit)}...` : compact;
}

export function formatEventTitle(event, nodeById, testLinkByTestNode) {
  if (!event) {
    return "";
  }
  const focusNodeId = focusNodeIdForEvent(event, testLinkByTestNode);
  const nodeLabel =
    nodeById.get(focusNodeId)?.label ??
    nodeById.get(event.node_id)?.label ??
    event.title;
  const eventLabel = EVENT_LABEL[event.kind] ?? event.kind;
  return `${eventLabel}: ${nodeLabel}`;
}

export function stripEventPrefix(title) {
  return title.replace(/^(enter|return|panic)\s+/i, "");
}

export function slugifyLabel(label) {
  return label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
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

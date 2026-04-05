import { useEffect, useMemo, useRef, useState } from "react";
import { EVENT_LABEL } from "../utils/constants.js";
import {
  activeEdgesForEvent,
  buildGraphModel,
  focusNodeIdForEvent
} from "../utils/graphUtils.js";

const HANDOFF_DURATION_MS = 520;

export function useWorkflowModel({
  fullGraphModel,
  isPlaying,
  playbackIndex,
  selectedRun,
  session,
  sortedEvents
}) {
  const effectivePlaybackIndex = playbackIndex < 0
    ? (isPlaying ? -1 : sortedEvents.length - 1)
    : playbackIndex;

  const visibleEvents = useMemo(() => {
    if (effectivePlaybackIndex < 0) {
      return [];
    }

    return sortedEvents.slice(0, effectivePlaybackIndex + 1);
  }, [effectivePlaybackIndex, sortedEvents]);

  const activeEvent = effectivePlaybackIndex >= 0
    ? sortedEvents[effectivePlaybackIndex] ?? null
    : null;

  const graphModel = useMemo(
    () => buildGraphModel(session, selectedRun, visibleEvents),
    [session, selectedRun, visibleEvents]
  );

  const activePlaybackNodeId = useMemo(() => {
    if (!graphModel || !activeEvent) {
      return null;
    }

    return focusNodeIdForEvent(
      activeEvent,
      graphModel.testLinkByTestNode,
      graphModel.nodeIdByCallId,
      graphModel.rootNodeId
    );
  }, [graphModel, activeEvent]);

  const activeEdgeIds = useMemo(
    () => (graphModel && activeEvent ? activeEdgesForEvent(activeEvent, graphModel) : new Set()),
    [graphModel, activeEvent]
  );
  const [handoff, setHandoff] = useState(null);
  const previousPlaybackNodeIdRef = useRef(null);

  useEffect(() => {
    if (!graphModel || !activeEvent || !activePlaybackNodeId) {
      previousPlaybackNodeIdRef.current = activePlaybackNodeId;
      setHandoff(null);
      return;
    }

    const previousNodeId = previousPlaybackNodeIdRef.current;
    previousPlaybackNodeIdRef.current = activePlaybackNodeId;

    if (!previousNodeId || previousNodeId === activePlaybackNodeId) {
      return;
    }

    const nextHandoff = {
      edgeIds: activeEdgesForEvent(activeEvent, graphModel),
      from: previousNodeId,
      to: activePlaybackNodeId,
    };

    setHandoff(nextHandoff);
    const timeoutId = window.setTimeout(() => {
      setHandoff((current) => (current === nextHandoff ? null : current));
    }, HANDOFF_DURATION_MS);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [activeEdgeIds, activeEvent, activePlaybackNodeId, graphModel]);

  const activeStepLabel = useMemo(() => {
    if (!graphModel || !activeEvent) {
      return "";
    }

    const focusNodeId = focusNodeIdForEvent(
      activeEvent,
      graphModel.testLinkByTestNode,
      graphModel.nodeIdByCallId,
      graphModel.rootNodeId
    );
    const nodeLabel =
      graphModel.nodeById.get(focusNodeId)?.label ??
      graphModel.nodeById.get(activeEvent.node_id)?.label ??
      activeEvent.title;
    const eventLabel = EVENT_LABEL[activeEvent.kind] ?? activeEvent.kind;

    return `${eventLabel}: ${nodeLabel}`;
  }, [graphModel, activeEvent]);

  const stepOptions = useMemo(() => {
    if (!fullGraphModel) {
      return [];
    }

    return sortedEvents.map((event, index) => {
      const focusNodeId = focusNodeIdForEvent(
        event,
        fullGraphModel.testLinkByTestNode,
        fullGraphModel.nodeIdByCallId,
        fullGraphModel.rootNodeId
      );
      const nodeLabel =
        fullGraphModel.nodeById.get(focusNodeId)?.label ??
        fullGraphModel.nodeById.get(event.node_id)?.label ??
        event.title;

      return {
        value: index,
        label: `${index + 1}. ${EVENT_LABEL[event.kind] ?? event.kind}: ${nodeLabel}`
      };
    });
  }, [fullGraphModel, sortedEvents]);

  return {
    activeEdgeIds,
    activePlaybackNodeId,
    activeStepLabel,
    effectivePlaybackIndex,
    graphModel,
    handoff,
    stepOptions
  };
}

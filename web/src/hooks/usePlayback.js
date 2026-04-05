import { useCallback, useEffect, useRef, useState } from "react"
import { BASE_PLAYBACK_INTERVAL_MS } from "../utils/constants.js"
import { resolvePlaybackStartIndex } from "../utils/graphUtils.js"

export function usePlayback({
  sortedEvents,
  fullGraphModel,
  onPlaybackComplete,
}) {
  const [isPlaying, setIsPlaying] = useState(false)
  const [playbackIndex, setPlaybackIndex] = useState(() => {
    if (typeof window === "undefined") return -1
    return parseInt(
      localStorage.getItem(
        "dbgflow_playback_index_" +
          fullGraphModel?.session?.events?.[0]?.node_id,
      ) || "-1",
      10,
    )
  })
  const [playbackSpeed, setPlaybackSpeed] = useState(1)
  const [requestedStartNodeId, setRequestedStartNodeId] = useState("")
  const animationFrameRef = useRef(0)
  const lastTimestampRef = useRef(null)
  const accumulatedMsRef = useRef(0)
  const playbackIndexRef = useRef(playbackIndex)

  const updatePlaybackIndex = useCallback(
    (nextIndex) => {
      playbackIndexRef.current = nextIndex
      setPlaybackIndex(nextIndex)
      if (typeof window !== "undefined") {
        const runId = fullGraphModel?.session?.events?.[0]?.node_id || "global"
        localStorage.setItem(
          "dbgflow_playback_index_" + runId,
          nextIndex.toString(),
        )
      }
    },
    [fullGraphModel],
  )

  const resetTimelineClock = useCallback(() => {
    if (animationFrameRef.current) {
      window.cancelAnimationFrame(animationFrameRef.current)
      animationFrameRef.current = 0
    }
    lastTimestampRef.current = null
    accumulatedMsRef.current = 0
  }, [])

  useEffect(() => {
    playbackIndexRef.current = playbackIndex
  }, [playbackIndex])

  useEffect(() => {
    if (isPlaying) {
      return
    }

    resetTimelineClock()

    if (!sortedEvents.length) {
      return
    }

    if (playbackIndex < 0 || playbackIndex >= sortedEvents.length) {
      updatePlaybackIndex(sortedEvents.length - 1)
    }
  }, [isPlaying, playbackIndex, sortedEvents])

  useEffect(() => {
    if (!isPlaying || !sortedEvents.length) {
      resetTimelineClock()
      return
    }

    if (playbackIndexRef.current < 0) {
      updatePlaybackIndex(
        resolvePlaybackStartIndex(
          sortedEvents,
          requestedStartNodeId,
          fullGraphModel,
        ),
      )
    }

    const stepIntervalMs = Math.max(
      120,
      BASE_PLAYBACK_INTERVAL_MS / playbackSpeed,
    )

    const stopPlayback = () => {
      resetTimelineClock()
      setIsPlaying(false)
      setRequestedStartNodeId("")
      onPlaybackComplete?.()
    }

    const tick = (timestamp) => {
      if (lastTimestampRef.current == null) {
        lastTimestampRef.current = timestamp
        animationFrameRef.current = window.requestAnimationFrame(tick)
        return
      }

      accumulatedMsRef.current += timestamp - lastTimestampRef.current
      lastTimestampRef.current = timestamp

      if (accumulatedMsRef.current >= stepIntervalMs) {
        const availableSteps =
          sortedEvents.length - 1 - playbackIndexRef.current
        const requestedSteps = Math.floor(
          accumulatedMsRef.current / stepIntervalMs,
        )
        const nextSteps = Math.min(availableSteps, requestedSteps)

        if (nextSteps > 0) {
          accumulatedMsRef.current -= nextSteps * stepIntervalMs
          updatePlaybackIndex(playbackIndexRef.current + nextSteps)
        }
      }

      if (playbackIndexRef.current >= sortedEvents.length - 1) {
        stopPlayback()
        return
      }

      animationFrameRef.current = window.requestAnimationFrame(tick)
    }

    animationFrameRef.current = window.requestAnimationFrame(tick)

    return () => {
      resetTimelineClock()
    }
  }, [
    isPlaying,
    sortedEvents,
    requestedStartNodeId,
    fullGraphModel,
    playbackSpeed,
    onPlaybackComplete,
  ])

  const play = useCallback(() => {
    if (
      !requestedStartNodeId &&
      playbackIndexRef.current >= sortedEvents.length - 1
    ) {
      updatePlaybackIndex(0)
    }
    setIsPlaying(true)
  }, [requestedStartNodeId, sortedEvents.length, updatePlaybackIndex])

  const pause = useCallback(() => {
    resetTimelineClock()
    setIsPlaying(false)
  }, [resetTimelineClock])

  const stepBackward = useCallback(() => {
    updatePlaybackIndex(Math.max(-1, playbackIndexRef.current - 1))
    setIsPlaying(false)
  }, [updatePlaybackIndex])

  const stepForward = useCallback(() => {
    updatePlaybackIndex(
      Math.min(sortedEvents.length - 1, playbackIndexRef.current + 1),
    )
    setIsPlaying(false)
  }, [sortedEvents.length, updatePlaybackIndex])

  const setSpeed = useCallback((speed) => {
    setPlaybackSpeed(speed)
  }, [])

  return {
    isPlaying,
    playbackIndex,
    playbackSpeed,
    pause,
    play,
    setIsPlaying,
    setPlaybackIndex: updatePlaybackIndex,
    setRequestedStartNodeId,
    setSpeed,
    stepForward,
    stepBackward,
  }
}

import { useEffect, useRef } from "react"
import { BASE_PLAYBACK_INTERVAL_MS } from "../utils/constants.js"
import { resolvePlaybackStartIndex } from "../utils/graphUtils.js"
import { usePlaybackStore } from "../store/playbackStore.js"

export function usePlayback({
  sortedEvents,
  fullGraphModel,
  onPlaybackComplete,
}) {
  // Get state and actions from store
  const isPlaying = usePlaybackStore((state) => state.isPlaying)
  const playbackIndex = usePlaybackStore((state) => state.playbackIndex)
  const playbackSpeed = usePlaybackStore((state) => state.playbackSpeed)
  const requestedStartNodeId = usePlaybackStore((state) => state.requestedStartNodeId)
  
  const setIsPlaying = usePlaybackStore((state) => state.setIsPlaying)
  const setPlaybackIndex = usePlaybackStore((state) => state.setPlaybackIndex)
  const setRequestedStartNodeId = usePlaybackStore((state) => state.setRequestedStartNodeId)
  const pause = usePlaybackStore((state) => state.pause)
  const play = usePlaybackStore((state) => state.play)
  const stepForward = usePlaybackStore((state) => state.stepForward)
  const stepBackward = usePlaybackStore((state) => state.stepBackward)
  const setSpeed = usePlaybackStore((state) => state.setPlaybackSpeed)

  const animationFrameRef = useRef(0)
  const lastTimestampRef = useRef(null)
  const accumulatedMsRef = useRef(0)
  const playbackIndexRef = useRef(playbackIndex)

  const resetTimelineClock = () => {
    if (animationFrameRef.current) {
      window.cancelAnimationFrame(animationFrameRef.current)
      animationFrameRef.current = 0
    }
    lastTimestampRef.current = null
    accumulatedMsRef.current = 0
  }

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
      setPlaybackIndex(sortedEvents.length - 1)
    }
  }, [isPlaying, playbackIndex, sortedEvents.length, setPlaybackIndex])

  useEffect(() => {
    if (!isPlaying || !sortedEvents.length) {
      resetTimelineClock()
      return
    }

    if (playbackIndexRef.current < 0) {
      setPlaybackIndex(
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
          setPlaybackIndex(playbackIndexRef.current + nextSteps)
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
    sortedEvents.length,
    requestedStartNodeId,
    fullGraphModel,
    playbackSpeed,
    onPlaybackComplete,
    setPlaybackIndex,
    setIsPlaying,
    setRequestedStartNodeId,
  ])

  // Wrap stepForward and stepBackward to include maxIndex
  const wrappedStepForward = () => {
    stepForward(sortedEvents.length - 1)
  }

  return {
    isPlaying,
    playbackIndex,
    playbackSpeed,
    pause,
    play,
    setIsPlaying,
    setPlaybackIndex,
    setRequestedStartNodeId,
    setSpeed,
    stepForward: wrappedStepForward,
    stepBackward,
  }
}

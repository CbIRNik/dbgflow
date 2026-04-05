import { create } from 'zustand'
import { persist } from 'zustand/middleware'

const DEFAULT_PIPELINE_STATE = {
  playbackIndex: -1,
  isDetailsOpen: false,
  playbackSpeed: 1,
  panelWidth: 420,
  canvasMode: "pan-canvas",
  nodePositions: null,
}

// Store for per-pipeline state
export const usePipelineStore = create(
  persist(
    (set, get) => ({
      // Map of pipelineId -> { playbackIndex, isDetailsOpen, playbackSpeed, panelWidth, canvasMode, nodePositions }
      // Note: selectedNodeId is NOT persisted - it's transient and driven by activePlaybackNodeId
      pipelineStates: {},

      // Get state for a specific pipeline
      getPipelineState: (pipelineId) => {
        const states = get().pipelineStates
        return {
          ...DEFAULT_PIPELINE_STATE,
          ...(states[pipelineId] || {}),
        }
      },

      // Update state for a specific pipeline
      setPipelineState: (pipelineId, updates) => {
        const current = get().getPipelineState(pipelineId)
        set((state) => ({
          pipelineStates: {
            ...state.pipelineStates,
            [pipelineId]: {
              ...current,
              ...updates,
            },
          },
        }))
      },

      // Set playback index for current pipeline
      setPlaybackIndex: (pipelineId, playbackIndex) => {
        const current = get().getPipelineState(pipelineId)
        get().setPipelineState(pipelineId, { ...current, playbackIndex })
      },

      // Set details panel state for current pipeline
      setIsDetailsOpen: (pipelineId, isDetailsOpen) => {
        const current = get().getPipelineState(pipelineId)
        get().setPipelineState(pipelineId, { ...current, isDetailsOpen })
      },

      // Set playback speed for current pipeline
      setPlaybackSpeed: (pipelineId, playbackSpeed) => {
        const current = get().getPipelineState(pipelineId)
        get().setPipelineState(pipelineId, { ...current, playbackSpeed })
      },
    }),
    {
      name: 'dbgflow-pipeline-states',
    }
  )
)

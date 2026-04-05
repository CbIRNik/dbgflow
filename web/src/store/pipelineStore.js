import { create } from 'zustand'
import { persist } from 'zustand/middleware'

// Store for per-pipeline state
export const usePipelineStore = create(
  persist(
    (set, get) => ({
      // Map of pipelineId -> { playbackIndex, isDetailsOpen, playbackSpeed }
      pipelineStates: {},

      // Get state for a specific pipeline
      getPipelineState: (pipelineId) => {
        const states = get().pipelineStates
        return states[pipelineId] || { playbackIndex: -1, isDetailsOpen: false, playbackSpeed: 1 }
      },

      // Update state for a specific pipeline
      setPipelineState: (pipelineId, updates) => {
        set((state) => ({
          pipelineStates: {
            ...state.pipelineStates,
            [pipelineId]: {
              ...state.pipelineStates[pipelineId],
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

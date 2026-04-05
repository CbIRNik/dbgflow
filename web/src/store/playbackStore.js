import { create } from 'zustand'

export const usePlaybackStore = create((set, get) => ({
  isPlaying: false,
  playbackIndex: -1,
  playbackSpeed: 1,
  requestedStartNodeId: "",

  // Actions
  play: () => {
    const state = get()
    // Reset to 0 if at the end and no specific start node requested
    if (!state.requestedStartNodeId && state.playbackIndex >= (state._maxIndex ?? Infinity)) {
      set({ playbackIndex: 0, isPlaying: true })
    } else {
      set({ isPlaying: true })
    }
  },
  
  pause: () => set({ isPlaying: false }),
  
  setPlaybackIndex: (index) => set({ playbackIndex: index }),
  
  setPlaybackSpeed: (speed) => set({ playbackSpeed: speed }),
  
  setRequestedStartNodeId: (nodeId) => set({ requestedStartNodeId: nodeId }),
  
  setIsPlaying: (playing) => set({ isPlaying: playing }),

  stepForward: (maxIndex) => {
    const current = get().playbackIndex
    set({ 
      playbackIndex: Math.min(maxIndex, current + 1),
      isPlaying: false,
      _maxIndex: maxIndex
    })
  },

  stepBackward: () => {
    const current = get().playbackIndex
    set({ 
      playbackIndex: Math.max(0, current - 1),
      isPlaying: false 
    })
  },

  reset: () => set({
    isPlaying: false,
    playbackIndex: -1,
    playbackSpeed: 1,
    requestedStartNodeId: "",
  }),
}))

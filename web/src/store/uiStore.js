import { create } from 'zustand'

const DEFAULT_PANEL_WIDTH = 500
const MIN_PANEL_WIDTH = 400
const DEFAULT_CANVAS_MODE = "pan-canvas"

export const useUIStore = create((set, get) => ({
  // UI state
  selectedNodeId: "",
  detailsNodeId: "",
  detailsPanelWidth: DEFAULT_PANEL_WIDTH,
  canvasMode: DEFAULT_CANVAS_MODE,
  viewportWidth: typeof window !== 'undefined' ? window.innerWidth : 1440,
  
  // Derived state helper
  isDetailsOpen: false,

  // Actions
  setSelectedNodeId: (nodeId) => set({ selectedNodeId: nodeId }),
  setDetailsNodeId: (nodeId) => set({ detailsNodeId: nodeId }),
  setDetailsPanelWidth: (width) => set({ detailsPanelWidth: width }),
  setCanvasMode: (mode) => set({ canvasMode: mode }),
  setViewportWidth: (width) => set({ viewportWidth: width }),
  setIsDetailsOpen: (isOpen) => set({ isDetailsOpen: isOpen }),

  openDetails: (nodeId) => set({
    selectedNodeId: nodeId,
    detailsNodeId: nodeId,
    isDetailsOpen: true,
  }),

  dismissDetails: () => set({
    selectedNodeId: "",
    detailsNodeId: "",
    isDetailsOpen: false,
  }),

  reset: () => set({
    selectedNodeId: "",
    detailsNodeId: "",
    detailsPanelWidth: DEFAULT_PANEL_WIDTH,
    canvasMode: DEFAULT_CANVAS_MODE,
    isDetailsOpen: false,
  }),
}))

export { DEFAULT_PANEL_WIDTH, MIN_PANEL_WIDTH }

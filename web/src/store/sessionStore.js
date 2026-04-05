import { create } from 'zustand'
import { DEFAULT_SERVER_STATUS } from '../utils/constants.js'

export const useSessionStore = create((set) => ({
  session: null,
  serverStatus: DEFAULT_SERVER_STATUS,
  error: "",

  // Actions
  setSession: (session) => set({ session }),
  setServerStatus: (statusOrUpdater) => set((state) => ({
    serverStatus: typeof statusOrUpdater === 'function' 
      ? statusOrUpdater(state.serverStatus)
      : statusOrUpdater
  })),
  setError: (error) => set({ error }),
}))

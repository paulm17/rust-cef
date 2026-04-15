import { create } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'

interface PersistentState {
  persistentCount: number
  incrementPersistentCount: () => void
  decrementPersistentCount: () => void
  resetPersistentCount: () => void
}

export const usePersistentStore = create<PersistentState>()(
  persist(
    (set) => ({
      persistentCount: 0,
      incrementPersistentCount: () => set((state) => ({ persistentCount: state.persistentCount + 1 })),
      decrementPersistentCount: () => set((state) => ({ persistentCount: state.persistentCount - 1 })),
      resetPersistentCount: () => set({ persistentCount: 0 }),
    }),
    {
      name: 'rust-cef-persistent-storage',
      storage: createJSONStorage(() => localStorage),
    },
  ),
)
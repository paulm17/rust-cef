import { usePersistentStore } from './store'

export function PersistentCounter() {
  const persistentCount = usePersistentStore((state) => state.persistentCount)
  const incrementPersistentCount = usePersistentStore((state) => state.incrementPersistentCount)
  const decrementPersistentCount = usePersistentStore((state) => state.decrementPersistentCount)
  const resetPersistentCount = usePersistentStore((state) => state.resetPersistentCount)

  return (
    <div style={{ marginTop: '16px', padding: '16px', background: '#1a1a2e', borderRadius: '8px' }}>
      <h3 style={{ margin: '0 0 12px 0', textAlign: 'center' }}>Persistent Counter (localStorage)</h3>
      <div style={{ display: 'flex', gap: '8px', justifyContent: 'center', alignItems: 'center' }}>
        <button onClick={decrementPersistentCount}>-</button>
        <span style={{ fontSize: '24px', minWidth: '60px', textAlign: 'center' }}>
          {persistentCount}
        </span>
        <button onClick={incrementPersistentCount}>+</button>
        <button onClick={resetPersistentCount} style={{ marginLeft: '8px' }}>Reset</button>
      </div>
      <p style={{ fontSize: '12px', color: '#888', textAlign: 'center', marginTop: '8px', marginBottom: '0' }}>
        This counter should persist across app restarts
      </p>
    </div>
  )
}
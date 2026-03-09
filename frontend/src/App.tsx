import { useState, useEffect } from 'react'
import reactLogo from './assets/react.svg'
import viteLogo from '/vite.svg'
import './App.css'
import { invoke, RustFileSystem, RustWindow, RustOS } from './rust-api'
import type { ShowMessageDialogRequest } from './types'

interface AppInfo {
  name: string;
  version: string;
  platform: string;
  arch: string;
}

function App() {
  const [count, setCount] = useState(0)
  const [greeting, setGreeting] = useState('')
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null)
  const [error, setError] = useState('')
  const [logs, setLogs] = useState<string[]>([])
  const [badgeCount, setBadgeCount] = useState(0)
  const [transparent, setTransparent] = useState(false)
  const [alwaysOnTop, setAlwaysOnTop] = useState(false)
  const [frameless, setFrameless] = useState(false)
  const [kiosk, setKiosk] = useState(false)

  const addLog = (msg: string) => setLogs(prev => [...prev, msg])

  useEffect(() => {
    const hasCef = typeof (window as any).cefQuery === 'function';
    addLog(hasCef ? '✓ CEF bridge detected' : '⚠️ Dev mode (no CEF)');
  }, [])

  const handleGreet = async () => {
    setError('');
    addLog(`→ invoke('greet', { name: 'Paul' })`);
    try {
      const result = await invoke<{ message: string }>('greet', { name: 'Paul' });
      setGreeting(result.message);
      addLog(`✓ ${result.message}`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleAppInfo = async () => {
    setError('');
    addLog(`→ invoke('get_app_info')`);
    try {
      const result = await invoke<AppInfo>('get_app_info');
      setAppInfo(result);
      addLog(`✓ ${result.name} v${result.version} (${result.platform}/${result.arch})`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleEcho = async () => {
    setError('');
    const payload = { count, timestamp: Date.now() };
    addLog(`→ invoke('echo', ${JSON.stringify(payload)})`);
    try {
      const result = await invoke<typeof payload>('echo', payload);
      addLog(`✓ echo: ${JSON.stringify(result)}`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleBadCommand = async () => {
    setError('');
    addLog(`→ invoke('nonexistent_command')`);
    try {
      await invoke('nonexistent_command');
      addLog('✓ (unexpected success)');
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleCreateWindow = async () => {
    setError('');
    addLog(`→ invoke('create_window')`);
    try {
      const result = await RustWindow.create({
        url: 'data:text/html,<html><body style="font-family:sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#242424;color:white;"><h1>Hello Window!</h1></body></html>',
        title: `Secondary Window ${Date.now()}`,
        width: 600,
        height: 400
      });
      addLog(`✓ Window ${result.status}`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleUpdateBadge = async (increment: number) => {
    const newCount = Math.max(0, badgeCount + increment);
    setBadgeCount(newCount);
    addLog(`→ invoke('set_badge_count', ${newCount})`);
    try {
      await RustOS.setBadgeCount(newCount);
      addLog(`✓ Badge updated: ${newCount}`);
    } catch (e) {
      addLog(`✗ ${(e as Error).message}`);
    }
  };

  const toggleConfig = async (key: 'transparent' | 'always_on_top' | 'frameless' | 'kiosk') => {
    try {
      if (key === 'transparent') {
        // Rust uses snake_case, JS UI expects it as passed
        await RustWindow.setConfig({ transparent: !transparent });
        setTransparent(!transparent);
      } else if (key === 'always_on_top') {
        await RustWindow.setConfig({ always_on_top: !alwaysOnTop });
        setAlwaysOnTop(!alwaysOnTop);
      } else if (key === 'frameless') {
        await RustWindow.setConfig({ frameless: !frameless });
        setFrameless(!frameless);
      } else if (key === 'kiosk') {
        await RustWindow.setConfig({ kiosk: !kiosk });
        setKiosk(!kiosk);
      }
      addLog(`✓ Toggled ${key}`);
    } catch (e) {
      addLog(`✗ ${(e as Error).message}`);
    }
  };


  const handleShowDialog = async (level: ShowMessageDialogRequest['level']) => {
    setError('');
    addLog(`→ show_message_dialog('${level}')`);
    try {
      const result = await invoke<boolean>('show_message_dialog', {
        level,
        title: `${level.charAt(0).toUpperCase() + level.slice(1)} Dialog`,
        message: `This is a test ${level} message from the frontend.`
      });
      addLog(`✓ Result: ${result}`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  // File System Handlers
  const [fileContent, setFileContent] = useState('');
  const [currentFile, setCurrentFile] = useState('');

  const handleOpenFile = async () => {
    setError('');
    addLog('→ show_open_dialog()');
    try {
      const selected = await RustFileSystem.showOpenDialog({ filters: ['txt', 'md', 'rs', 'js', 'ts'] });
      if (selected) {
        // If multiple is false (default), it returns a string. If true, string[] (but we didn't set multiple: true)
        const path = Array.isArray(selected) ? selected[0] : selected;
        addLog(`✓ Selected: ${path}`);
        setCurrentFile(path);

        const content = await RustFileSystem.readFile(path);
        setFileContent(content);
        addLog(`✓ Read ${content.length} bytes`);
      } else {
        addLog('• Cancelled');
      }
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleSaveFile = async () => {
    setError('');
    addLog('→ show_save_dialog()');
    try {
      const path = await RustFileSystem.showSaveDialog({
        filters: ['txt'],
        filename: currentFile ? undefined : 'new_file.txt'
      });

      if (path) {
        addLog(`✓ Saving to: ${path}`);
        await RustFileSystem.writeFile(path, fileContent);
        setCurrentFile(path);
        addLog('✓ Saved successfully');
      } else {
        addLog('• Cancelled');
      }
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handlePickFolder = async () => {
    setError('');
    addLog('→ show_pick_folder_dialog()');
    try {
      const selected = await RustFileSystem.showPickFolderDialog();
      if (selected) {
        const path = Array.isArray(selected) ? selected[0] : selected;
        addLog(`✓ Selected Folder: ${path}`);

        // List directory contents
        const entries = await RustFileSystem.readDir(path);
        addLog(`✓ Found ${entries.length} items`);
        const fileNames = entries.map(e => e.name).join(', ');
        setFileContent(`Directory Listing:\n${fileNames}`);
        setCurrentFile(path);
      } else {
        addLog('• Cancelled');
      }
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  return (
    <>
      <div>
        <a href="https://vite.dev" target="_blank">
          <img src={viteLogo} className="logo" alt="Vite logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <h1>Rust + CEF Shell</h1>

      <div className="card">
        <div style={{ display: 'flex', gap: '8px', justifyContent: 'center', flexWrap: 'wrap' }}>
          <button onClick={() => setCount(c => c + 1)}>count is {count}</button>
          <button onClick={handleGreet}>Greet</button>
          <button onClick={handleAppInfo}>App Info</button>
          <button onClick={handleEcho}>Echo</button>
          <button onClick={handleCreateWindow}>New Window</button>
          <button onClick={handleBadCommand} style={{ opacity: 0.7 }}>Bad Command</button>
        </div>

        <div style={{ display: 'flex', gap: '8px', justifyContent: 'center', marginTop: '10px' }}>
          <button onClick={() => handleShowDialog('info')}>Info</button>
          <button onClick={() => handleShowDialog('warning')}>Warning</button>
          <button onClick={() => handleShowDialog('error')}>Error</button>
          <button onClick={() => handleShowDialog('confirm')}>Confirm</button>
        </div>

        {greeting && (
          <p style={{ color: '#61dafb', fontSize: '18px' }}>{greeting}</p>
        )}

        {appInfo && (
          <div style={{ textAlign: 'left', margin: '12px auto', maxWidth: '300px', background: '#1a1a2e', padding: '12px', borderRadius: '8px' }}>
            <div><strong>{appInfo.name}</strong></div>
            <div>Version: {appInfo.version}</div>
            <div>Platform: {appInfo.platform} / {appInfo.arch}</div>
          </div>
        )}

        {error && (
          <p style={{ color: '#ff6b6b' }}>Error: {error}</p>
        )}

        {/* File System UI */}
        <div style={{ padding: '20px', borderTop: '1px solid #333', marginTop: '20px' }}>
          <h3>File System</h3>
          <div style={{ display: 'flex', gap: '8px', justifyContent: 'center', marginBottom: '10px' }}>
            <button onClick={handleOpenFile}>Open File</button>
            <button onClick={handleSaveFile}>Save Text</button>
            <button onClick={handlePickFolder}>Pick Folder</button>
          </div>
          {currentFile && <div style={{ fontSize: '12px', color: '#888' }}>Current: {currentFile}</div>}
          <textarea
            value={fileContent}
            onChange={(e) => setFileContent(e.target.value)}
            style={{ width: '100%', height: '100px', background: '#1a1a2e', color: '#fff', padding: '8px', borderRadius: '4px', border: '1px solid #333' }}
            placeholder="File content..."
          />
        </div>

        {/* OS Integration & Window Settings */}
        <div style={{ padding: '20px', borderTop: '1px solid #333', marginTop: '20px' }}>
          <h3>Window Modes & OS Badges</h3>
          <div style={{ display: 'flex', gap: '8px', justifyContent: 'center', marginBottom: '10px' }}>
            <button onClick={() => toggleConfig('frameless')}>
              {frameless ? 'Restore Frame' : 'Make Frameless'}
            </button>
            <button onClick={() => toggleConfig('transparent')}>
              {transparent ? 'Disable Transparent' : 'Make Transparent'}
            </button>
            <button onClick={() => toggleConfig('always_on_top')}>
              {alwaysOnTop ? 'Disable Always on Top' : 'Always on Top'}
            </button>
            <button onClick={() => toggleConfig('kiosk')}>
              {kiosk ? 'Exit Kiosk' : 'Kiosk Mode'}
            </button>
          </div>
          <div style={{ display: 'flex', gap: '8px', justifyContent: 'center', marginBottom: '10px' }}>
            <span>Badge Count: {badgeCount}</span>
            <button onClick={() => handleUpdateBadge(-1)}>-1</button>
            <button onClick={() => handleUpdateBadge(1)}>+1</button>
            <button onClick={() => handleUpdateBadge(-badgeCount)}>Clear</button>
          </div>
        </div>

        <div style={{
          textAlign: 'left', marginTop: '20px', background: '#1a1a2e',
          padding: '10px', borderRadius: '8px', maxHeight: '160px', overflowY: 'auto',
          fontFamily: 'monospace', fontSize: '11px'
        }}>
          {logs.map((log, i) => <div key={i}>{log}</div>)}
        </div>
      </div>
    </>
  )
}

export default App

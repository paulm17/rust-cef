import { useState, useEffect } from 'react'
import reactLogo from './assets/react.svg'
import viteLogo from '/vite.svg'
import './App.css'
import { invoke, RustClipboard, RustEvents, RustFileSystem, RustWindow, RustOS } from './rust-api'
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
  const [shortcutId, setShortcutId] = useState('demo-shortcut')
  const [shortcutAccelerator, setShortcutAccelerator] = useState('CmdOrCtrl+Shift+Y')
  const [notificationBody, setNotificationBody] = useState('Rich notification test from the demo app')
  const [notificationImage, setNotificationImage] = useState('')
  const [streamPath, setStreamPath] = useState('')
  const [streamUrl, setStreamUrl] = useState('')
  const [streamMimeType, setStreamMimeType] = useState('')
  const [clipboardImagePreview, setClipboardImagePreview] = useState('')

  const addLog = (msg: string) => setLogs(prev => [...prev, msg])

  useEffect(() => {
    const hasCef = typeof (window as any).cefQuery === 'function';
    addLog(hasCef ? '✓ CEF bridge detected' : '⚠️ Dev mode (no CEF)');
  }, [])

  useEffect(() => {
    return RustEvents.subscribe(({ event, payload }) => {
      addLog(`⇠ event ${event}: ${JSON.stringify(payload)}`);
    });
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
      const childWindowHtml = `
        <html>
          <body style="font-family:sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#242424;color:white;">
            <h1>Hello Window!</h1>
          </body>
        </html>
      `;

      const result = await RustWindow.create({
        url: `data:text/html;charset=utf-8,${encodeURIComponent(childWindowHtml)}`,
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

  const handleCreatePersistentWindow = async () => {
    setError('');
    addLog(`→ invoke('create_window', { persist_key: 'demo:persistent-window' })`);
    try {
      const childWindowHtml = `
        <html>
          <body style="font-family:sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#1f3a5f;color:white;">
            <h1>Hello Persistent Window!</h1>
          </body>
        </html>
      `;

      const result = await RustWindow.create({
        url: `data:text/html;charset=utf-8,${encodeURIComponent(childWindowHtml)}`,
        title: 'Persistent Secondary Window',
        width: 600,
        height: 400,
        x: 120,
        y: 120,
        persist_key: 'demo:persistent-window',
      });
      addLog(`✓ Persistent window ${result.status}`);
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

  const handleRegisterShortcut = async () => {
    setError('');
    addLog(`→ register_global_shortcut('${shortcutId}', '${shortcutAccelerator}')`);
    try {
      const result = await RustOS.registerGlobalShortcut(shortcutId, shortcutAccelerator);
      addLog(`✓ Shortcut registered: ${result.accelerator}`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleUnregisterShortcut = async () => {
    setError('');
    addLog(`→ unregister_global_shortcut('${shortcutId}')`);
    try {
      const result = await RustOS.unregisterGlobalShortcut(shortcutId);
      addLog(`✓ Shortcut ${result.status}: ${result.id}`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handlePollShortcutEvents = async () => {
    setError('');
    addLog('→ poll_global_shortcut_events()');
    try {
      const events = await RustOS.pollGlobalShortcutEvents();
      if (events.length === 0) {
        addLog('• No shortcut events');
      } else {
        for (const event of events) {
          addLog(`✓ Shortcut ${event.id}: ${event.state} (${event.accelerator})`);
        }
      }
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleShowRichNotification = async () => {
    setError('');
    addLog('→ show_notification(rich)');
    try {
      const result = await RustOS.showNotification({
        title: 'Rust CEF',
        body: notificationBody,
        subtitle: 'Rich notification test',
        sound: 'default',
        action: 'Open',
        close_button: 'Dismiss',
        wait_for_click: false,
        content_image: notificationImage || undefined,
      });
      addLog(`✓ Notification shown: ${result.response?.kind ?? 'none'}`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleCreateStreamUrl = async () => {
    setError('');
    addLog(`→ create_file_stream_url('${streamPath}')`);
    try {
      const result = await RustFileSystem.createFileStreamUrl(streamPath);
      setStreamUrl(result.url);
      setStreamMimeType(result.mime_type);
      addLog(`✓ Stream URL created: ${result.mime_type}`);
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleOpenStream = () => {
    if (!streamUrl) {
      addLog('• No stream URL to open');
      return;
    }
    window.open(streamUrl, '_blank');
    addLog(`✓ Opened stream URL`);
  }

  const handlePollAppEvents = async () => {
    setError('');
    addLog('→ poll_app_events()');
    try {
      const events = await RustOS.pollAppEvents();
      if (events.length === 0) {
        addLog('• No app events');
      } else {
        for (const event of events) {
          addLog(`✓ App event: ${event.event} ${JSON.stringify(event.payload)}`);
        }
      }
    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

	const handleReadClipboardImage = async () => {
	    setError('');
	    addLog('→ clipboard_read_image()');
	    try {
	      const image = await RustClipboard.readImage();
	      const pngBytes = new Uint8Array(image.bytes.length);
	      pngBytes.set(image.bytes);
	      const blob = new Blob([pngBytes], { type: 'image/png' });
	      setClipboardImagePreview(URL.createObjectURL(blob));
	      addLog(`✓ Clipboard image read: ${image.width}x${image.height}`);
	    } catch (e) {
      const msg = (e as Error).message;
      setError(msg);
      addLog(`✗ ${msg}`);
    }
  }

  const handleWriteClipboardImage = async () => {
    setError('');
    addLog('→ clipboard_write_image()');
    try {
      const canvas = document.createElement('canvas');
      canvas.width = 64;
      canvas.height = 64;
      const context = canvas.getContext('2d');
      if (!context) {
        throw new Error('Canvas 2D context unavailable');
      }
      context.fillStyle = '#1f6feb';
      context.fillRect(0, 0, 64, 64);
      context.fillStyle = '#ffffff';
      context.font = 'bold 28px sans-serif';
      context.fillText('RC', 10, 40);
      const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
      if (!blob) {
        throw new Error('Failed to create PNG blob');
      }
      const bytes = new Uint8Array(await blob.arrayBuffer());
      await RustClipboard.writeImage(bytes);
      addLog('✓ Wrote demo image to clipboard');
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
      <button onClick={handleCreatePersistentWindow}>New window (persistence)</button>
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

        <div style={{ padding: '20px', borderTop: '1px solid #333', marginTop: '20px' }}>
          <h3>Medium Feature Tests</h3>

          <div style={{ textAlign: 'left', marginBottom: '12px' }}>
            <div style={{ marginBottom: '6px' }}><strong>Global Shortcuts</strong></div>
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', marginBottom: '8px' }}>
              <input
                value={shortcutId}
                onChange={(e) => setShortcutId(e.target.value)}
                placeholder="Shortcut ID"
                style={{ flex: '1 1 160px' }}
              />
              <input
                value={shortcutAccelerator}
                onChange={(e) => setShortcutAccelerator(e.target.value)}
                placeholder="CmdOrCtrl+Shift+Y"
                style={{ flex: '2 1 220px' }}
              />
            </div>
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
              <button onClick={handleRegisterShortcut}>Register Shortcut</button>
              <button onClick={handleUnregisterShortcut}>Unregister Shortcut</button>
              <button onClick={handlePollShortcutEvents}>Poll Shortcut Events</button>
              <button onClick={handlePollAppEvents}>Poll App Events</button>
            </div>
          </div>

          <div style={{ textAlign: 'left', marginBottom: '12px' }}>
            <div style={{ marginBottom: '6px' }}><strong>Rich Notifications</strong></div>
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', marginBottom: '8px' }}>
              <input
                value={notificationBody}
                onChange={(e) => setNotificationBody(e.target.value)}
                placeholder="Notification body"
                style={{ flex: '2 1 260px' }}
              />
              <input
                value={notificationImage}
                onChange={(e) => setNotificationImage(e.target.value)}
                placeholder="Optional image path"
                style={{ flex: '2 1 260px' }}
              />
            </div>
            <button onClick={handleShowRichNotification}>Show Rich Notification</button>
          </div>

          <div style={{ textAlign: 'left' }}>
            <div style={{ marginBottom: '6px' }}><strong>Streamed File URL</strong></div>
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', marginBottom: '8px' }}>
              <input
                value={streamPath}
                onChange={(e) => setStreamPath(e.target.value)}
                placeholder="/absolute/path/to/file"
                style={{ flex: '3 1 360px' }}
              />
              <button onClick={handleCreateStreamUrl}>Create Stream URL</button>
              <button onClick={handleOpenStream}>Open Stream URL</button>
            </div>
            {streamUrl && (
              <div style={{ fontSize: '12px', color: '#888' }}>
                <div>MIME: {streamMimeType}</div>
                <div style={{ wordBreak: 'break-all' }}>{streamUrl}</div>
              </div>
            )}
          </div>

          <div style={{ textAlign: 'left', marginTop: '12px' }}>
            <div style={{ marginBottom: '6px' }}><strong>Image Clipboard</strong></div>
            <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', marginBottom: '8px' }}>
              <button onClick={handleWriteClipboardImage}>Write Demo Image</button>
              <button onClick={handleReadClipboardImage}>Read Clipboard Image</button>
            </div>
            {clipboardImagePreview && (
              <img
                src={clipboardImagePreview}
                alt="Clipboard preview"
                style={{ maxWidth: '160px', maxHeight: '160px', borderRadius: '8px', border: '1px solid #333' }}
              />
            )}
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

/**
 * Rust IPC Bridge
 *
 * High-level API for invoking Rust commands from the frontend.
 * Uses callRust() from ipc.ts as the transport layer.
 *
 * Usage:
 *   import { invoke } from './rust-api';
 *   const result = await invoke<{ message: string }>('greet', { name: 'Paul' });
 */

import { callRust } from './ipc';

let requestId = 0;

function generateId(): string {
    return `req_${++requestId}_${Date.now()}`;
}

/**
 * Invoke a Rust command with optional arguments.
 * Returns a Promise that resolves with the command's response data.
 */
export async function invoke<T = unknown>(
    command: string,
    args: Record<string, unknown> = {},
): Promise<T> {
    const id = generateId();
    const requestJson = JSON.stringify({ cmd: command, args, id });

    // Use the working callRust transport
    const responseJson = await callRust(requestJson);

    const response = JSON.parse(responseJson);

    if (response.success) {
        return response.data as T;
    } else {
        throw new Error(response.error || 'Unknown IPC error');
    }
}

// --- Types ---

export interface FileMetadata {
    is_file: boolean;
    is_dir: boolean;
    size: number;
    modified: number | null;
}

export interface DirEntry {
    name: string;
    metadata: FileMetadata;
}

export interface OpenDialogOptions {
    title?: string;
    directory?: string;
    filters?: string[] | null;
    multiple?: boolean;
}

export interface SaveDialogOptions {
    title?: string;
    directory?: string;
    filename?: string;
    filters?: string[] | null;
}

// --- API Wrapper ---

export class RustFileSystem {
    static async readFile(path: string): Promise<string> {
        return invoke<string>('read_file', { path });
    }

    static async readFileBinary(path: string): Promise<string> {
        return invoke<string>('read_file_binary', { path });
    }

    static async writeFile(path: string, content: string): Promise<boolean> {
        return invoke<boolean>('write_file', { path, content });
    }

    static async writeFileBinary(path: string, content: string): Promise<boolean> {
        return invoke<boolean>('write_file_binary', { path, content });
    }

    static async exists(path: string): Promise<boolean> {
        return invoke<boolean>('exists', { path });
    }

    static async readDir(path: string): Promise<DirEntry[]> {
        return invoke<DirEntry[]>('read_dir', { path });
    }

    static async getMetadata(path: string): Promise<FileMetadata> {
        return invoke<FileMetadata>('get_metadata', { path });
    }

    static async createFileStreamUrl(path: string): Promise<{
        url: string;
        path: string;
        mime_type: string;
        size: number;
    }> {
        return invoke<{
            url: string;
            path: string;
            mime_type: string;
            size: number;
        }>('create_file_stream_url', { path });
    }

    static async showOpenDialog(options: OpenDialogOptions = {}): Promise<string[] | string | null> {
        return invoke<string[] | string | null>('show_open_dialog', options as Record<string, unknown>);
    }

    static async showSaveDialog(options: SaveDialogOptions = {}): Promise<string | null> {
        return invoke<string | null>('show_save_dialog', options as Record<string, unknown>);
    }

    static async showPickFolderDialog(options: OpenDialogOptions = {}): Promise<string[] | string | null> {
        return invoke<string[] | string | null>('show_pick_folder_dialog', options as Record<string, unknown>);
    }
}

export interface WindowOptions {
    url?: string;
    title?: string;
    width?: number;
    height?: number;
    x?: number;
    y?: number;
    persist_key?: string;
    resizable?: boolean;
    frameless?: boolean;
    transparent?: boolean;
    always_on_top?: boolean;
    kiosk?: boolean;
    icon?: string; // base64 encoded png
}

export interface WindowConfigOptions {
    frameless?: boolean;
    transparent?: boolean;
    always_on_top?: boolean;
    kiosk?: boolean;
    icon?: string; // base64 encoded png
}

export class RustWindow {
    static async create(options: WindowOptions = {}): Promise<{ status: string, url: string }> {
        return invoke<{ status: string, url: string }>('create_window', options as Record<string, unknown>);
    }

    static async setConfig(options: WindowConfigOptions): Promise<{ status: string }> {
        return invoke<{ status: string }>('set_window_config', options as Record<string, unknown>);
    }
}

export interface PrintToPdfOptions {
    path: string;
    landscape?: boolean;
    print_background?: boolean;
    display_header_footer?: boolean;
    scale?: number;
}

export interface StartDownloadOptions {
    url: string;
    path?: string;
    show_dialog?: boolean;
}

export class RustBrowser {
    static async printToPdf(options: PrintToPdfOptions): Promise<{ status: string; path: string }> {
        return invoke<{ status: string; path: string }>('print_to_pdf', options as Record<string, unknown>);
    }

    static async startDownload(options: StartDownloadOptions): Promise<{ status: string; url: string }> {
        return invoke<{ status: string; url: string }>('start_download', options as Record<string, unknown>);
    }
}

export class RustOS {
    /**
     * Set the badge count on the macOS Dock and System Tray.
     * Pass 0 to clear the badge.
     */
    static async setBadgeCount(count: number): Promise<{ status: string, count: number }> {
        return invoke<{ status: string, count: number }>('set_badge_count', { count });
    }

    static async getLaunchContext(): Promise<{ deep_link: string | null; files: string[] }> {
        return invoke<{ deep_link: string | null; files: string[] }>('get_launch_context');
    }

    static async showNotification(options: {
        title: string;
        body?: string;
        subtitle?: string;
        sound?: string;
        app_icon?: string;
        content_image?: string;
        action?: string;
        close_button?: string;
        wait_for_click?: boolean;
    }): Promise<{
        status: string;
        title: string;
        body: string;
        subtitle?: string;
        sound?: string;
        response?: { kind: string; value?: string };
    }> {
        return invoke<{
            status: string;
            title: string;
            body: string;
            subtitle?: string;
            sound?: string;
            response?: { kind: string; value?: string };
        }>(
            'show_notification',
            options as Record<string, unknown>,
        );
    }

    static async registerGlobalShortcut(id: string, accelerator: string): Promise<{
        status: string;
        id: string;
        accelerator: string;
    }> {
        return invoke<{
            status: string;
            id: string;
            accelerator: string;
        }>('register_global_shortcut', { id, accelerator });
    }

    static async unregisterGlobalShortcut(id: string): Promise<{
        status: string;
        id: string;
    }> {
        return invoke<{ status: string; id: string }>('unregister_global_shortcut', { id });
    }

    static async listGlobalShortcuts(): Promise<Array<{ id: string; accelerator: string }>> {
        return invoke<Array<{ id: string; accelerator: string }>>('list_global_shortcuts');
    }

    static async pollGlobalShortcutEvents(): Promise<Array<{
        id: string;
        accelerator: string;
        state: 'pressed' | 'released';
    }>> {
        return invoke<Array<{
            id: string;
            accelerator: string;
            state: 'pressed' | 'released';
        }>>('poll_global_shortcut_events');
    }
}

export class RustClipboard {
    static async readText(): Promise<string> {
        const result = await invoke<{ text: string }>('clipboard_read_text');
        return result.text;
    }

    static async writeText(text: string): Promise<void> {
        await invoke<{ status: string }>('clipboard_write_text', { text });
    }

    static async clear(): Promise<void> {
        await invoke<{ status: string }>('clipboard_clear');
    }
}

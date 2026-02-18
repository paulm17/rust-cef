/**
 * IPC Type Definitions
 *
 * Shared types for the Rust ↔ JavaScript IPC bridge.
 * These must match the Rust structs in `src/ipc/bridge.rs`.
 */

/** Request sent from JS to Rust via cefQuery */
export interface IpcRequest {
    cmd: string;
    args: Record<string, unknown>;
    id: string;
}

export interface ShowMessageDialogRequest {
    level: "info" | "warning" | "error" | "confirm";
    title: string;
    message: string;
}

/** Response received from Rust */
export interface IpcResponse<T = unknown> {
    id: string;
    success: boolean;
    data?: T;
    error?: string;
}

/** The global `window.rust` API */
export interface RustApi {
    invoke: <T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>;
}

/** Augment the global Window interface */
declare global {
    interface Window {
        rust: RustApi;
    }
}

export { };

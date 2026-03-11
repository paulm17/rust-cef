export interface CefQueryRequest {
    request: string;
    onSuccess: (response: string) => void;
    onFailure: (error_code: number, error_message: string) => void;
    persistent?: boolean;
}

declare global {
    interface Window {
        cefQuery: (request: CefQueryRequest) => number;
        cefQueryCancel: (request_id: number) => void;
    }
}

const ipcDebugEnabled =
    import.meta.env.DEV &&
    typeof window !== 'undefined' &&
    window.localStorage?.getItem('rust_cef_ipc_debug') === '1';

function ipcDebug(...args: unknown[]) {
    if (ipcDebugEnabled) {
        console.debug('[rust-cef:ipc]', ...args);
    }
}

export function callRust(message: string): Promise<string> {
    return new Promise((resolve, reject) => {
        ipcDebug('callRust', message);

        if (!window.cefQuery) {
            ipcDebug('cefQuery is not defined');

            if (import.meta.env.DEV) {
                setTimeout(() => {
                    resolve(`Mock response (DEV mode): ${message}`);
                }, 100);
            } else {
                reject(new Error("cefQuery not defined - CEF IPC not initialized"));
            }
            return;
        }

        const queryRequest: CefQueryRequest = {
            request: message,
            onSuccess: (response: string) => {
                ipcDebug('success', response);
                resolve(response);
            },
            onFailure: (code: number, msg: string) => {
                ipcDebug('failure', code, msg);
                reject(new Error(`IPC Error [${code}]: ${msg}`));
            },
            persistent: false
        };

        try {
            window.cefQuery(queryRequest);
        } catch (error) {
            ipcDebug('exception', error);
            reject(error);
        }
    });
}

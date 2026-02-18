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

export function callRust(message: string): Promise<string> {
    return new Promise((resolve, reject) => {
        console.log('========================================');
        console.log('📞 callRust() called');
        console.log('📞 Message:', message);

        console.log('🔍 Checking window.cefQuery availability...');
        console.log('   - window:', !!window);
        console.log('   - window.cefQuery:', !!window.cefQuery);
        console.log('   - typeof window.cefQuery:', typeof window.cefQuery);

        if (!window.cefQuery) {
            console.warn('⚠️ window.cefQuery is not defined!');
            console.warn('⚠️ This means CEF IPC is not available');
            console.warn('⚠️ The renderer process should have injected this');

            if (import.meta.env.DEV) {
                console.log('🔧 DEV mode detected - returning mock response');
                setTimeout(() => {
                    const mockResponse = `Mock response (DEV mode): ${message}`;
                    console.log('✓ Mock response:', mockResponse);
                    resolve(mockResponse);
                }, 100);
            } else {
                console.error('✗ PRODUCTION mode - cefQuery unavailable, rejecting');
                reject(new Error("cefQuery not defined - CEF IPC not initialized"));
            }
            return;
        }

        console.log('✓ window.cefQuery is available');
        console.log('🚀 Creating query request object...');

        const queryRequest: CefQueryRequest = {
            request: message,
            onSuccess: (response: string) => {
                console.log('========================================');
                console.log('✓ IPC SUCCESS CALLBACK');
                console.log('✓ Response:', response);
                console.log('========================================');
                resolve(response);
            },
            onFailure: (code: number, msg: string) => {
                console.error('========================================');
                console.error('✗ IPC FAILURE CALLBACK');
                console.error('✗ Error code:', code);
                console.error('✗ Error message:', msg);
                console.error('========================================');
                reject(new Error(`IPC Error [${code}]: ${msg}`));
            },
            persistent: false
        };

        console.log('📤 Query request object created:', {
            request: queryRequest.request,
            persistent: queryRequest.persistent,
            hasOnSuccess: !!queryRequest.onSuccess,
            hasOnFailure: !!queryRequest.onFailure
        });

        console.log('📞 Calling window.cefQuery(queryRequest)...');

        try {
            const requestId = window.cefQuery(queryRequest);
            console.log('✓ window.cefQuery() returned request ID:', requestId);
            console.log('⏳ Waiting for Rust to respond...');
            console.log('========================================');
        } catch (error) {
            console.error('========================================');
            console.error('✗ EXCEPTION when calling window.cefQuery:');
            console.error('✗ Error:', error);
            console.error('✗ Error type:', typeof error);
            console.error('✗ Error details:', {
                message: (error as Error).message,
                stack: (error as Error).stack
            });
            console.error('========================================');
            reject(error);
        }
    });
}

use cef::{
    Browser, Frame, ImplBrowser,
    wrapper::message_router::{BrowserSideCallback, BrowserSideHandler},
};
use std::sync::{Arc, Mutex};

use crate::ipc::bridge::CommandRouter;

pub struct SimpleHandler {
    pub router: Arc<CommandRouter>,
}

impl BrowserSideHandler for SimpleHandler {
    fn on_query_str(
        &self,
        browser: Option<Browser>,
        _frame: Option<Frame>,
        query_id: i64,
        request: &str,
        _persistent: bool,
        callback: Arc<Mutex<dyn BrowserSideCallback>>,
    ) -> bool {
        let browser_id = browser.as_ref().map(|b| b.identifier()).unwrap_or(-1);
        eprintln!("DEBUG: SimpleHandler - Query {} from browser {}", query_id, browser_id);
        eprintln!("DEBUG: SimpleHandler - Request: '{}'", request);

        // Dispatch using the injected router
        let response = self.router.dispatch(request);

        eprintln!("DEBUG: SimpleHandler - Response: '{}'", response);

        match callback.lock() {
            Ok(cb) => {
                cb.success_str(&response);
                eprintln!("DEBUG: SimpleHandler - Response sent successfully");
            }
            Err(e) => {
                eprintln!("ERROR: SimpleHandler - Failed to lock callback: {:?}", e);
            }
        }

        true
    }
}

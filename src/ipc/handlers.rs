use cef::{
    wrapper::message_router::{BrowserSideCallback, BrowserSideHandler},
    Browser, Frame, ImplBrowser,
};
use std::sync::{Arc, Mutex};
use tracing::{debug, error};

use crate::{debug_logger::print_debug, ipc::bridge::CommandRouter};

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
        debug!(query_id, browser_id, "IPC query received");
        print_debug(&format!(
            "DEBUG: SimpleHandler - Query {} from browser {}",
            query_id, browser_id
        ));

        // Dispatch using the injected router
        let response = self.router.dispatch(request);
        debug!(query_id, browser_id, "IPC query handled");

        match callback.lock() {
            Ok(cb) => {
                cb.success_str(&response);
                debug!(query_id, browser_id, "IPC response sent");
            }
            Err(e) => {
                error!("Failed to lock IPC callback: {:?}", e);
            }
        }

        true
    }
}

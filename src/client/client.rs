use cef::{self, ImplClient, ImplBrowser};
use cef::{
    Client, ContextMenuHandler, LoadHandler, LifeSpanHandler, DisplayHandler, WrapClient,
    rc::{Rc, RcImpl},
    sys,
    wrapper::message_router::{
        BrowserSideRouter, MessageRouterConfig,
        MessageRouterBrowserSide, MessageRouterBrowserSideHandlerCallbacks
    },
    ImplProcessMessage,
};
use std::ptr::null_mut;
use crate::debug_logger::{log_debug, print_debug};

use crate::client::context_menu_handler::{ContextMenuHandlerBuilder, IcyContextMenuHandler};
use crate::client::lifespan_handler::{IcyLifeSpanHandler, LifeSpanHandlerBuilder};
use crate::client::load_handler::{IcyLoadHandler, LoadHandlerBuilder};
use crate::client::display_handler::{IcyDisplayHandler, DisplayHandlerBuilder};
use crate::ipc::handlers::SimpleHandler;

use crate::ipc::bridge::CommandRouter;
use std::sync::Arc;
use winit::event_loop::EventLoopProxy;
use crate::AppEvent;

pub struct IcyClient;

impl IcyClient {
    pub fn new(router: Arc<CommandRouter>, proxy: Option<EventLoopProxy<AppEvent>>) -> (Self, IcyClientHandlers) {
        print_debug("========================================");
        print_debug("DEBUG: IcyClient::new called");
        log_debug("DEBUG: IcyClient::new called");
        
        // Pass proxy to LoadHandler
        let load_handler = IcyLoadHandler::new(proxy);
        print_debug("DEBUG: IcyClient - LoadHandler created");
        
        let lifespan_handler = IcyLifeSpanHandler::new();
        print_debug("DEBUG: IcyClient - LifeSpanHandler created");
        
        let context_menu_handler = IcyContextMenuHandler::new();
        print_debug("DEBUG: IcyClient - ContextMenuHandler created");

        let display_handler = IcyDisplayHandler::new();
        print_debug("DEBUG: IcyClient - DisplayHandler created");

        let router_config = MessageRouterConfig::default();
        let browser_side_router = BrowserSideRouter::new(router_config);
        
        let handler = std::sync::Arc::new(SimpleHandler { router });
        
        let handler_id = browser_side_router.add_handler(handler, true);
        if handler_id.is_some() {
            print_debug("DEBUG: ✓ Handler successfully added to BrowserSideRouter");
        } else {
            print_debug("DEBUG: ✗ FAILED to add handler to BrowserSideRouter");
        }

        let handlers = IcyClientHandlers {
            load_handler,
            lifespan_handler,
            context_menu_handler,
            display_handler,
            message_router: browser_side_router,
        };
        
        print_debug("DEBUG: IcyClient::new completed");
        print_debug("========================================");
        
        (Self, handlers)
    }
}

#[derive(Clone)]
pub struct IcyClientHandlers {
    load_handler: IcyLoadHandler,
    lifespan_handler: IcyLifeSpanHandler,
    context_menu_handler: IcyContextMenuHandler,
    display_handler: IcyDisplayHandler,
    message_router: std::sync::Arc<BrowserSideRouter>,
}

pub(crate) struct ClientBuilder {
    object: *mut RcImpl<sys::cef_client_t, Self>,
    load_handler: LoadHandler,
    lifespan_handler: LifeSpanHandler,
    context_menu_handler: ContextMenuHandler,
    display_handler: DisplayHandler,
    message_router: std::sync::Arc<BrowserSideRouter>,
}

impl ClientBuilder {
    pub(crate) fn build(client_handlers: IcyClientHandlers) -> Client {
        let IcyClientHandlers {
            load_handler,
            lifespan_handler,
            context_menu_handler,
            display_handler,
            message_router,
        } = client_handlers;
        let load_handler = LoadHandlerBuilder::build(load_handler);
        let lifespan_handler = LifeSpanHandlerBuilder::build(lifespan_handler);
        let context_menu_handler = ContextMenuHandlerBuilder::build(context_menu_handler);
        let display_handler = DisplayHandlerBuilder::build(display_handler);
        
        Client::new(Self {
            object: null_mut(),
            load_handler,
            lifespan_handler,
            context_menu_handler,
            display_handler,
            message_router,
        })
    }
}

impl Rc for ClientBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

impl WrapClient for ClientBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::cef_client_t, Self>) {
        self.object = object;
    }
}

impl Clone for ClientBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            rc_impl.interface.add_ref();
            rc_impl
        };

        Self {
            object,
            load_handler: self.load_handler.clone(),
            lifespan_handler: self.lifespan_handler.clone(),
            context_menu_handler: self.context_menu_handler.clone(),
            display_handler: self.display_handler.clone(),
            message_router: self.message_router.clone(),
        }
    }
}

impl ImplClient for ClientBuilder {
    fn get_raw(&self) -> *mut sys::_cef_client_t {
        // Safe to log here? It's called very often. Maybe just once?
        log_debug("DEBUG: Client::get_raw called"); 
        self.object.cast()
    }

    fn load_handler(&self) -> Option<cef::LoadHandler> {
        Some(self.load_handler.clone())
    }

    fn life_span_handler(&self) -> Option<cef::LifeSpanHandler> {
        Some(self.lifespan_handler.clone())
    }

    fn context_menu_handler(&self) -> Option<ContextMenuHandler> {
        Some(self.context_menu_handler.clone())
    }

    fn display_handler(&self) -> Option<cef::DisplayHandler> {
        Some(self.display_handler.clone())
    }

    fn on_process_message_received(
        &self,
        browser: Option<&mut cef::Browser>,
        frame: Option<&mut cef::Frame>,
        source_process: cef::ProcessId,
        message: Option<&mut cef::ProcessMessage>,
    ) -> i32 {
        print_debug("========================================");
        print_debug("DEBUG: Client::on_process_message_received called");
        log_debug("DEBUG: Client::on_process_message_received called");
        
        let browser_id = browser.as_ref().map(|b| b.identifier()).unwrap_or(-1);
        print_debug(&format!("DEBUG: Client - Browser ID: {}", browser_id));
        log_debug(&format!("DEBUG: Client - Browser ID: {}", browser_id));
        
        
        print_debug(&format!("DEBUG: Client - Source process: {:?}", source_process));
        log_debug(&format!("DEBUG: Client - Source process: {:?}", source_process));
        
        if let Some(msg) = message.as_ref() {
            let name = cef::CefStringUtf16::from(&msg.name()).to_string();
            print_debug(&format!("DEBUG: Client - Message name: '{}'", name));
            log_debug(&format!("DEBUG: Client - Message name: '{}'", name));
            
            // Try to get argument list if available
            if msg.is_valid() != 0 {
                print_debug("DEBUG: Client - Message is valid");
                log_debug("DEBUG: Client - Message is valid");
            } else {
                print_debug("DEBUG: Client - Message is INVALID");
                log_debug("DEBUG: Client - Message is INVALID");
            }
        } else {
            print_debug("DEBUG: Client - Message is None");
            log_debug("DEBUG: Client - Message is None");
        }

        print_debug("DEBUG: Client - Calling message_router.on_process_message_received");
        log_debug("DEBUG: Client - Calling message_router.on_process_message_received");
        
        let handled = self.message_router.on_process_message_received(
            browser.map(|b| b.clone()),
            frame.map(|f| f.clone()),
            source_process,
            message.as_deref().cloned(),
        );
        
        
        print_debug(&format!("DEBUG: Client - Message handled by router: {}", handled));
        log_debug(&format!("DEBUG: Client - Message handled by router: {}", handled));
        
        let result = if handled { 1 } else { 0 };
        print_debug(&format!("DEBUG: Client - Returning: {}", result));
        print_debug("========================================");
        log_debug(&format!("DEBUG: Client - Returning: {}", result));
        
        result
    }
}
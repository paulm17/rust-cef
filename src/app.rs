use cef::{
    self, App, BrowserProcessHandler, CommandLine, ImplApp, ImplBrowser, ImplFrame, ImplBrowserProcessHandler,
    ImplCommandLine, ImplRenderProcessHandler, RenderProcessHandler, WrapApp,
    WrapBrowserProcessHandler, WrapRenderProcessHandler, rc::{Rc, RcImpl}, sys,
    SchemeRegistrar, ImplSchemeRegistrar,
    wrapper::message_router::{
        RendererSideRouter, MessageRouterConfig,
        MessageRouterRendererSide, MessageRouterRendererSideHandlerCallbacks
    },
    ImplProcessMessage,
};
use std::ptr::null_mut;

use crate::debug_logger::{log_debug, print_debug};

pub fn get_start_url() -> String {
    if cfg!(debug_assertions) {
        "http://localhost:5173".to_string()
    } else {
        "app://localhost/index.html".to_string()
    }
}
// ... imports

#[derive(Clone)]
pub struct SimpleApp;

use crate::platform::scheme_handler::AssetResolver;
use std::sync::Arc;

pub struct AppBuilder {
    object: *mut RcImpl<sys::_cef_app_t, Self>,
    app: SimpleApp,
    message_router: std::sync::Arc<RendererSideRouter>,
    resolver: Arc<AssetResolver>,
}

impl AppBuilder {
    pub fn build(resolver: Arc<AssetResolver>) -> App {
        print_debug("========================================");
        print_debug("DEBUG: AppBuilder::build called");
        log_debug("DEBUG: AppBuilder::build called");
        
        
        print_debug("DEBUG: AppBuilder - Creating MessageRouterConfig");
        log_debug("DEBUG: AppBuilder - Creating MessageRouterConfig");
        let router_config = MessageRouterConfig::default();
        
        
        print_debug("DEBUG: AppBuilder - Creating RendererSideRouter");
        log_debug("DEBUG: AppBuilder - Creating RendererSideRouter");
        let message_router = RendererSideRouter::new(router_config);
        
        
        print_debug("DEBUG: AppBuilder - Creating App");
        log_debug("DEBUG: AppBuilder - Creating App");
        let app = App::new(Self {
            object: null_mut(),
            app: SimpleApp,
            message_router,
            resolver,
        });

        print_debug("DEBUG: AppBuilder::build completed");
        print_debug("========================================");
        log_debug("DEBUG: AppBuilder::build completed");
        
        app
    }
}

impl Rc for AppBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

impl WrapApp for AppBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_app_t, Self>) {
        self.object = object;
    }
}

impl Clone for AppBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc = &mut *self.object;
            rc.interface.add_ref();
            self.object
        };
        Self {
            object,
            app: self.app.clone(),
            message_router: self.message_router.clone(),
            resolver: self.resolver.clone(),
        }
    }
}

impl ImplApp for AppBuilder {
    fn get_raw(&self) -> *mut sys::_cef_app_t {
        self.object as *mut sys::_cef_app_t
    }

    fn on_before_command_line_processing(
        &self,
        _process_type: Option<&cef::CefStringUtf16>,
        command_line: Option<&mut CommandLine>,
    ) {
        let Some(command_line) = command_line else {
            return;
        };

        // ... command line switches ...
        command_line.append_switch(Some(&"disable-javascript-open-windows".into()));
        command_line.append_switch(Some(&"disable-web-security".into()));
        command_line.append_switch(Some(&"hide-crash-restore-bubble".into()));
        command_line.append_switch(Some(&"disable-chrome-login-prompt".into()));
        command_line.append_switch(Some(&"allow-running-insecure-content".into()));
        command_line.append_switch(Some(&"no-startup-window".into()));
        command_line.append_switch(Some(&"disable-popup-blocking".into()));
        command_line.append_switch(Some(&"noerrdialogs".into()));
        
        #[cfg(target_os = "macos")]
        {
            command_line.append_switch_with_value(
                Some(&"password-store".into()),
                Some(&"basic".into()),
            );
            command_line.append_switch(Some(&"use-mock-keychain".into()));
            command_line.append_switch(Some(&"disable-encryption".into()));
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            command_line.append_switch(Some(&"use-mock-keychain".into()));
        }
        
        command_line.append_switch(Some(&"disable-spell-checking".into()));
        command_line.append_switch(Some(&"disable-session-crashed-bubble".into()));
        command_line
            .append_switch_with_value(Some(&"remote-debugging-port".into()), Some(&"9229".into()));
    }

    fn on_register_custom_schemes(
        &self,
        registrar: Option<&mut SchemeRegistrar>,
    ) {
        if let Some(registrar) = registrar {
            // Register "app" scheme as standard, secure, and cors-enabled
            // This allows it to behave like http/https and bypass CORS for relative assets
            let options = (sys::cef_scheme_options_t::CEF_SCHEME_OPTION_STANDARD as i32)
                | (sys::cef_scheme_options_t::CEF_SCHEME_OPTION_SECURE as i32)
                | (sys::cef_scheme_options_t::CEF_SCHEME_OPTION_CORS_ENABLED as i32)
                | (sys::cef_scheme_options_t::CEF_SCHEME_OPTION_FETCH_ENABLED as i32);
            
            registrar.add_custom_scheme(Some(&"app".into()), options);
        }
    }

    fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
        Some(self.clone().to_browser_process_handler())
    }

    fn render_process_handler(&self) -> Option<RenderProcessHandler> {
        Some(self.clone().to_render_process_handler())
    }
}

impl ImplBrowserProcessHandler for AppBuilder {
    fn get_raw(&self) -> *mut sys::_cef_browser_process_handler_t {
        self.object as *mut sys::_cef_browser_process_handler_t
    }

    fn on_context_initialized(&self) {
        let _ = cef::register_scheme_handler_factory(
            Some(&"app".into()),
            Some(&"localhost".into()),
            Some(&mut crate::platform::scheme_handler::AppSchemeHandlerFactory::new(self.resolver.clone())),
        );
    }

    fn on_schedule_message_pump_work(&self, delay_ms: i64) {
        if let Some(proxy_mutex) = crate::EVENT_LOOP_PROXY.get() {
            if let Ok(proxy) = proxy_mutex.lock() {
                let _ = proxy.send_event(crate::AppEvent::ScheduleMessagePumpWork(delay_ms));
            }
        }
    }
}

impl WrapBrowserProcessHandler for AppBuilder {
    fn wrap_rc(&mut self, _object: *mut RcImpl<sys::_cef_browser_process_handler_t, Self>) {
        // No need to store the object for now
    }
}

impl ImplRenderProcessHandler for AppBuilder {
    fn get_raw(&self) -> *mut sys::_cef_render_process_handler_t {
        self.object as *mut sys::_cef_render_process_handler_t
    }

    fn on_context_created(
        &self,
        browser: Option<&mut cef::Browser>,
        frame: Option<&mut cef::Frame>,
        context: Option<&mut cef::V8Context>,
    ) {
        print_debug("========================================");
        print_debug("DEBUG: RenderProcessHandler::on_context_created called");
        log_debug("DEBUG: RenderProcessHandler::on_context_created called");
        
        let browser_id = browser.as_ref().map(|b| b.identifier()).unwrap_or(-1);

        print_debug(&format!("DEBUG: Renderer - Browser ID: {}", browser_id));
        
        let frame_id = frame.as_ref()
            .map(|f| cef::CefStringUtf16::from(&f.identifier()).to_string())
            .unwrap_or_else(|| "unknown".to_string());
        print_debug(&format!("DEBUG: Renderer - Frame ID: {}", frame_id));
        log_debug(&format!("DEBUG: Renderer - Frame ID: {}", frame_id));
        
        
        print_debug(&format!("DEBUG: Renderer - Context present: {}", context.is_some()));
        log_debug(&format!("DEBUG: Renderer - Context present: {}", context.is_some()));
        
        
        print_debug("DEBUG: Renderer - Calling message_router.on_context_created");
        log_debug("DEBUG: Renderer - Calling message_router.on_context_created");

        self.message_router.on_context_created(
            browser.map(|b| b.clone()),
            frame.map(|f| f.clone()),
            context.map(|c| c.clone())
        );

        print_debug("DEBUG: Renderer - message_router.on_context_created completed");
        print_debug("DEBUG: Renderer - This should have injected window.cefQuery into JavaScript");
        print_debug("========================================");
        log_debug("DEBUG: Renderer - message_router.on_context_created completed");
        log_debug("DEBUG: Renderer - This should have injected window.cefQuery into JavaScript");
    }

    fn on_context_released(
        &self,
        browser: Option<&mut cef::Browser>,
        frame: Option<&mut cef::Frame>,
        context: Option<&mut cef::V8Context>,
    ) {
        self.message_router.on_context_released(
            browser.map(|b| b.clone()),
            frame.map(|f| f.clone()),
            context.map(|c| c.clone())
        );
    }

    fn on_process_message_received(
        &self,
        browser: Option<&mut cef::Browser>,
        frame: Option<&mut cef::Frame>,
        source_process: cef::ProcessId,
        message: Option<&mut cef::ProcessMessage>,
    ) -> i32 {
        print_debug("========================================");
        print_debug("DEBUG: RenderProcessHandler::on_process_message_received called");
        log_debug("DEBUG: RenderProcessHandler::on_process_message_received called");
        
        let browser_id = browser.as_ref().map(|b| b.identifier()).unwrap_or(-1);

        print_debug(&format!("DEBUG: Renderer - Browser ID: {}", browser_id));
        
        
        print_debug(&format!("DEBUG: Renderer - Source process: {:?}", source_process));
        log_debug(&format!("DEBUG: Renderer - Source process: {:?}", source_process));
        
        if let Some(msg) = message.as_ref() {
            let name = cef::CefStringUtf16::from(&msg.name()).to_string();
            print_debug(&format!("DEBUG: Renderer - Message name: '{}'", name));
            log_debug(&format!("DEBUG: Renderer - Message name: '{}'", name));
            
            if msg.is_valid() != 0 {
                print_debug("DEBUG: Renderer - Message is valid");
                log_debug("DEBUG: Renderer - Message is valid");
            } else {
                print_debug("DEBUG: Renderer - Message is INVALID");
                log_debug("DEBUG: Renderer - Message is INVALID");
            }
        } else {
            print_debug("DEBUG: Renderer - Message is None");
            log_debug("DEBUG: Renderer - Message is None");
        }

        print_debug("DEBUG: Renderer - Calling message_router.on_process_message_received");
        log_debug("DEBUG: Renderer - Calling message_router.on_process_message_received");

        let handled = self.message_router.on_process_message_received(
            browser.map(|b| b.clone()),
            frame.map(|f| f.clone()),
            Some(source_process),
            message.as_deref().cloned()
        );

        print_debug(&format!("DEBUG: Renderer - Message handled by router: {}", handled));
        log_debug(&format!("DEBUG: Renderer - Message handled by router: {}", handled));
        
        let result = if handled { 1 } else { 0 };
        print_debug(&format!("DEBUG: Renderer - Returning: {}", result));
        print_debug("========================================");
        log_debug(&format!("DEBUG: Renderer - Returning: {}", result));
        
        result
    }
}

impl WrapRenderProcessHandler for AppBuilder {
    fn wrap_rc(&mut self, _object: *mut RcImpl<sys::_cef_render_process_handler_t, Self>) {
        // No need to store the object for now
    }
}

trait ToBrowserProcessHandler {
    fn to_browser_process_handler(self) -> BrowserProcessHandler;
}

impl ToBrowserProcessHandler for AppBuilder {
    fn to_browser_process_handler(self) -> BrowserProcessHandler {
        BrowserProcessHandler::new(self)
    }
}

trait ToRenderProcessHandler {
    fn to_render_process_handler(self) -> RenderProcessHandler;
}

impl ToRenderProcessHandler for AppBuilder {
    fn to_render_process_handler(self) -> RenderProcessHandler {
        RenderProcessHandler::new(self)
    }
}
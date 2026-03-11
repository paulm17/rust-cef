use crate::debug_logger::{print_debug, print_info};
use crate::AppEvent;
use cef;
use cef::{
    rc::{Rc, RcImpl},
    sys, ImplLoadHandler, LoadHandler, WrapLoadHandler, *,
};
use std::ptr::null_mut;
use winit::event_loop::EventLoopProxy;

#[derive(Clone)]
pub struct IcyLoadHandler {
    proxy: Option<EventLoopProxy<AppEvent>>,
}

impl IcyLoadHandler {
    pub fn new(proxy: Option<EventLoopProxy<AppEvent>>) -> Self {
        Self { proxy }
    }
}

pub(crate) struct LoadHandlerBuilder {
    object: *mut RcImpl<sys::_cef_load_handler_t, Self>,
    _load_handler: IcyLoadHandler,
}

impl LoadHandlerBuilder {
    pub(crate) fn build(loader: IcyLoadHandler) -> LoadHandler {
        LoadHandler::new(Self {
            object: null_mut(),
            _load_handler: loader,
        })
    }
}

impl WrapLoadHandler for LoadHandlerBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_load_handler_t, Self>) {
        self.object = object;
    }
}

impl Rc for LoadHandlerBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

impl Clone for LoadHandlerBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            rc_impl.interface.add_ref();
            rc_impl
        };

        Self {
            object,
            _load_handler: self._load_handler.clone(),
        }
    }
}

impl ImplLoadHandler for LoadHandlerBuilder {
    fn get_raw(&self) -> *mut sys::_cef_load_handler_t {
        self.object.cast()
    }

    fn on_loading_state_change(
        &self,
        _browser: Option<&mut Browser>,
        _is_loading: ::std::os::raw::c_int,
        _can_go_back: ::std::os::raw::c_int,
        _can_go_forward: ::std::os::raw::c_int,
    ) {
    }

    fn on_load_start(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        _transition_type: cef::TransitionType,
    ) {
    }

    fn on_load_end(
        &self,
        _browser: Option<&mut Browser>,
        frame: Option<&mut Frame>,
        http_status_code: ::std::os::raw::c_int,
    ) {
        if let Some(frame) = frame {
            if frame.is_main() == 1 {
                // usage: CefStringUtf16::from(userfree).to_string()
                let url_cef = frame.url();
                let url_utf16 = cef::CefStringUtf16::from(&url_cef);
                let url = String::from_utf16_lossy(url_utf16.as_slice().unwrap_or(&[]));

                if !url.contains("rust_cef_loading.html") {
                    print_info(&format!(
                        "Load end for URL: {} (Status: {})",
                        url, http_status_code
                    ));
                }

                // If this is a real content URL (not file:// loading screen or about:blank), show window
                if (url.starts_with("http://")
                    || url.starts_with("https://")
                    || url.starts_with("scheme://"))
                    && !url.contains("google.com")
                // Ignore random stuff if any
                {
                    print_info("Main content loaded. Signaling event loop to show window.");
                    if let Some(proxy) = &self._load_handler.proxy {
                        let _ = proxy.send_event(AppEvent::ContentLoaded);
                    }
                }
            }
        }
    }

    fn on_load_error(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        error_code: cef::Errorcode,
        error_text: Option<&cef::CefString>,
        failed_url: Option<&cef::CefString>,
    ) {
        let error_text_str = error_text
            .map(cef::CefStringUtf8::from)
            .and_then(|s| s.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let failed_url_str = failed_url
            .map(cef::CefStringUtf8::from)
            .and_then(|s| s.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        tracing::error!(
            "Load error: code={}, text={}, url={}",
            *error_code.as_ref() as i32,
            error_text_str,
            failed_url_str
        );
        print_debug(&format!(
            "DEBUG: Load error for {}: {}",
            failed_url_str, error_text_str
        ));
    }
}

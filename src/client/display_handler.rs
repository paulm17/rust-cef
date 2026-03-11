use crate::debug_logger::print_debug;
use cef;
use cef::{
    rc::{Rc, RcImpl},
    sys, DisplayHandler, ImplDisplayHandler, WrapDisplayHandler, *,
};
use std::ptr::null_mut;

#[derive(Clone)]
pub struct IcyDisplayHandler;

impl IcyDisplayHandler {
    pub fn new() -> Self {
        Self
    }
}

pub(crate) struct DisplayHandlerBuilder {
    object: *mut RcImpl<sys::_cef_display_handler_t, Self>,
    _display_handler: IcyDisplayHandler,
}

impl DisplayHandlerBuilder {
    pub(crate) fn build(handler: IcyDisplayHandler) -> DisplayHandler {
        DisplayHandler::new(Self {
            object: null_mut(),
            _display_handler: handler,
        })
    }
}

impl WrapDisplayHandler for DisplayHandlerBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_display_handler_t, Self>) {
        self.object = object;
    }
}

impl Rc for DisplayHandlerBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

impl Clone for DisplayHandlerBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            rc_impl.interface.add_ref();
            rc_impl
        };

        Self {
            object,
            _display_handler: self._display_handler.clone(),
        }
    }
}

impl ImplDisplayHandler for DisplayHandlerBuilder {
    fn get_raw(&self) -> *mut sys::_cef_display_handler_t {
        self.object.cast()
    }

    fn on_console_message(
        &self,
        _browser: Option<&mut Browser>,
        level: cef::LogSeverity,
        message: Option<&cef::CefString>,
        source: Option<&cef::CefString>,
        line: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int {
        let message_str = message
            .map(cef::CefStringUtf8::from)
            .and_then(|s| s.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let source_str = source
            .map(cef::CefStringUtf8::from)
            .and_then(|s| s.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        // Use debug formatting for level since enum variants are not directly matching or are newtypes
        let level_str = format!("{:?}", level);

        print_debug(&format!(
            "[JS {}] {}:{}: {}",
            level_str, source_str, line, message_str
        ));

        // Return 0 to let default behavior proceed (logging to console), or 1 to suppress
        0
    }
}

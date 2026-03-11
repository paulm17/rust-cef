use cef;
use cef::{rc::*, sys, ImplLifeSpanHandler, LifeSpanHandler, WrapLifeSpanHandler};
use std::ptr::null_mut;

#[derive(Clone)]
pub struct IcyLifeSpanHandler;

impl IcyLifeSpanHandler {
    pub fn new() -> Self {
        Self
    }
}

impl LifeSpanHandlerBuilder {
    pub fn build(handler: IcyLifeSpanHandler) -> LifeSpanHandler {
        LifeSpanHandler::new(Self {
            object: null_mut(),
            _handler: handler,
        })
    }
}

pub(crate) struct LifeSpanHandlerBuilder {
    object: *mut RcImpl<sys::cef_life_span_handler_t, Self>,
    _handler: IcyLifeSpanHandler,
}

impl Rc for LifeSpanHandlerBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

impl WrapLifeSpanHandler for LifeSpanHandlerBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_life_span_handler_t, Self>) {
        self.object = object;
    }
}

impl Clone for LifeSpanHandlerBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            rc_impl.interface.add_ref();
            rc_impl
        };

        Self {
            object,
            _handler: self._handler.clone(),
        }
    }
}

impl ImplLifeSpanHandler for LifeSpanHandlerBuilder {
    fn get_raw(&self) -> *mut sys::_cef_life_span_handler_t {
        self.object.cast()
    }

    fn on_after_created(&self, _browser: Option<&mut cef::Browser>) {}

    fn on_before_close(&self, _browser: Option<&mut cef::Browser>) {}

    fn on_before_popup(
        &self,
        _browser: Option<&mut cef::Browser>,
        _frame: Option<&mut cef::Frame>,
        _popup_id: ::std::os::raw::c_int,
        _target_url: Option<&cef::CefString>,
        _target_frame_name: Option<&cef::CefString>,
        _target_disposition: cef::WindowOpenDisposition,
        _user_gesture: ::std::os::raw::c_int,
        _popup_features: Option<&cef::PopupFeatures>,
        _window_info: Option<&mut cef::WindowInfo>,
        _client: Option<&mut Option<cef::Client>>,
        _settings: Option<&mut cef::BrowserSettings>,
        _extra_info: Option<&mut Option<cef::DictionaryValue>>,
        _no_javascript_access: Option<&mut ::std::os::raw::c_int>,
    ) -> ::std::os::raw::c_int {
        false as _
    }
}

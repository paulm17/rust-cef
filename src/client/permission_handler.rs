use cef;
use cef::{
    rc::{Rc, RcImpl},
    sys, Browser, CefStringUtf8, Frame, ImplMediaAccessCallback, ImplPermissionHandler,
    ImplPermissionPromptCallback, MediaAccessCallback, PermissionHandler, PermissionPromptCallback,
    PermissionRequestResult, WrapPermissionHandler,
};
use std::ptr::null_mut;

#[derive(Clone)]
pub struct IcyPermissionHandler;

impl IcyPermissionHandler {
    pub fn new() -> Self {
        Self
    }
}

pub(crate) struct PermissionHandlerBuilder {
    object: *mut RcImpl<sys::_cef_permission_handler_t, Self>,
    permission_handler: IcyPermissionHandler,
}

impl PermissionHandlerBuilder {
    pub(crate) fn build(permission_handler: IcyPermissionHandler) -> PermissionHandler {
        PermissionHandler::new(Self {
            object: null_mut(),
            permission_handler,
        })
    }
}

impl WrapPermissionHandler for PermissionHandlerBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_permission_handler_t, Self>) {
        self.object = object;
    }
}

impl Rc for PermissionHandlerBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

impl Clone for PermissionHandlerBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            rc_impl.interface.add_ref();
            rc_impl
        };

        Self {
            object,
            permission_handler: self.permission_handler.clone(),
        }
    }
}

impl ImplPermissionHandler for PermissionHandlerBuilder {
    fn get_raw(&self) -> *mut sys::_cef_permission_handler_t {
        self.object.cast()
    }

    fn on_request_media_access_permission(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        requesting_origin: Option<&cef::CefString>,
        requested_permissions: u32,
        callback: Option<&mut MediaAccessCallback>,
    ) -> std::os::raw::c_int {
        let origin = requesting_origin
            .map(CefStringUtf8::from)
            .and_then(|value| value.as_str().map(|value| value.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        tracing::warn!(
            origin,
            requested_permissions,
            "denying media access permission request by default"
        );
        if let Some(callback) = callback {
            callback.cancel();
        }
        1
    }

    fn on_show_permission_prompt(
        &self,
        _browser: Option<&mut Browser>,
        prompt_id: u64,
        requesting_origin: Option<&cef::CefString>,
        requested_permissions: u32,
        callback: Option<&mut PermissionPromptCallback>,
    ) -> std::os::raw::c_int {
        let origin = requesting_origin
            .map(CefStringUtf8::from)
            .and_then(|value| value.as_str().map(|value| value.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        tracing::warn!(
            origin,
            prompt_id,
            requested_permissions,
            "denying permission prompt by default"
        );
        if let Some(callback) = callback {
            callback.cont(PermissionRequestResult::DENY);
        }
        1
    }
}

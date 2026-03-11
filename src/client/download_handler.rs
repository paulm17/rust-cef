use cef;
use cef::{
    rc::{Rc, RcImpl},
    sys, BeforeDownloadCallback, Browser, CefString, CefStringUtf8, DownloadHandler, DownloadItem,
    DownloadItemCallback, ImplBeforeDownloadCallback, ImplBrowser, ImplDownloadHandler,
    ImplDownloadItem, WrapDownloadHandler,
};
use std::ptr::null_mut;

#[derive(Clone)]
pub struct IcyDownloadHandler;

impl IcyDownloadHandler {
    pub fn new() -> Self {
        Self
    }
}

pub(crate) struct DownloadHandlerBuilder {
    object: *mut RcImpl<sys::_cef_download_handler_t, Self>,
    download_handler: IcyDownloadHandler,
}

impl DownloadHandlerBuilder {
    pub(crate) fn build(download_handler: IcyDownloadHandler) -> DownloadHandler {
        DownloadHandler::new(Self {
            object: null_mut(),
            download_handler,
        })
    }
}

impl WrapDownloadHandler for DownloadHandlerBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_download_handler_t, Self>) {
        self.object = object;
    }
}

impl Rc for DownloadHandlerBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

impl Clone for DownloadHandlerBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            rc_impl.interface.add_ref();
            rc_impl
        };

        Self {
            object,
            download_handler: self.download_handler.clone(),
        }
    }
}

impl ImplDownloadHandler for DownloadHandlerBuilder {
    fn get_raw(&self) -> *mut sys::_cef_download_handler_t {
        self.object.cast()
    }

    fn can_download(
        &self,
        _browser: Option<&mut Browser>,
        url: Option<&CefString>,
        request_method: Option<&CefString>,
    ) -> std::os::raw::c_int {
        let url = url
            .map(CefStringUtf8::from)
            .and_then(|value| value.as_str().map(|value| value.to_string()))
            .unwrap_or_default();
        let method = request_method
            .map(CefStringUtf8::from)
            .and_then(|value| value.as_str().map(|value| value.to_string()))
            .unwrap_or_default();
        tracing::info!(url, method, "download allowed");
        1
    }

    fn on_before_download(
        &self,
        browser: Option<&mut Browser>,
        download_item: Option<&mut DownloadItem>,
        suggested_name: Option<&CefString>,
        callback: Option<&mut BeforeDownloadCallback>,
    ) -> std::os::raw::c_int {
        let Some(browser) = browser else {
            return 0;
        };
        let Some(callback) = callback else {
            return 0;
        };

        let browser_id = browser.identifier();
        let pending = crate::state::take_pending_download(browser_id);
        let suggested_name = suggested_name
            .map(CefStringUtf8::from)
            .and_then(|value| value.as_str().map(|value| value.to_string()))
            .or_else(|| {
                download_item.and_then(|item| {
                    let name = item.suggested_file_name();
                    let utf16 = cef::CefStringUtf16::from(&name);
                    utf16.as_slice().map(String::from_utf16_lossy)
                })
            })
            .unwrap_or_else(|| "download.bin".to_string());

        let (download_path, show_dialog) = if let Some(pending) = pending {
            (
                pending
                    .path
                    .unwrap_or_else(|| default_download_path(&suggested_name)),
                if pending.show_dialog { 1 } else { 0 },
            )
        } else {
            (default_download_path(&suggested_name), 1)
        };

        tracing::info!(browser_id, path = %download_path, show_dialog, "starting download");
        callback.cont(Some(&CefString::from(download_path.as_str())), show_dialog);
        1
    }

    fn on_download_updated(
        &self,
        browser: Option<&mut Browser>,
        download_item: Option<&mut DownloadItem>,
        _callback: Option<&mut DownloadItemCallback>,
    ) {
        let browser_id = browser
            .as_ref()
            .map(|browser| browser.identifier())
            .unwrap_or(-1);
        let Some(download_item) = download_item else {
            return;
        };

        let path = {
            let full_path = download_item.full_path();
            let utf16 = cef::CefStringUtf16::from(&full_path);
            utf16
                .as_slice()
                .map(String::from_utf16_lossy)
                .unwrap_or_default()
        };

        tracing::info!(
            browser_id,
            path,
            percent_complete = download_item.percent_complete(),
            in_progress = download_item.is_in_progress(),
            complete = download_item.is_complete(),
            "download updated"
        );
    }
}

fn default_download_path(filename: &str) -> String {
    dirs::download_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(filename)
        .to_string_lossy()
        .to_string()
}

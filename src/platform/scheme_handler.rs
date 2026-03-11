use cef::{
    rc::{Rc, RcImpl},
    sys, Browser, Callback, CefString, Frame, ImplCallback, ImplRequest, ImplResourceHandler,
    ImplResponse, ImplSchemeHandlerFactory, Request, ResourceHandler, Response,
    SchemeHandlerFactory, WrapResourceHandler, WrapSchemeHandlerFactory,
};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ptr::null_mut;
use std::sync::Arc;

// Type alias for the asset resolver function
pub type AssetResolver = dyn Fn(&str) -> Option<rust_embed::EmbeddedFile> + Send + Sync;

#[derive(Clone)]
pub struct AppSchemeHandler {
    object: *mut RcImpl<sys::_cef_resource_handler_t, Self>,
    state: RefCell<HandlerState>,
    resolver: Arc<AssetResolver>,
}

#[derive(Clone)]
struct HandlerState {
    offset: u64,
    source: ResponseSource,
    mime_type: String,
    status_code: i32,
    content_length: i64,
}

#[derive(Clone)]
enum ResponseSource {
    Memory(Vec<u8>),
    StreamFile(String),
    Empty,
}

impl AppSchemeHandler {
    pub fn new(resolver: Arc<AssetResolver>) -> ResourceHandler {
        ResourceHandler::new(Self {
            object: null_mut(),
            state: RefCell::new(HandlerState {
                offset: 0,
                source: ResponseSource::Empty,
                mime_type: "text/html".to_string(),
                status_code: 404,
                content_length: 0,
            }),
            resolver,
        })
    }
}

impl ImplResourceHandler for AppSchemeHandler {
    fn get_raw(&self) -> *mut sys::_cef_resource_handler_t {
        self.object as *mut sys::_cef_resource_handler_t
    }

    fn open(
        &self,
        request: Option<&mut Request>,
        handle_request: Option<&mut ::std::os::raw::c_int>,
        callback: Option<&mut Callback>,
    ) -> ::std::os::raw::c_int {
        let Some(request) = request else { return 0 };
        let Some(callback) = callback else { return 0 };
        let Some(handle_request) = handle_request else {
            return 0;
        };

        *handle_request = 1;

        let url = request.url();
        let url = CefString::from(&url).to_string();

        // Remove "app://localhost/" prefix
        let path = url.trim_start_matches("app://localhost/");
        let path = if path.starts_with("app://") {
            // Fallback if trim didn't work as expected or for other domains
            url.split("://")
                .nth(1)
                .unwrap_or("")
                .split('/')
                .skip(1)
                .collect::<Vec<&str>>()
                .join("/")
        } else {
            path.to_string()
        };

        // Handle root
        let path = if path.is_empty() || path == "/" {
            "index.html"
        } else {
            &path
        };

        // Remove query params
        let path = path.split('?').next().unwrap_or(path);

        tracing::info!("Loading asset: {}", path);

        {
            let mut state = self.state.borrow_mut();
            state.offset = 0;

            if let Some(token) = path.strip_prefix("__stream__/") {
                if let Some(stream) = crate::state::get_file_stream(token) {
                    match std::fs::metadata(&stream.path) {
                        Ok(metadata) if metadata.is_file() => {
                            state.source = ResponseSource::StreamFile(stream.path);
                            state.mime_type = stream.mime_type;
                            state.status_code = 200;
                            state.content_length = metadata.len() as i64;
                        }
                        Ok(_) => {
                            tracing::warn!("Stream target is not a file: {}", token);
                            state.source = ResponseSource::Memory(
                                "400 Invalid stream target".as_bytes().to_vec(),
                            );
                            state.mime_type = "text/plain".to_string();
                            state.status_code = 400;
                            state.content_length = 25;
                        }
                        Err(err) => {
                            tracing::warn!("Failed to open stream target {}: {}", token, err);
                            state.source =
                                ResponseSource::Memory("404 Stream Not Found".as_bytes().to_vec());
                            state.mime_type = "text/plain".to_string();
                            state.status_code = 404;
                            state.content_length = 20;
                        }
                    }
                } else {
                    tracing::warn!("Unknown stream token: {}", token);
                    state.source =
                        ResponseSource::Memory("404 Stream Not Found".as_bytes().to_vec());
                    state.mime_type = "text/plain".to_string();
                    state.status_code = 404;
                    state.content_length = 20;
                }
            } else if let Some(file) = (self.resolver)(path) {
                let data = file.data.into_owned();
                state.content_length = data.len() as i64;
                state.source = ResponseSource::Memory(data);
                state.mime_type = mime_guess::from_path(path)
                    .first_or_text_plain()
                    .to_string();
                state.status_code = 200;
            } else {
                tracing::warn!("Asset not found: {}", path);
                let data = "404 Not Found".as_bytes().to_vec();
                state.content_length = data.len() as i64;
                state.source = ResponseSource::Memory(data);
                state.mime_type = "text/plain".to_string();
                state.status_code = 404;
            }
        } // Drop borrow before calling continue

        callback.cont();
        1
    }

    fn process_request(
        &self,
        _request: Option<&mut Request>,
        _callback: Option<&mut Callback>,
    ) -> i32 {
        0
    }

    fn response_headers(
        &self,
        response: Option<&mut Response>,
        response_length: Option<&mut i64>,
        _redirect_url: Option<&mut CefString>,
    ) {
        let Some(response) = response else { return };
        let state = self.state.borrow();

        response.set_mime_type(Some(&CefString::from(state.mime_type.as_str())));
        response.set_status(state.status_code);

        if let Some(len) = response_length {
            *len = state.content_length;
            tracing::info!("Setting response length: {}", *len);
        }
        tracing::info!(
            "Response headers set: status={}, mime={}",
            state.status_code,
            state.mime_type
        );
    }

    fn read(
        &self,
        data_out: *mut u8,
        bytes_to_read: i32,
        bytes_read: Option<&mut i32>,
        _callback: Option<&mut cef::ResourceReadCallback>,
    ) -> i32 {
        let mut state = self.state.borrow_mut();
        let bytes_to_read = bytes_to_read as usize;
        match state.source.clone() {
            ResponseSource::Memory(data) => {
                let remaining = data.len().saturating_sub(state.offset as usize);
                tracing::info!(
                    "Read called: offset={}, remaining={}, requested={}",
                    state.offset,
                    remaining,
                    bytes_to_read
                );

                if remaining == 0 {
                    if let Some(read) = bytes_read {
                        *read = 0;
                    }
                    return 0;
                }

                let amount = std::cmp::min(bytes_to_read, remaining);

                unsafe {
                    let dest = std::slice::from_raw_parts_mut(data_out, amount);
                    dest.copy_from_slice(
                        &data[state.offset as usize..state.offset as usize + amount],
                    );
                }

                state.offset += amount as u64;
                if let Some(read) = bytes_read {
                    *read = amount as i32;
                }

                1
            }
            ResponseSource::StreamFile(path) => {
                let mut file = match File::open(&path) {
                    Ok(file) => file,
                    Err(err) => {
                        tracing::error!("Failed to open stream file {}: {}", path, err);
                        if let Some(read) = bytes_read {
                            *read = 0;
                        }
                        return 0;
                    }
                };

                if file.seek(SeekFrom::Start(state.offset)).is_err() {
                    if let Some(read) = bytes_read {
                        *read = 0;
                    }
                    return 0;
                }

                let mut buffer = vec![0u8; bytes_to_read];
                let amount = match file.read(&mut buffer) {
                    Ok(amount) => amount,
                    Err(err) => {
                        tracing::error!("Failed to read stream file {}: {}", path, err);
                        if let Some(read) = bytes_read {
                            *read = 0;
                        }
                        return 0;
                    }
                };

                if amount == 0 {
                    if let Some(read) = bytes_read {
                        *read = 0;
                    }
                    return 0;
                }

                unsafe {
                    let dest = std::slice::from_raw_parts_mut(data_out, amount);
                    dest.copy_from_slice(&buffer[..amount]);
                }

                state.offset += amount as u64;
                if let Some(read) = bytes_read {
                    *read = amount as i32;
                }

                1
            }
            ResponseSource::Empty => {
                if let Some(read) = bytes_read {
                    *read = 0;
                }
                0
            }
        }
    }
}

impl WrapResourceHandler for AppSchemeHandler {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_resource_handler_t, Self>) {
        self.object = object;
    }
}

impl Rc for AppSchemeHandler {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

#[derive(Clone)]
pub struct AppSchemeHandlerFactory {
    object: *mut RcImpl<sys::_cef_scheme_handler_factory_t, Self>,
    resolver: Arc<AssetResolver>,
}

impl AppSchemeHandlerFactory {
    pub fn new(resolver: Arc<AssetResolver>) -> SchemeHandlerFactory {
        SchemeHandlerFactory::new(Self {
            object: null_mut(),
            resolver,
        })
    }
}

impl ImplSchemeHandlerFactory for AppSchemeHandlerFactory {
    fn get_raw(&self) -> *mut sys::_cef_scheme_handler_factory_t {
        self.object as *mut sys::_cef_scheme_handler_factory_t
    }

    fn create(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        _scheme_name: Option<&cef::CefString>,
        _request: Option<&mut Request>,
    ) -> Option<ResourceHandler> {
        Some(AppSchemeHandler::new(self.resolver.clone()))
    }
}

impl WrapSchemeHandlerFactory for AppSchemeHandlerFactory {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_scheme_handler_factory_t, Self>) {
        self.object = object;
    }
}

impl Rc for AppSchemeHandlerFactory {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            std::mem::transmute(&base.cef_object)
        }
    }
}

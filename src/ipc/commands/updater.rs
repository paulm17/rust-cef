use serde_json::Value;
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

pub fn get_updater_config(
    args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    crate::updater::get_config(args)
}

pub fn check_for_updates(
    args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    crate::updater::check_for_updates(args)
}

pub fn download_update(
    args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    crate::updater::download_update(args)
}

pub fn install_update(
    args: &Value,
    _proxy: &Option<Arc<Mutex<EventLoopProxy<crate::AppEvent>>>>,
) -> Result<Value, String> {
    crate::updater::install_update(args)
}

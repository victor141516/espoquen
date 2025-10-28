use once_cell::sync::Lazy;
use rdev::Key as RdevKey;
use std::sync::{Arc, Mutex};

// Global state for hotkey configuration
static HOTKEY: Lazy<Arc<Mutex<RdevKey>>> = Lazy::new(|| {
    Arc::new(Mutex::new(RdevKey::F6)) // Default hotkey is F6
});

/// Set the hotkey for starting/stopping recording
pub fn set_hotkey(key: RdevKey) {
    let mut hotkey = HOTKEY.lock().unwrap();
    *hotkey = key;
    println!("Hotkey updated to: {:?}", key);
}

/// Get the current hotkey
pub fn get_hotkey() -> RdevKey {
    let hotkey = HOTKEY.lock().unwrap();
    *hotkey
}

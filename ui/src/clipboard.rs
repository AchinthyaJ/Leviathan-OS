use std::fs;
use std::sync::Mutex;

static GLOBAL_CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);
const CLIPBOARD_FILE: &str = "/tmp/leviathan_clipboard.txt";

pub struct Clipboard;

impl Clipboard {
    pub fn init() {
        if let Ok(content) = fs::read_to_string(CLIPBOARD_FILE) {
            let mut lock = GLOBAL_CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());
            *lock = Some(content);
        }
    }

    pub fn set_text(text: &str) {
        let mut lock = GLOBAL_CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());
        *lock = Some(text.to_string());
        let _ = fs::write(CLIPBOARD_FILE, text);
    }

    pub fn get_text() -> String {
        // First check if external app wrote to the clipboard file
        if let Ok(content) = fs::read_to_string(CLIPBOARD_FILE) {
            let mut lock = GLOBAL_CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());
            *lock = Some(content.clone());
            return content;
        }

        let lock = GLOBAL_CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());
        lock.clone().unwrap_or_default()
    }

    pub fn has_text() -> bool {
        !Self::get_text().is_empty()
    }

    pub fn clear() {
        let mut lock = GLOBAL_CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());
        *lock = None;
        let _ = fs::remove_file(CLIPBOARD_FILE);
    }
}

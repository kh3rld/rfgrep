use arboard::Clipboard;

pub fn copy_to_clipboard(content: &str) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(content)
}
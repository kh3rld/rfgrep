// use anyhow::{Context, Result};
// use arboard::Clipboard;

// pub struct ClipboardManager {
//     clipboard: Clipboard,
// }

// impl ClipboardManager {
//     pub fn new() -> Result<Self> {
//         Ok(Self {
//             clipboard: Clipboard::new().context("Failed to initialize clipboard")?,
//         })
//     }

//     pub fn copy_to_clipboard(&mut self, content: &str) -> Result<()> {
//         // Set the content
//         self.clipboard
//             .set_text(content)
//             .context("Failed to copy content to clipboard")?;

//         // Verify the copy was successful by reading back
//         let copied = self
//             .clipboard
//             .get_text()
//             .context("Failed to verify clipboard content")?;

//         if copied != content {
//             anyhow::bail!("Clipboard verification failed: content mismatch");
//         }

//         Ok(())
//     }

//     pub fn get_clipboard_content(&mut self) -> Result<String> {
//         self.clipboard
//             .get_text()
//             .context("Failed to get clipboard content")
//     }
// }

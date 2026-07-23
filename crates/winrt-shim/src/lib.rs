//! `winrt-shim` — a reusable capability shim generalizing the proven
//! `tauri-winrt-notification` / `winrt-toast` pattern (BACKLOG §5.2).
//!
//! The portable, testable core is here: building the WinRT `ToastGeneric` XML
//! payload from a structured [`ToastContent`]. The actual OS dispatch is exposed
//! behind the [`ToastShim`] trait; a Windows implementation binds this XML to
//! `Windows.UI.Notifications.ToastNotificationManager` via `windows-rs` (see
//! [`SystemToast::show`] — that wiring is the documented next step and requires a
//! registered AppUserModelID / package identity to actually surface a toast).

/// A single actionable button on a toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastAction {
    pub label: String,
    /// `arguments` string handed back to the app when the button is clicked.
    pub arguments: String,
}

/// Structured toast content, independent of any OS API.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToastContent {
    pub title: String,
    pub body: String,
    pub actions: Vec<ToastAction>,
}

impl ToastContent {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        ToastContent {
            title: title.into(),
            body: body.into(),
            actions: Vec::new(),
        }
    }

    pub fn with_action(mut self, label: impl Into<String>, arguments: impl Into<String>) -> Self {
        self.actions.push(ToastAction {
            label: label.into(),
            arguments: arguments.into(),
        });
        self
    }

    /// Render the WinRT `ToastGeneric` XML payload.
    ///
    /// Shape follows the toast content schema: a `<binding template="ToastGeneric">`
    /// with two `<text>` lines, and one `<action>` per button inside `<actions>`.
    pub fn to_xml(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<toast>");
        xml.push_str("<visual><binding template=\"ToastGeneric\">");
        xml.push_str(&format!("<text>{}</text>", escape(&self.title)));
        xml.push_str(&format!("<text>{}</text>", escape(&self.body)));
        xml.push_str("</binding></visual>");
        if !self.actions.is_empty() {
            xml.push_str("<actions>");
            for a in &self.actions {
                xml.push_str(&format!(
                    "<action content=\"{}\" arguments=\"{}\" activationType=\"foreground\"/>",
                    escape(&a.label),
                    escape(&a.arguments)
                ));
            }
            xml.push_str("</actions>");
        }
        xml.push_str("</toast>");
        xml
    }
}

/// Minimal XML attribute/text escaping.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Error surface for a shim dispatch attempt.
#[derive(Debug, PartialEq, Eq)]
pub enum ShimError {
    /// The capability is not available on the current OS.
    Unsupported,
    /// Recognized on this OS, but the native binding isn't wired yet.
    NotImplemented(&'static str),
}

impl std::fmt::Display for ShimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShimError::Unsupported => write!(f, "capability unsupported on this OS"),
            ShimError::NotImplemented(why) => write!(f, "not implemented: {why}"),
        }
    }
}
impl std::error::Error for ShimError {}

/// A shim that can surface a toast on the host OS.
pub trait ToastShim {
    fn show(&self, content: &ToastContent) -> Result<(), ShimError>;
}

/// The OS-backed toast shim.
pub struct SystemToast;

impl ToastShim for SystemToast {
    fn show(&self, _content: &ToastContent) -> Result<(), ShimError> {
        // The XML payload is ready via `_content.to_xml()`. Wiring it to
        // `Windows.UI.Notifications.ToastNotificationManager::CreateToastNotifier`
        // (windows-rs) requires a registered AppUserModelID and is the next step.
        #[cfg(windows)]
        {
            Err(ShimError::NotImplemented(
                "bind ToastContent::to_xml() to ToastNotificationManager via windows-rs",
            ))
        }
        #[cfg(not(windows))]
        {
            Err(ShimError::Unsupported)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_contains_title_and_body() {
        let c = ToastContent::new("Hello", "World");
        let xml = c.to_xml();
        assert!(xml.contains("template=\"ToastGeneric\""));
        assert!(xml.contains("<text>Hello</text>"));
        assert!(xml.contains("<text>World</text>"));
        assert!(!xml.contains("<actions>"));
    }

    #[test]
    fn xml_renders_actions() {
        let c = ToastContent::new("t", "b")
            .with_action("Open", "action=open")
            .with_action("Dismiss", "action=dismiss");
        let xml = c.to_xml();
        assert!(xml.contains("content=\"Open\""));
        assert!(xml.contains("arguments=\"action=open\""));
        assert!(xml.contains("content=\"Dismiss\""));
    }

    #[test]
    fn xml_escapes_special_chars() {
        let c = ToastContent::new("a & b", "<x> \"q\"");
        let xml = c.to_xml();
        assert!(xml.contains("a &amp; b"));
        assert!(xml.contains("&lt;x&gt; &quot;q&quot;"));
    }

    #[test]
    fn system_toast_reports_an_error_until_wired() {
        // Either Unsupported (non-Windows) or NotImplemented (Windows) — never a
        // silent success while the native binding is a TODO.
        assert!(SystemToast.show(&ToastContent::new("t", "b")).is_err());
    }
}

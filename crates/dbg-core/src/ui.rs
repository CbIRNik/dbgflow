//! Embedded UI assets.
//!
//! This module contains the static assets for the browser-based debugger UI,
//! embedded at compile time via `include_str!`.

/// Returns the HTML content for the main page.
pub fn index_html() -> String {
    include_str!("../ui/index.html").to_owned()
}

/// Returns the JavaScript application code.
pub fn app_js() -> String {
    include_str!("../ui/app.js").to_owned()
}

/// Returns the CSS stylesheet.
pub fn app_css() -> String {
    include_str!("../ui/app.css").to_owned()
}

/// Returns the global Tailwind stylesheet used by shadcn/ui primitives.
pub fn globals_css() -> String {
    include_str!("../ui/globals.css").to_owned()
}

use gtk::prelude::GtkSettingsExt;
use tauri::Webview;
use webkit2gtk::{
    UserContentInjectedFrames, UserContentManagerExt, UserStyleLevel, UserStyleSheet, WebViewExt,
};

// Pages ask for the desktop font with `system-ui`, as Nextcloud does. Browsers use the GTK
// font (e.g. Ubuntu), but WebKitGTK 2.52 hands the name straight to fontconfig, whose
// aliases pick another font (Adwaita Sans on Ubuntu). WebKit looks for an @font-face of
// that name first, so a user style sheet maps it back to the GTK font.
pub fn configure(webview: &Webview) {
    let _ = webview.with_webview(|platform_webview| {
        let font_name = gtk::Settings::default().and_then(|settings| settings.gtk_font_name());
        let Some(sheet) = font_name.as_deref().and_then(style_sheet) else { return };
        if let Some(manager) = platform_webview.inner().user_content_manager() {
            manager.add_style_sheet(&UserStyleSheet::new(
                &sheet,
                UserContentInjectedFrames::AllFrames,
                UserStyleLevel::User,
                &[],
                &[],
            ));
        }
    });
}

fn style_sheet(gtk_font_name: &str) -> Option<String> {
    let family = gtk::pango::FontDescription::from_string(gtk_font_name).family()?;
    let family = family.replace('\\', "\\\\").replace('"', "\\\"");
    Some(["normal", "italic"].map(|style| {
        format!(r#"@font-face{{font-family:"system-ui";src:local("{family}");font-weight:1 1000;font-style:{style}}}"#)
    }).concat())
}

#[cfg(test)]
#[path = "../../tests/unit/shared/system_font.rs"]
mod tests;

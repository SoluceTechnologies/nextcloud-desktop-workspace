use gtk::prelude::{GtkSettingsExt, ObjectExt};
use tauri::Webview;
use webkit2gtk::{
    UserContentInjectedFrames, UserContentManagerExt, UserStyleLevel, UserStyleSheet, WebViewExt,
};

// Pages should read as they do in a browser on the same desktop.
//
// Family: pages ask for the desktop font with `system-ui`, as Nextcloud does. Browsers use
// the GTK font (e.g. Ubuntu), but WebKitGTK 2.52 hands the name straight to fontconfig,
// whose aliases pick another font (Adwaita Sans on Ubuntu). WebKit looks for an
// @font-face of that name first, so the style sheet maps it back to the GTK font.
//
// Smoothing: Chromium-based browsers draw grayscale text on Wayland, while WebKitGTK
// follows GNOME's subpixel setting, whose colour fringes made every weight look bolder.
pub fn configure(webview: &Webview) {
    let _ = webview.with_webview(|platform_webview| {
        let font_name = gtk::Settings::default().and_then(|settings| settings.gtk_font_name());
        let wayland = gtk::gdk::Display::default().is_some_and(|display| display.type_().name() == "GdkWaylandDisplay");
        let sheet = style_sheet(font_name.as_deref(), wayland);
        if sheet.is_empty() {
            return;
        }
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

fn style_sheet(gtk_font_name: Option<&str>, grayscale: bool) -> String {
    let family = gtk_font_name.and_then(|name| gtk::pango::FontDescription::from_string(name).family());
    let mut sheet = String::new();
    if let Some(family) = family {
        let family = family.replace('\\', "\\\\").replace('"', "\\\"");
        for style in ["normal", "italic"] {
            sheet += &format!(r#"@font-face{{font-family:"system-ui";src:local("{family}");font-weight:1 1000;font-style:{style}}}"#);
        }
    }
    if grayscale {
        sheet += "*{-webkit-font-smoothing:antialiased!important}";
    }
    sheet
}

#[cfg(test)]
#[path = "../../tests/unit/shared/fonts.rs"]
mod tests;

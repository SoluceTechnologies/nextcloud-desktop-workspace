use super::*;

#[test]
fn system_ui_becomes_the_desktop_font_family() {
    let sheet = style_sheet(Some("Ubuntu 11"), false);
    assert_eq!(sheet.matches(r#"font-family:"system-ui";src:local("Ubuntu")"#).count(), 2);
    assert!(sheet.contains("font-style:normal") && sheet.contains("font-style:italic"));
    assert!(sheet.contains("font-weight:1 1000"));
}

#[test]
fn style_and_size_are_not_part_of_the_family() {
    assert!(style_sheet(Some("Noto Sans Bold Italic 10"), false).contains(r#"local("Noto Sans")"#));
}

#[test]
fn no_desktop_font_means_no_font_face() {
    assert_eq!(style_sheet(None, false), "");
    assert_eq!(style_sheet(Some(""), false), "");
}

#[test]
fn family_names_cannot_break_out_of_the_css_string() {
    assert!(style_sheet(Some(r#"Odd\"Font 11"#), false).contains(r#"local("Odd\\\"Font")"#));
}

#[test]
fn grayscale_smoothing_only_when_asked() {
    assert!(style_sheet(None, true).contains("-webkit-font-smoothing:antialiased!important"));
    assert!(!style_sheet(Some("Ubuntu 11"), false).contains("font-smoothing"));
}

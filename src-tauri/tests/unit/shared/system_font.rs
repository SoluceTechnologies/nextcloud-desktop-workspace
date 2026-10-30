use super::*;

#[test]
fn system_ui_becomes_the_desktop_font_family() {
    let sheet = style_sheet("Ubuntu 11").unwrap();
    assert_eq!(sheet.matches(r#"font-family:"system-ui";src:local("Ubuntu")"#).count(), 2);
    assert!(sheet.contains("font-style:normal") && sheet.contains("font-style:italic"));
    assert!(sheet.contains("font-weight:1 1000"));
}

#[test]
fn style_and_size_are_not_part_of_the_family() {
    assert!(style_sheet("Noto Sans Bold Italic 10").unwrap().contains(r#"local("Noto Sans")"#));
}

#[test]
fn no_desktop_font_means_no_style_sheet() {
    assert_eq!(style_sheet(""), None);
}

#[test]
fn family_names_cannot_break_out_of_the_css_string() {
    assert!(style_sheet(r#"Odd\"Font 11"#).unwrap().contains(r#"local("Odd\\\"Font")"#));
}

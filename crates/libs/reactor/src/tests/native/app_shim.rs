use super::*;
use crate::Color;

#[test]
fn theme_colors_get_a_brush_per_theme() {
    let xaml = theme_color_dictionary_xaml(&[ThemeColorResource {
        key: "AppGreenBrush",
        light: Color::rgb(0x10, 0x20, 0x30),
        dark: Color::rgb(0xA0, 0xB0, 0xC0),
    }]);
    let entry = |color: &str| format!("<SolidColorBrush x:Key='AppGreenBrush' Color='{color}'/>");
    assert!(xaml.starts_with("<ResourceDictionary "));
    assert!(xaml.contains("<ResourceDictionary.ThemeDictionaries>"));
    assert!(xaml.contains(&format!(
        "<ResourceDictionary x:Key='Light'>{}</ResourceDictionary>",
        entry("#FF102030")
    )));
    assert!(xaml.contains(&format!(
        "<ResourceDictionary x:Key='Dark'>{}</ResourceDictionary>",
        entry("#FFA0B0C0")
    )));
    assert!(xaml.contains(&format!(
        "<ResourceDictionary x:Key='Default'>{}</ResourceDictionary>",
        entry("#FFA0B0C0")
    )));
    assert!(xaml.contains(&format!(
        "<ResourceDictionary x:Key='HighContrast'>{}</ResourceDictionary>",
        entry("{ThemeResource SystemColorButtonTextColor}")
    )));
}

#[test]
fn resource_doubles_are_plain_entries() {
    let xaml =
        double_dictionary_xaml(&[("NavigationViewItemOnLeftMinHeight", 28.0), ("Half", 0.5)]);
    assert!(xaml.starts_with("<ResourceDictionary "));
    assert!(xaml.contains("<x:Double x:Key='NavigationViewItemOnLeftMinHeight'>28</x:Double>"));
    assert!(xaml.contains("<x:Double x:Key='Half'>0.5</x:Double>"));
    assert!(!xaml.contains("ThemeDictionaries"));
}

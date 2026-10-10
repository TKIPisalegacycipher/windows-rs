use super::*;
use bindings::Application;
use bindings::*;
use windows_core::*;

implement_decl! {
    impl ReactorApplicationOverrides as pub ReactorApplicationOverrides_Impl:
        [IApplicationOverrides, IXamlMetadataProvider]
}

pub struct ReactorApplicationOverrides {
    controls_provider: RefCell<Option<XamlControlsXamlMetaDataProvider>>,
    on_launched: RefCell<Option<Box<dyn FnOnce() -> Result<()>>>>,
}

impl ReactorApplicationOverrides {
    fn new(on_launched: Box<dyn FnOnce() -> Result<()>>) -> Self {
        Self {
            controls_provider: RefCell::new(None),
            on_launched: RefCell::new(Some(on_launched)),
        }
    }

    fn provider(&self) -> Result<XamlControlsXamlMetaDataProvider> {
        if let Some(provider) = self.controls_provider.borrow().as_ref() {
            return Ok(provider.clone());
        }
        let provider = XamlControlsXamlMetaDataProvider::new()?;
        *self.controls_provider.borrow_mut() = Some(provider.clone());
        Ok(provider)
    }
}

impl IApplicationOverrides_Impl for ReactorApplicationOverrides_Impl {
    fn OnLaunched(&self, _args: Ref<LaunchActivatedEventArgs>) -> Result<()> {
        if let Some(on_launched) = self.on_launched.borrow_mut().take() {
            on_launched()?;
        }
        Ok(())
    }
}

impl IXamlMetadataProvider_Impl for ReactorApplicationOverrides_Impl {
    fn GetXamlType(&self, r#type: &TypeName) -> Result<IXamlType> {
        self.provider()?.GetXamlType(r#type)
    }

    fn GetXamlTypeByFullName(&self, full_name: &HSTRING) -> Result<IXamlType> {
        self.provider()?
            .GetXamlTypeByFullName(&full_name.to_string_lossy())
    }

    fn GetXmlnsDefinitions(&self) -> Result<Array<XmlnsDefinition>> {
        self.provider()?.GetXmlnsDefinitions()
    }
}

pub(super) fn create_application(
    on_launched: Box<dyn FnOnce() -> Result<()>>,
) -> Result<Application> {
    Application::compose(ReactorApplicationOverrides::new(on_launched))
}

pub(super) fn install_xaml_controls_resources(application: &Application) -> Result<()> {
    let controls = XamlControlsResources::new()?;
    let resources: ResourceDictionary = controls.cast()?;
    let merged = application.Resources()?.MergedDictionaries()?;
    merged.Append(&resources)?;
    let colors = registered_theme_colors();
    if !colors.is_empty() {
        merged.Append(&load_dictionary(&theme_color_dictionary_xaml(&colors))?)?;
    }
    let doubles = registered_resource_doubles();
    if !doubles.is_empty() {
        merged.Append(&load_dictionary(&double_dictionary_xaml(&doubles))?)?;
    }
    Ok(())
}

fn load_dictionary(xaml: &str) -> Result<ResourceDictionary> {
    XamlReader::Load(xaml)?.cast()
}

const DICTIONARY_START: &str = "<ResourceDictionary \
    xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' \
    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>";

fn theme_color_dictionary_xaml(colors: &[ThemeColorResource]) -> String {
    let mut xaml = format!("{DICTIONARY_START}<ResourceDictionary.ThemeDictionaries>");
    for theme in ["Light", "Dark", "Default", "HighContrast"] {
        xaml.push_str(&format!("<ResourceDictionary x:Key='{theme}'>"));
        for color in colors {
            let value = match theme {
                "Light" => hex_color(color.light),
                "HighContrast" => "{ThemeResource SystemColorButtonTextColor}".to_string(),
                _ => hex_color(color.dark),
            };
            xaml.push_str(&format!(
                "<SolidColorBrush x:Key='{}' Color='{value}'/>",
                color.key
            ));
        }
        xaml.push_str("</ResourceDictionary>");
    }
    xaml.push_str("</ResourceDictionary.ThemeDictionaries></ResourceDictionary>");
    xaml
}

fn double_dictionary_xaml(values: &[(&str, f64)]) -> String {
    let mut xaml = DICTIONARY_START.to_string();
    for (key, value) in values {
        xaml.push_str(&format!("<x:Double x:Key='{key}'>{value}</x:Double>"));
    }
    xaml.push_str("</ResourceDictionary>");
    xaml
}

fn hex_color(color: crate::Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color.a, color.r, color.g, color.b
    )
}

#[cfg(test)]
#[path = "../tests/native/app_shim.rs"]
mod tests;

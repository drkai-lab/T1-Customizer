fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target != "windows" {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    let icon = std::path::Path::new("..").join("icons").join("app.ico");
    if icon.is_file() {
        res.set_icon(icon.to_str().unwrap_or("app.ico"));
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "1.0.6".into());
    res.set("FileVersion", &format!("{version}.0"));
    res.set("ProductVersion", &version);
    res.set("ProductName", "T1-Customizer");
    res.set("FileDescription", "T1-Customizer");
    res.set("CompanyName", "AppWizz Kai");
    res.set("LegalCopyright", "Copyright (c) 2026 AppWizz Kai");
    let _ = res.compile();
}

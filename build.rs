fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var_os("CARGO_FEATURE_GUI").is_some()
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/openproject.ico");
        resource.compile().expect("compile Windows resources");
    }
}

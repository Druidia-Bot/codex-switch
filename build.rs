fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        // Icon is a nice-to-have — don't fail the build if rc.exe is absent.
        let _ = res.compile();
    }
}

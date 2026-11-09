#![windows_subsystem = "windows"]
mod language;
#[cfg(windows)]
mod tray;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::component::{
    ActiveTheme, Theme, ThemeMode, WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};
use gpui_kit::*;
use language::text as tr;
use openproject_manager::{self as backend, Appearance, Settings};
use std::sync::mpsc::{self, Receiver};

// App buttons use icons beyond GPUI's smaller default component bundle.
const APP_ASSETS: gpui_kit::assets::AllAssets = gpui_kit::assets::AllAssets;

fn ui_button(id: impl Into<ElementId>, mm: bool) -> Button {
    Button::new(id)
        .h(px(44.))
        .line_height(px(28.))
        .font_family(if mm { "Myanmar Text" } else { "Segoe UI" })
}

// Myanmar Text reserves extra descent for stacked marks. Adjust only the text,
// keeping icons and control boxes on the same geometric center in both languages.
fn control_text(mm: bool, label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .relative()
        .top(px(if mm { 3. } else { 0. }))
        .line_height(px(28.))
        .whitespace_nowrap()
        .child(label.into())
}

fn labeled_button(id: impl Into<ElementId>, mm: bool, label: &'static str) -> Button {
    ui_button(id, mm)
        .accessibility_label(tr(mm, label))
        .child(control_text(mm, tr(mm, label)))
}

fn field_label(mm: bool, label: &'static str) -> impl IntoElement {
    div()
        .h(px(32.))
        .flex()
        .items_center()
        .flex_shrink_0()
        .line_height(px(28.))
        .child(control_text(mm, tr(mm, label)))
}

struct Manager {
    choosing_folder: bool,
    #[cfg(windows)]
    tray: Option<tray::Tray>,
    mm: bool,
    folder: Entity<InputState>,
    host: Entity<InputState>,
    port: Entity<InputState>,
    message: String,
    status: String,
    web_status: String,
    pending: Option<Receiver<anyhow::Result<String>>>,
    statuses: Receiver<(String, String)>,
}
impl Manager {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let loaded = backend::managed_settings();
        let mut message = loaded
            .as_ref()
            .err()
            .map(|e| format!("Cannot load settings: {e}"))
            .unwrap_or_else(|| {
                "Wait for Web: Ready, then click Open OpenProject. Use Start if the service is stopped.".into()
            });
        let mm = backend::load_language().unwrap_or_else(|error| {
            message = format!("Could not load language preference: {error:#}");
            false
        });
        let appearance = backend::load_appearance().unwrap_or_else(|error| {
            message = format!("Could not load appearance preference: {error:#}");
            Appearance::default()
        });
        Theme::change(
            match appearance {
                Appearance::Light => ThemeMode::Light,
                Appearance::Dark => ThemeMode::Dark,
            },
            Some(window),
            cx,
        );
        let settings = loaded.unwrap_or_default();
        #[cfg(windows)]
        let tray = match tray::Tray::new(window, mm) {
            Ok(tray) => {
                tray.hide_on_close(window, cx);
                Some(tray)
            }
            Err(error) => {
                message = format!(
                    "System tray unavailable: {error:#}. Closing the window will exit the manager."
                );
                None
            }
        };
        let folder = cx.new(|cx| {
            InputState::new(window, cx).default_value(settings.folder.to_string_lossy().to_string())
        });
        let host = cx.new(|cx| InputState::new(window, cx).default_value(settings.host));
        let port =
            cx.new(|cx| InputState::new(window, cx).default_value(settings.port.to_string()));
        let (status_tx, statuses) = mpsc::channel();
        std::thread::spawn(move || {
            loop {
                let status = backend::service_status().unwrap_or_else(|e| e.to_string());
                let web = if status == "Running" {
                    backend::managed_settings()
                        .and_then(|settings| backend::web_status(&settings))
                        .unwrap_or_else(|_| "Web: Unavailable".into())
                } else {
                    "Web: Not running".into()
                };
                if status_tx.send((status, web)).is_err() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        });
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                if view
                    .update(cx, |this, cx| {
                        #[cfg(windows)]
                        if let Some(tray) = &mut this.tray {
                            tray.refresh(
                                this.mm,
                                &this.status,
                                this.pending.is_some() || this.choosing_folder,
                            );
                            let actions = tray.events();
                            for action in actions {
                                match action {
                                    tray::Action::Show => this.tray.as_ref().unwrap().show(),
                                    tray::Action::Start => this.run("Start", cx),
                                    tray::Action::Stop => this.run("Stop", cx),
                                    tray::Action::Restart => this.run("Restart", cx),
                                    tray::Action::Exit => {
                                        if this.pending.is_none() && !this.choosing_folder {
                                            this.tray.take();
                                            cx.quit();
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                        while let Ok((status, web)) = this.statuses.try_recv() {
                            if this.status != status || this.web_status != web {
                                this.status = status;
                                this.web_status = web;
                                cx.notify();
                            }
                        }
                        if let Some(rx) = &this.pending {
                            match rx.try_recv() {
                                Ok(result) => {
                                    #[cfg(windows)]
                                    if result.is_err()
                                        && let Some(tray) = &this.tray
                                    {
                                        tray.show();
                                    }
                                    this.message = match result {
                                        Ok(text) => text,
                                        Err(e) => format!("Error: {e:#}"),
                                    };
                                    this.pending = None;
                                    cx.notify();
                                }
                                Err(mpsc::TryRecvError::Disconnected) => {
                                    this.message = "Background operation ended unexpectedly".into();
                                    #[cfg(windows)]
                                    if let Some(tray) = &this.tray {
                                        tray.show();
                                    }
                                    this.pending = None;
                                    cx.notify();
                                }
                                _ => {}
                            }
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            choosing_folder: false,
            #[cfg(windows)]
            tray,
            mm,
            folder,
            host,
            port,
            message,
            status: "Press Refresh to check service".into(),
            web_status: "Web: Checking".into(),
            pending: None,
            statuses,
        }
    }
    fn settings(&self, cx: &App) -> anyhow::Result<Settings> {
        Ok(Settings {
            folder: self.folder.read(cx).value().trim().into(),
            host: self.host.read(cx).value().trim().into(),
            port: self.port.read(cx).value().trim().parse()?,
        })
    }
    fn use_existing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.choosing_folder || self.pending.is_some() {
            return;
        }
        self.choosing_folder = true;
        cx.notify();
        cx.spawn_in(window, async move |view, cx| {
            let result = cx.background_executor().spawn(async { backend::attach_existing() }).await;
            let _ = view.update_in(cx, |this, window, cx| {
                this.choosing_folder = false;
                match result {
                    Ok(settings) => {
                        this.folder.update(cx, |input, cx| input.set_value(settings.folder.to_string_lossy().to_string(), window, cx));
                        this.host.update(cx, |input, cx| input.set_value(settings.host, window, cx));
                        this.port.update(cx, |input, cx| input.set_value(settings.port.to_string(), window, cx));
                        this.message = "Existing service connected. Use Start / Stop / Restart. Configuration was not changed.".into();
                    }
                    Err(error) => this.message = format!("Cannot connect existing service: {error:#}"),
                }
                cx.notify();
            });
        }).detach();
    }
    fn choose_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.choosing_folder || self.pending.is_some() {
            return;
        }
        self.choosing_folder = true;
        cx.notify();
        // GPUI owns the native dialog lifecycle; never run a synchronous modal
        // message loop while holding a mutable Manager/Window update.
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(tr(self.mm, "OpenProject installation folder").into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = selection.await;
            let _ = view.update_in(cx, |this, window, cx| {
                this.choosing_folder = false;
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            this.folder.update(cx, |input, cx| {
                                input.set_value(path.to_string_lossy().to_string(), window, cx)
                            });
                        }
                    }
                    Ok(Ok(None)) => {} // Cancel preserves the current folder.
                    Ok(Err(error)) => {
                        this.message = format!("Cannot open folder picker: {error:#}")
                    }
                    Err(error) => {
                        this.message = format!("Folder picker ended unexpectedly: {error}")
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn run(&mut self, action: &'static str, cx: &mut Context<Self>) {
        if self.pending.is_some() || self.choosing_folder {
            return;
        }
        let selected = if matches!(action, "Save settings" | "Install service") {
            self.settings(cx)
        } else if matches!(action, "Start" | "Stop" | "Restart" | "Remove service") {
            Ok(Settings::default())
        } else {
            backend::managed_settings()
        };
        let settings = match selected {
            Ok(s) => s,
            Err(e) => {
                self.message = format!("Cannot read settings: {e}");
                #[cfg(windows)]
                if let Some(tray) = &self.tray {
                    tray.show();
                }
                cx.notify();
                return;
            }
        };
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.message = format!("{action} …");
        cx.notify();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<String> {
                match action {
                    "Save settings" => {
                        settings.save()?;
                        Ok("Settings saved. Start the service to apply them.".into())
                    }
                    "Install service" => settings.install(),
                    "Start" => backend::control("start"),
                    "Stop" => backend::control("stop"),
                    "Restart" => backend::restart(),
                    "Remove service" => backend::control("delete"),
                    "View log" => backend::read_log(&settings.folder),
                    "Login details" => {
                        open::that(backend::settings_path().with_file_name("login.txt"))?;
                        Ok("Opened the initial login details. A password you changed later is not stored here.".into())
                    }
                    "Open OpenProject" => {
                        let readiness = backend::web_status(&settings).map_err(|_| anyhow::anyhow!("OpenProject web server is unavailable. A running Windows service does not mean OpenProject is ready. View log includes web and worker logs."))?;
                        if !matches!(
                            readiness.as_str(),
                            "Web: Ready" | "Web: Database migration required"
                        ) {
                            anyhow::bail!(
                                "{readiness}. Wait for startup to finish and check View log."
                            );
                        }
                        open::that(settings.url())?;
                        Ok("Opened OpenProject in your browser.".into())
                    }
                    _ => backend::service_status(),
                }
            })();
            let _ = tx.send(result);
        });
    }
}
impl Render for Manager {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.pending.is_some() || self.choosing_folder;
        let mm = self.mm;
        let dark = cx.theme().mode.is_dark();
        let output = if let Some(action) = self.message.strip_suffix(" …") {
            format!("{} …", tr(mm, action))
        } else {
            tr(mm, &self.message).to_owned()
        };
        div().id("manager").size_full().overflow_y_scroll().flex().flex_col().p_8().gap_4()
            .font_family(if mm { "Myanmar Text" } else { "Segoe UI" })
            .line_height(relative(1.8))
            .bg(cx.theme().background).text_color(cx.theme().foreground)
            .child(div().flex().items_center().justify_between()
                .child(div().font_family("Segoe UI").text_3xl().child("OpenProject Manager"))
                .child(div().flex().items_center().gap_2()
                    .child(labeled_button("appearance", mm, if dark { "Light mode" } else { "Dark mode" })
                        .icon(if dark { IconName::Sun } else { IconName::Moon })
                        .on_click(cx.listener(|this, _, window, cx| {
                            let (appearance, mode) = if cx.theme().mode.is_dark() {
                                (Appearance::Light, ThemeMode::Light)
                            } else {
                                (Appearance::Dark, ThemeMode::Dark)
                            };
                            match backend::save_appearance(appearance) {
                                Ok(()) => Theme::change(mode, Some(window), cx),
                                Err(error) => this.message = format!("Could not save appearance preference: {error:#}"),
                            }
                            cx.notify();
                        })))
                    .child(ui_button("language", false).font_family("Segoe UI").label(if mm { "MM → EN" } else { "EN → MM" })
                        .accessibility_label(if mm { "Switch to English" } else { "မြန်မာဘာသာသို့ ပြောင်းရန်" })
                        .on_click(cx.listener(|this, _, _, cx| {
                            let selected = !this.mm;
                            match backend::save_language(selected) {
                                Ok(()) => this.mm = selected,
                                Err(error) => this.message = format!("Could not save language preference: {error:#}"),
                            }
                            cx.notify();
                        })))
                .child(ui_button("instructions", mm).icon(IconName::Info).ghost()
                    .accessibility_label(tr(mm, "Setup instructions")).tooltip(tr(mm, "Setup instructions"))
                    .on_click(move |_, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog.title(tr(mm, "Setup instructions")).width(px(700.)).child(
                                div().id("instructions-content").font_family(if mm { "Myanmar Text" } else { "Segoe UI" }).line_height(relative(1.8)).max_h(px(440.)).overflow_y_scroll().flex().flex_col().gap_4()
                                    .child(tr(mm, "1. Run OpenProjectManager-Setup.exe and approve the administrator prompt. Ruby, PostgreSQL and OpenProject are included. Setup creates a private database and three Windows services. Existing data is kept."))
                                    .child(tr(mm, "2. Wait for Web: Ready, then click Open OpenProject. First startup can take several minutes. Use View log if startup fails, or Start if the service is stopped."))
                                    .child(tr(mm, "3. Click Login details for your generated admin password. The default address is http://127.0.0.1:8088."))
                                    .child(tr(mm, "4. Keep database, attachments and private credentials backed up. Reinstall repairs this bundle; it does not migrate the earlier prototype installation."))
                                    .child(tr(mm, "5. Close this window to keep the manager in the system tray. Double-click its tray icon to reopen it. Right-click for Start, Stop, Restart and Exit. Exit closes only the manager; the service keeps running."))
                                    .child(tr(mm, "To change settings later: Stop the service, save your changes, then Start. To change installation folders, remove the stopped service first."))
                            )
                        });
                    }))))
            .child(div().text_sm().child(tr(mm, "WINDOWS SERVICE · Start automatically when Windows starts")))
            .child(labeled_button("use-existing", mm, "Use existing service").icon(IconName::Link).info()
                .disabled(busy || self.status == "Not installed")
                .on_click(cx.listener(|this, _, window, cx| this.use_existing(window, cx))))
            .child(div().flex().items_center().gap_3().child(labeled_button("refresh", mm, "Refresh status").icon(IconName::RefreshCw).on_click(cx.listener(|this, _, _, cx| {
                this.status = backend::service_status().unwrap_or_else(|e| e.to_string()); cx.notify();
            }))).child(div().h(px(44.)).flex().items_center().child(control_text(mm, format!("{} {} · {}", tr(mm, "Windows service:"), tr(mm, &self.status), tr(mm, &self.web_status))))))
            .child(div().flex().flex_col().gap_2()
                .child(field_label(mm, "OpenProject installation folder"))
                .child(div().flex().items_center().gap_2().child(div().flex_1().min_w_0().child(Input::new(&self.folder).h(px(44.)).font_family("Segoe UI")))
                .child(labeled_button("browse", mm, "Browse…").icon(IconName::FolderOpen).disabled(busy).on_click(cx.listener(|this, _, window, cx| {
                    this.choose_folder(window, cx);
                })))))
            .child(div().flex().items_start().gap_4()
                .child(div().flex().flex_col().gap_2().flex_1().min_w_0().child(field_label(mm, "Host IP")).child(Input::new(&self.host).h(px(44.)).font_family("Segoe UI")))
                .child(div().flex().flex_col().gap_2().w_40().flex_shrink_0().child(field_label(mm, "Port")).child(Input::new(&self.port).h(px(44.)).font_family("Segoe UI"))))
            .child(div().text_sm().child(tr(mm, "127.0.0.1 = this computer only · 0.0.0.0 = all IPv4 interfaces. LAN access needs your firewall configuration.")))
            .child(div().flex().items_center().flex_wrap().gap_2().children(
                ["Save settings", "Install service", "Start", "Stop", "Restart", "Remove service", "Open OpenProject", "Login details", "View log"].into_iter().enumerate().map(|(i, action)| {
                    let button = labeled_button(("action", i), mm, action);
                    let button = match action {
                        "Save settings" => button.icon(IconName::Save).info(),
                        "Install service" => button.icon(IconName::Download).primary(),
                        "Start" => button.icon(IconName::Play).success(),
                        "Stop" => button.icon(IconName::Square).warning(),
                        "Restart" => button.icon(IconName::RefreshCw).info(),
                        "Remove service" => button.icon(IconName::Trash).danger(),
                        "Open OpenProject" => button.icon(IconName::ExternalLink).primary(),
                        _ => button.icon(IconName::FileText).secondary(),
                    };
                    let unavailable = match action {
                        "Install service" => !matches!(self.status.as_str(), "Not installed" | "Incomplete"),
                        "Save settings" => !matches!(self.status.as_str(), "Stopped" | "Not installed" | "Incomplete"),
                        "Start" => !matches!(self.status.as_str(), "Stopped" | "Partially running"),
                        "Stop" | "Restart" => !matches!(self.status.as_str(), "Running" | "Partially running" | "Incomplete"),
                        "Remove service" => !matches!(self.status.as_str(), "Stopped" | "Incomplete"),
                        "Open OpenProject" => !matches!(self.web_status.as_str(), "Web: Ready" | "Web: Database migration required"),
                        _ => false,
                    };
                    button.disabled(busy || unavailable).on_click(cx.listener(move |this, _, _, cx| this.run(action, cx)))
                })
            ))
            .child(div().text_sm().child(tr(mm, "Stop before editing settings. Close (×) hides the manager in the system tray. Right-click the tray icon for controls or Exit.")))
            .child(div().id("output").flex_1().min_h(px(160.)).overflow_y_scroll().p_4().border_1().border_color(cx.theme().border).rounded_md().child(output))
    }
}
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    if matches!(mode.as_str(), "--setup-bundled" | "--remove-services") {
        // Inno Setup already runs elevated and needs a synchronous exit code.
        let result = (|| -> anyhow::Result<String> {
            if mode == "--remove-services" {
                backend::control("stop")?;
                return backend::control("delete");
            }
            let executable = std::env::current_exe()?;
            backend::setup_bundled(
                executable
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("Missing app directory"))?,
            )
        })();
        let path = backend::settings_path().with_file_name("installer.log");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            path,
            match &result {
                Ok(message) => message.clone(),
                Err(error) => format!("{error:#}"),
            },
        );
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    #[cfg(windows)]
    if !elevate() {
        return;
    }
    gpui_kit::application().with_assets(APP_ASSETS).run(|cx| {
        gpui_kit::init(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        gpui_kit::open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("OpenProject Manager".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Maximized(Bounds::centered(
                    None,
                    size(px(1040.), px(760.)),
                    cx,
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Manager::new(window, cx)),
        )
        .expect("open OpenProject Manager");
    });
}

#[cfg(windows)]
fn elevate() -> bool {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn IsUserAnAdmin() -> i32;
        fn ShellExecuteW(
            hwnd: *mut std::ffi::c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }
    // The shell receives separate, nul-terminated strings; no command shell or user arguments.
    if unsafe { IsUserAnAdmin() } != 0 {
        return true;
    }
    let path = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let verb: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result <= 32 {
        rfd::MessageDialog::new().set_title("OpenProject Manager").set_description("Administrator permission is required to manage the OpenProject Windows service. Launch the app again and approve the Windows prompt.").show();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{APP_ASSETS, IconName};
    use gpui_kit::AssetSource;

    #[test]
    fn registered_assets_include_the_complete_icon_catalog() {
        for icon in IconName::ALL {
            let path = icon.path();
            let bytes = APP_ASSETS.load(&path).expect("load bundled icon");
            assert!(
                bytes.is_some_and(|bytes| !bytes.is_empty()),
                "Missing icon: {path}"
            );
        }
    }
}

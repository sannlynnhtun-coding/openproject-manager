use crate::language::text as tr;
use anyhow::Result;
use gpui_kit::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray_icon::{
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

#[derive(Clone, Copy)]
pub enum Action {
    Show,
    Start,
    Stop,
    Restart,
    Exit,
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_tray_icon_can_be_created_and_removed() {
        let icon = tray_icon::Icon::from_resource(1, Some((32, 32)))
            .expect("OpenProject icon resource must be embedded");
        let tray = tray_icon::TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("OpenProject Manager tray verification")
            .build()
            .expect("Windows must register the tray icon");
        drop(tray);
    }
}

pub struct Tray {
    _icon: TrayIcon,
    show: MenuItem,
    start: MenuItem,
    stop: MenuItem,
    restart: MenuItem,
    exit: MenuItem,
    mm: bool,
    hwnd: isize,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn ShowWindow(hwnd: *mut std::ffi::c_void, command: i32) -> i32;
    fn SetForegroundWindow(hwnd: *mut std::ffi::c_void) -> i32;
}

impl Tray {
    pub fn new(window: &Window, mm: bool) -> Result<Self> {
        let handle = HasWindowHandle::window_handle(window)
            .map_err(|error| anyhow::anyhow!("Get manager window handle: {error}"))?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            anyhow::bail!("Expected a Windows window");
        };
        let hwnd = handle.hwnd.get();
        let menu = Menu::new();
        let show = MenuItem::new(tr(mm, "Show manager"), true, None);
        let start = MenuItem::new(tr(mm, "Start"), false, None);
        let stop = MenuItem::new(tr(mm, "Stop"), false, None);
        let restart = MenuItem::new(tr(mm, "Restart"), false, None);
        let exit = MenuItem::new(tr(mm, "Exit"), true, None);
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &start,
            &stop,
            &restart,
            &PredefinedMenuItem::separator(),
            &exit,
        ])?;
        let icon = TrayIconBuilder::new()
            .with_icon(Icon::from_resource(1, Some((32, 32)))?)
            .with_tooltip("OpenProject Manager")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()?;
        Ok(Self {
            _icon: icon,
            show,
            start,
            stop,
            restart,
            exit,
            mm,
            hwnd,
        })
    }
    pub fn hide_on_close(&self, window: &Window, cx: &gpui_kit::App) {
        let hwnd = self.hwnd;
        window.on_window_should_close(cx, move |_, _| {
            // Keep the window/entity and its async tasks alive; hide only this HWND.
            unsafe {
                ShowWindow(hwnd as *mut _, 0);
            }
            false
        });
    }
    pub fn show(&self) {
        unsafe {
            ShowWindow(self.hwnd as *mut _, 3); // SW_SHOWMAXIMIZED
            SetForegroundWindow(self.hwnd as *mut _);
        }
    }
    pub fn refresh(&mut self, mm: bool, status: &str, busy: bool) {
        if self.mm != mm {
            for (item, label) in [
                (&self.show, "Show manager"),
                (&self.start, "Start"),
                (&self.stop, "Stop"),
                (&self.restart, "Restart"),
                (&self.exit, "Exit"),
            ] {
                item.set_text(tr(mm, label));
            }
            self.mm = mm;
        }
        self.start.set_enabled(!busy && status == "Stopped");
        self.stop.set_enabled(!busy && status == "Running");
        self.restart.set_enabled(!busy && status == "Running");
        self.exit.set_enabled(!busy);
    }
    pub fn events(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            for (item, action) in [
                (&self.show, Action::Show),
                (&self.start, Action::Start),
                (&self.stop, Action::Stop),
                (&self.restart, Action::Restart),
                (&self.exit, Action::Exit),
            ] {
                if event.id == *item.id() {
                    actions.push(action);
                }
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                actions.push(Action::Show);
            }
        }
        actions
    }
}

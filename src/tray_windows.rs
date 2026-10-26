//! Notification-area icon via `tray-icon`, driven by a plain Win32 message loop.

use crate::{Status, autostart, combo_labels, icon, poll_loop};
use std::sync::mpsc;
use std::time::Duration;
use std::{mem, ptr, thread};
use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIconBuilder};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, SetTimer, TranslateMessage,
};

fn make_icon(percent: Option<u8>) -> Icon {
    Icon::from_rgba(icon::render(32, percent), 32, 32).expect("valid icon size")
}

pub fn run(interval: Duration) {
    let (refresh_tx, refresh_rx) = mpsc::channel();
    let (status_tx, status_rx) = mpsc::channel::<Status>();
    thread::spawn(move || {
        poll_loop(refresh_rx, interval, |s| {
            let _ = status_tx.send(s);
        })
    });

    let info = MenuItem::new(Status::Pending.text(), false, None);
    let refresh = MenuItem::new("Aggiorna ora", true, None);
    let startup = CheckMenuItem::new("Avvio automatico", true, autostart::is_enabled(), None);
    let quit = MenuItem::new("Esci", true, None);
    let combos: Vec<MenuItem> = combo_labels().map(|label| MenuItem::new(label, false, None)).collect();
    let combo_refs: Vec<&dyn IsMenuItem> = combos.iter().map(|i| i as &dyn IsMenuItem).collect();
    let combos_menu = Submenu::with_items("Combinazioni tasti", true, &combo_refs).expect("menu");
    let menu = Menu::new();
    menu.append_items(&[
        &info,
        &PredefinedMenuItem::separator(),
        &refresh,
        &startup,
        &combos_menu,
        &PredefinedMenuItem::separator(),
        &quit,
    ])
    .expect("menu");

    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(Status::Pending.text())
        .with_icon(make_icon(None))
        .build()
    {
        Ok(t) => t,
        Err(e) => crate::fail(&format!("impossibile creare l'icona nella tray: {e}")),
    };

    // A thread timer wakes the loop regularly to pick up poller and menu events.
    unsafe { SetTimer(ptr::null_mut(), 0, 250, None) };
    let mut msg: MSG = unsafe { mem::zeroed() };
    while unsafe { GetMessageW(&mut msg, ptr::null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        while let Ok(status) = status_rx.try_recv() {
            let _ = tray.set_icon(Some(make_icon(status.percent())));
            let _ = tray.set_tooltip(Some(status.text()));
            info.set_text(status.text());
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == *refresh.id() {
                let _ = refresh_tx.send(());
            } else if event.id == *startup.id() {
                if autostart::set_enabled(startup.is_checked()).is_err() {
                    startup.set_checked(autostart::is_enabled());
                }
            } else if event.id == *quit.id() {
                return;
            }
        }
    }
}

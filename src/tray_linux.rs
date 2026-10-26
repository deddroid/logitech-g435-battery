//! StatusNotifierItem tray (KDE natively; GNOME needs the AppIndicator extension).

use crate::{Status, autostart, combo_labels, icon, poll_loop};
use ksni::blocking::TrayMethods;
use ksni::menu::{CheckmarkItem, StandardItem, SubMenu};
use ksni::{Category, Icon, MenuItem, ToolTip};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

struct G435Tray {
    status: Status,
    refresh: Sender<()>,
}

fn pixmap(size: usize, percent: Option<u8>) -> Icon {
    // SNI wants ARGB32 in network byte order.
    let data = icon::render(size, percent)
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[3], p[0], p[1], p[2]])
        .collect();
    Icon { width: size as i32, height: size as i32, data }
}

impl ksni::Tray for G435Tray {
    fn id(&self) -> String {
        "g435-tray".into()
    }

    fn title(&self) -> String {
        self.status.text()
    }

    fn category(&self) -> Category {
        Category::Hardware
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let p = self.status.percent();
        vec![pixmap(32, p), pixmap(64, p)]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "Logitech G435".into(),
            description: self.status.text(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem { label: self.status.text(), enabled: false, ..Default::default() }.into(),
            MenuItem::Separator,
            SubMenu {
                label: "Comandi della cuffia".into(),
                icon_name: "audio-headset".into(),
                submenu: combo_labels()
                    .map(|label| StandardItem { label, enabled: false, ..Default::default() }.into())
                    .collect(),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Aggiorna ora".into(),
                icon_name: "view-refresh".into(),
                activate: Box::new(|t: &mut Self| {
                    t.status = Status::Pending;
                    let _ = t.refresh.send(());
                }),
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: "Avvio automatico".into(),
                checked: autostart::is_enabled(),
                activate: Box::new(|_: &mut Self| {
                    if let Err(e) = autostart::set_enabled(!autostart::is_enabled()) {
                        eprintln!("g435-tray: avvio automatico: {e}");
                    }
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Esci".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_: &mut Self| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn run(interval: Duration) {
    // Re-checked periodically: the control goes back to 100% when the dongle
    // is plugged in again.
    std::thread::spawn(|| {
        loop {
            crate::audio::unblock_volume_up();
            std::thread::sleep(Duration::from_secs(5));
        }
    });

    let (refresh_tx, refresh_rx) = mpsc::channel();
    let tray = G435Tray { status: Status::Pending, refresh: refresh_tx };
    let handle = match tray.spawn() {
        Ok(h) => h,
        Err(e) => crate::fail(&format!("impossibile creare l'icona nella tray: {e}")),
    };
    poll_loop(refresh_rx, interval, |status| {
        handle.update(|t| t.status = status);
    });
}

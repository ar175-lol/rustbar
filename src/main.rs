pub mod bar;
pub mod config;
pub mod modules;

use bar::Bar;
use modules::audio::AudioModule;
use modules::battery::BatteryModule;
use modules::bluetooth::BluetoothModule;
use modules::clock::ClockModule;
use modules::workspaces::WorkspacesModule;

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, Layer, LayerShell},
    },
    shm::slot::SlotPool,
};
use std::os::fd::AsFd;
use wayland_client::{Connection, globals::registry_queue_init};

use crate::config::{BAR_HEIGHT, BAR_WIDTH};

pub enum AppEvent {
    Niri(niri_ipc::Event),
    Battery(String),
    Bluetooth { text: String, is_on: bool },
    Audio(modules::audio::AudioState),
}

fn main() {
    if let Err(e) = config::validate_config() {
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let (tx_bar, rx_bar) = calloop::channel::channel::<AppEvent>();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("layer_shell is not available");
    let surface = compositor.create_surface(&qh);

    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Top,
        Some("rustbar".to_string()),
        None,
    );

    layer.set_anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
    layer.set_size(config::BAR_WIDTH, config::BAR_HEIGHT);
    layer.set_exclusive_zone(BAR_HEIGHT as i32);
    layer.commit();

    let shm = smithay_client_toolkit::shm::Shm::bind(&globals, &qh).unwrap();

    let mut simple_bar = Bar {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        pool: SlotPool::new(BAR_WIDTH as usize * BAR_HEIGHT as usize * 4, &shm).unwrap(),
        shm,
        first_configure: true,
        width: BAR_WIDTH,
        height: config::BAR_HEIGHT,
        layer,
        clock_module: ClockModule::new(),
        workspaces_module: WorkspacesModule::new(),
        battery_module: BatteryModule::new(),
        bluetooth_module: BluetoothModule::new(),
        audio_module: AudioModule::new(),
        audio_tx: tx_bar.clone(),
        pointer: None,
        rx: rx_bar,
        buffers: None,
        needs_redraw: false,
    };

    let tx_niri = tx_bar.clone();
    std::thread::spawn(move || {
        if let Ok(mut socket) = niri_ipc::socket::Socket::connect() {
            if let Ok(Ok(_)) = socket.send(niri_ipc::Request::EventStream) {
                let mut read_event = socket.read_events();
                while let Ok(event) = read_event() {
                    let _ = tx_niri.send(AppEvent::Niri(event));
                }
            }
        }
    });

    let tx_audio = tx_bar.clone();
    std::thread::spawn(move || {
        let mut last_vol = 0;
        let mut last_muted = false;

        loop {
            if let Some(state) = crate::modules::audio::query_volume() {
                if state.volume != last_vol || state.is_muted != last_muted {
                    last_vol = state.volume;
                    last_muted = state.is_muted;
                    let _ = tx_audio.send(AppEvent::Audio(state));
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    let tx_bat = tx_bar.clone();
    std::thread::spawn(move || {
        let mut last_text = String::new();
        loop {
            if let (Ok(capacity_str), Ok(status_str)) = (
                std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity"),
                std::fs::read_to_string("/sys/class/power_supply/BAT0/status"),
            ) {
                if let Ok(pct) = capacity_str.trim().parse::<u32>() {
                    let is_charging = status_str.trim() == "Charging";
                    let icon = if is_charging { "󱐋 " } else { "󰁹 " };
                    let current_text = format!("{}{}%", icon, pct);

                    if current_text != last_text {
                        last_text = current_text.clone();
                        let _ = tx_bat.send(AppEvent::Battery(current_text));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });

    let tx_bt = tx_bar.clone();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            if let Ok(session) = bluer::Session::new().await {
                let mut last_text = String::new();
                let mut last_is_on = false;
                let mut first_run = true;

                loop {
                    let mut text = "󰂲".to_string();
                    let mut is_on = false;

                    if let Ok(adapter) = session.default_adapter().await {
                        if let Ok(powered) = adapter.is_powered().await {
                            if powered {
                                is_on = true;
                                text = "󰂯".to_string();

                                if let Ok(devices) = adapter.device_addresses().await {
                                    for addr in devices {
                                        if let Ok(device) = adapter.device(addr) {
                                            if let Ok(true) = device.is_connected().await {
                                                text = "󰂱".to_string();
                                                if let Ok(Some(battery)) =
                                                    device.battery_percentage().await
                                                {
                                                    text = format!("󰂯 {}%", battery);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if first_run || text != last_text || is_on != last_is_on {
                        last_text = text.clone();
                        last_is_on = is_on;
                        first_run = false;
                        let _ = tx_bt.send(AppEvent::Bluetooth { text, is_on });
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        });
    });

    let mut event_loop = calloop::EventLoop::<Bar>::try_new().unwrap();
    let loop_handle = event_loop.handle();

    let qh_clone = qh.clone();

    let (_, dummy_rx) = calloop::channel::channel::<AppEvent>();
    let rx_source = std::mem::replace(&mut simple_bar.rx, dummy_rx);

    let _ = loop_handle.insert_source(rx_source, move |event, _, bar| match event {
        calloop::channel::Event::Msg(AppEvent::Audio(state)) => {
            bar.audio_module.apply(state);
            bar.draw(&qh_clone);
        }
        calloop::channel::Event::Msg(AppEvent::Niri(niri_ipc::Event::WorkspacesChanged {
            workspaces,
        })) => {
            bar.workspaces_module.workspaces = workspaces;
            bar.draw(&qh_clone);
        }
        calloop::channel::Event::Msg(AppEvent::Niri(niri_ipc::Event::WorkspaceActivated {
            id,
            focused,
        })) => {
            let target_output = bar
                .workspaces_module
                .workspaces
                .iter()
                .find(|w| w.id == id)
                .and_then(|w| w.output.clone());

            for ws in bar.workspaces_module.workspaces.iter_mut() {
                if ws.id == id {
                    ws.is_active = true;
                    if focused {
                        ws.is_focused = true;
                    }
                } else {
                    if ws.output == target_output {
                        ws.is_active = false;
                    }
                    if focused {
                        ws.is_focused = false;
                    }
                }
            }
            bar.draw(&qh_clone);
        }
        calloop::channel::Event::Msg(AppEvent::Battery(text)) => {
            bar.battery_module.text = text;
            bar.draw(&qh_clone);
        }
        calloop::channel::Event::Msg(AppEvent::Bluetooth { text, is_on }) => {
            bar.bluetooth_module.text = text;
            bar.bluetooth_module.is_on = is_on;
            bar.draw(&qh_clone);
        }
        _ => {}
    });

    let conn_fd = conn.as_fd();
    let _ = event_loop.handle().insert_source(
        calloop::generic::Generic::new(conn_fd, calloop::Interest::READ, calloop::Mode::Level),
        move |_, _, bar| {
            if let Some(guard) = event_queue.prepare_read() {
                if let Err(e) = guard.read() {
                    eprintln!("Error reading from Wayland socket: {}", e);
                }
            }
            event_queue.dispatch_pending(bar).unwrap();
            Ok(calloop::PostAction::Continue)
        },
    );

    loop {
        conn.flush().unwrap();
        event_loop.dispatch(None, &mut simple_bar).unwrap();
    }
}

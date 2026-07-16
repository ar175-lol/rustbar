use crate::config::{self, FONT_NAME, FONT_WEIGHT};
use crate::modules::audio::AudioModule;
use crate::modules::battery::BatteryModule;
use crate::modules::bluetooth::BluetoothModule;
use crate::modules::clock::ClockModule;
use crate::modules::workspaces::WorkspacesModule;
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{BTN_LEFT, PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};

use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
};

pub struct Buffers {
    buffers: [smithay_client_toolkit::shm::slot::Buffer; 2],
    current: usize,
}

impl Buffers {
    fn new(pool: &mut SlotPool, width: u32, height: u32, format: wl_shm::Format) -> Self {
        let stride = width as i32 * 4;
        Self {
            buffers: [
                pool.create_buffer(width as i32, height as i32, stride, format)
                    .expect("create buffer 1")
                    .0,
                pool.create_buffer(width as i32, height as i32, stride, format)
                    .expect("create buffer 2")
                    .0,
            ],
            current: 0,
        }
    }

    fn flip(&mut self) {
        self.current = 1 - self.current;
    }

    fn buffer(&self) -> &smithay_client_toolkit::shm::slot::Buffer {
        &self.buffers[self.current]
    }

    fn canvas<'a>(&'a self, pool: &'a mut SlotPool) -> Option<&'a mut [u8]> {
        self.buffers[self.current].canvas(pool)
    }
}

pub struct Bar {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,
    pub shm: Shm,
    pub first_configure: bool,
    pub pool: SlotPool,
    pub width: u32,
    pub height: u32,
    pub layer: LayerSurface,
    pub clock_module: ClockModule,
    pub workspaces_module: WorkspacesModule,
    pub battery_module: BatteryModule,
    pub rx: calloop::channel::Channel<crate::AppEvent>,
    pub bluetooth_module: BluetoothModule,
    pub audio_module: AudioModule,
    pub audio_tx: calloop::channel::Sender<crate::AppEvent>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub buffers: Option<Buffers>,
    pub needs_redraw: bool,
}

impl Bar {
    pub fn draw(&mut self, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;

        if let Some(ref mut buffers) = self.buffers {
            // If the current buffer hasn't been released by the compositor
            // yet, try the other one before giving up -- that's the whole
            // point of double-buffering. The old code bailed out (`None =>
            // return`) on the very first busy buffer without calling
            // flip() or re-requesting the frame callback below, so a single
            // transient stall permanently desynced `current` and killed
            // the self-sustaining frame loop. A burst of rapid workspace
            // switches -- several WorkspaceActivated events firing draw()
            // faster than the compositor releases a 2-frame-old buffer,
            // right when the compositor is busiest doing its own
            // workspace-switch animation -- is exactly the condition most
            // likely to trigger that stall. Over hours of normal use it
            // compounds, since nothing was left to retry until some
            // unrelated event (battery/bluetooth/audio) happened to call
            // draw() again on that same still-maybe-busy slot.
            if buffers.canvas(&mut self.pool).is_none() {
                buffers.flip();
            }

            let canvas = match buffers.canvas(&mut self.pool) {
                Some(c) => c,
                None => {
                    // Both buffers are still busy -- genuinely nothing to
                    // draw into right now. Re-arm the frame callback so we
                    // get another chance next vblank instead of leaving
                    // the surface with nothing scheduled.
                    self.layer
                        .wl_surface()
                        .frame(qh, self.layer.wl_surface().clone());
                    self.layer.commit();
                    return;
                }
            };

            let mut surface =
                cairo::ImageSurface::create(cairo::Format::ARgb32, width as i32, height as i32)
                    .unwrap();

            let cr = cairo::Context::new(&surface).unwrap();

            cr.set_source_rgb(
                config::BACKGROUND_COLOR.r,
                config::BACKGROUND_COLOR.g,
                config::BACKGROUND_COLOR.b,
            );
            cr.paint().unwrap();

            cr.select_font_face(FONT_NAME, cairo::FontSlant::Normal, FONT_WEIGHT);
            cr.set_font_size(config::FONT_SIZE);

            let bar_height_f64 = height as f64;

            self.clock_module.draw(&cr, bar_height_f64);

            let mut right_edge = width as f64 - config::ITEM_PADDING;
            right_edge = self.battery_module.draw(&cr, bar_height_f64, right_edge);
            right_edge = self.audio_module.draw(&cr, bar_height_f64, right_edge);
            self.bluetooth_module.draw(&cr, bar_height_f64, right_edge);
            self.workspaces_module.draw(&cr, bar_height_f64, width);
            drop(cr);

            let data = surface.data().unwrap();
            canvas.copy_from_slice(&data);

            self.layer
                .wl_surface()
                .damage_buffer(0, 0, width as i32, height as i32);

            buffers
                .buffer()
                .attach_to(self.layer.wl_surface())
                .expect("buffer attach");

            self.layer
                .wl_surface()
                .frame(qh, self.layer.wl_surface().clone());

            self.layer.commit();

            buffers.flip();
        }
    }
}

impl LayerShellHandler for Bar {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        std::process::exit(0);
    }

    fn configure(
        &mut self,
        _c: &Connection,
        qh: &QueueHandle<Self>,
        _l: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _s: u32,
    ) {
        use std::num::NonZeroU32;
        self.width = NonZeroU32::new(configure.new_size.0).map_or(1920, NonZeroU32::get);
        self.height =
            NonZeroU32::new(configure.new_size.1).map_or(config::BAR_HEIGHT, NonZeroU32::get);

        self.buffers = Some(Buffers::new(
            &mut self.pool,
            self.width,
            self.height,
            wl_shm::Format::Argb8888,
        ));

        self.first_configure = false;
        self.draw(qh);
    }
}

impl CompositorHandler for Bar {
    fn scale_factor_changed(
        &mut self,
        _c: &Connection,
        _q: &QueueHandle<Self>,
        _s: &wl_surface::WlSurface,
        _f: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _c: &Connection,
        _q: &QueueHandle<Self>,
        _s: &wl_surface::WlSurface,
        _t: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _c: &Connection,
        qh: &QueueHandle<Self>,
        _s: &wl_surface::WlSurface,
        _t: u32,
    ) {
        if self.clock_module.update() {
            self.draw(qh);
        } else {
            self.layer
                .wl_surface()
                .frame(qh, self.layer.wl_surface().clone());
            self.layer.commit();
        }
    }
}

impl OutputHandler for Bar {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _c: &Connection, _q: &QueueHandle<Self>, _o: wl_output::WlOutput) {}
    fn update_output(&mut self, _c: &Connection, _q: &QueueHandle<Self>, _o: wl_output::WlOutput) {}
    fn output_destroyed(
        &mut self,
        _c: &Connection,
        _q: &QueueHandle<Self>,
        _o: wl_output::WlOutput,
    ) {
    }
}

impl ShmHandler for Bar {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl SeatHandler for Bar {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _c: &Connection, _q: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _c: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(pointer) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(pointer);
            }
        }
    }

    fn remove_capability(
        &mut self,
        _c: &Connection,
        _q: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
    }

    fn remove_seat(&mut self, _c: &Connection, _q: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}
}

impl PointerHandler for Bar {
    fn pointer_frame(
        &mut self,
        _c: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        let mut needs_redraw = false;

        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }

            match event.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                    let (x, y) = event.position;
                    let now_hovered = self.audio_module.hit_test(x, y);
                    if now_hovered != self.audio_module.is_hovered {
                        self.audio_module.is_hovered = now_hovered;
                        needs_redraw = true;
                    }
                }
                PointerEventKind::Leave { .. } => {
                    if self.audio_module.is_hovered {
                        self.audio_module.is_hovered = false;
                        needs_redraw = true;
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    let (x, y) = event.position;
                    if button == BTN_LEFT && self.audio_module.hit_test(x, y) {
                        crate::modules::audio::toggle_mute(self.audio_tx.clone());
                    }
                }
                PointerEventKind::Axis { vertical, .. } => {
                    let (x, y) = event.position;
                    if self.audio_module.hit_test(x, y) {
                        let scrolled_down = if vertical.discrete != 0 {
                            vertical.discrete > 0
                        } else {
                            vertical.absolute > 0.0
                        };
                        crate::modules::audio::step_volume(self.audio_tx.clone(), !scrolled_down);
                    }
                }
                PointerEventKind::Press { .. } => {}
            }
        }

        if needs_redraw {
            self.draw(qh);
        }
    }
}

delegate_registry!(Bar);
impl ProvidesRegistryState for Bar {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(Bar);
delegate_layer!(Bar);
delegate_shm!(Bar);
delegate_output!(Bar);
delegate_seat!(Bar);
delegate_pointer!(Bar);

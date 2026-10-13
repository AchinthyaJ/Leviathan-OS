//! Wayland backend for leviathan-ui.
//!
//! When `WAYLAND_DISPLAY` is set (i.e. running under Hyprland), this module
//! presents the CPU backbuffer as a wlr-layer-shell surface that covers the
//! full screen (OVERLAY layer, exclusive-zone -1).  Pointer and keyboard
//! events are dispatched back to the rest of the UI via [`WaylandPointer`]
//! and [`WaylandKeyEvent`].

use std::{
    collections::VecDeque,
    fs::File,
    os::unix::io::{AsFd, AsRawFd, FromRawFd, OwnedFd},
};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_keyboard::{self, WlKeyboard},
        wl_pointer::{self, WlPointer},
        wl_registry::WlRegistry,
        wl_seat::WlSeat,
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, QueueHandle, WEnum,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, ZwlrLayerSurfaceV1},
};

// ─── Key events ──────────────────────────────────────────────────────────────

/// Raw Wayland key code (Linux evdev code, NOT xkb keysym).
#[derive(Debug, Clone, Copy)]
pub struct WaylandKeyEvent {
    /// Raw Linux evdev keycode (same values as `input-event-codes.h`).
    pub code: u32,
    pub pressed: bool,
}

// ─── Pointer state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct WaylandPointer {
    pub x: f64,
    pub y: f64,
    pub left_down: bool,
    pub right_down: bool,
    pub middle_down: bool,
    // Transient — cleared at the start of every frame
    pub left_clicked: bool,
    pub right_clicked: bool,
    pub middle_clicked: bool,
    pub left_released: bool,
    pub right_released: bool,
    pub scroll_delta: i32,
}

impl WaylandPointer {
    /// Clear per-frame transient fields.
    pub fn clear_transient(&mut self) {
        self.left_clicked = false;
        self.right_clicked = false;
        self.middle_clicked = false;
        self.left_released = false;
        self.right_released = false;
        self.scroll_delta = 0;
    }
}

// ─── Shared-memory double buffer ─────────────────────────────────────────────

pub(crate) struct ShmBuf {
    _file: File, // keep alive
    _pool: WlShmPool,
    buffers: [WlBuffer; 2],
    ptr: *mut u8,
    frame_size: usize, // bytes per frame
    front: usize,      // index of the buffer currently being filled (0 or 1)
}

unsafe impl Send for ShmBuf {}

impl ShmBuf {
    fn current_buf(&self) -> &WlBuffer {
        &self.buffers[self.front]
    }
    fn current_offset(&self) -> usize {
        self.front * self.frame_size
    }
    fn flip(&mut self) {
        self.front ^= 1;
    }
}

impl Drop for ShmBuf {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                libc::munmap(self.ptr as *mut libc::c_void, self.frame_size * 2);
            }
        }
    }
}

// ─── Compositor state ────────────────────────────────────────────────────────

pub struct State {
    pub compositor: Option<WlCompositor>,
    pub shm: Option<WlShm>,
    pub layer_shell: Option<ZwlrLayerShellV1>,
    pub seat: Option<WlSeat>,
    pub surface: Option<WlSurface>,
    pub layer_surface: Option<ZwlrLayerSurfaceV1>,
    pub shm_buf: Option<ShmBuf>,
    pub pointer: WaylandPointer,
    pub keys: VecDeque<WaylandKeyEvent>,
    pub configured: bool,
    pub width: u32,
    pub height: u32,
    pub done: bool,
}

impl State {
    fn new(fallback_w: u32, fallback_h: u32) -> Self {
        State {
            compositor: None,
            shm: None,
            layer_shell: None,
            seat: None,
            surface: None,
            layer_surface: None,
            shm_buf: None,
            pointer: WaylandPointer::default(),
            keys: VecDeque::new(),
            configured: false,
            width: fallback_w,
            height: fallback_h,
            done: false,
        }
    }
}

// ─── Registry dispatch ────────────────────────────────────────────────────────

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _registry: &WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Globals are bound via registry_queue_init; this handles dynamic add/remove.
    }
}

// ─── Noop dispatches ──────────────────────────────────────────────────────────

delegate_noop!(State: ignore WlCompositor);
delegate_noop!(State: ignore WlShm);
delegate_noop!(State: ignore WlShmPool);
delegate_noop!(State: ignore WlBuffer);
delegate_noop!(State: ignore WlSurface);
delegate_noop!(State: ignore ZwlrLayerShellV1);

// ─── Layer surface dispatch ───────────────────────────────────────────────────

impl Dispatch<ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        ls: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                ls.ack_configure(serial);
                if width > 0 {
                    state.width = width;
                }
                if height > 0 {
                    state.height = height;
                }
                state.configured = true;
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.done = true;
            }
            _ => {}
        }
    }
}

// ─── Seat dispatch ────────────────────────────────────────────────────────────

impl Dispatch<WlSeat, ()> for State {
    fn event(
        _state: &mut Self,
        seat: &WlSeat,
        event: wayland_client::protocol::wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_seat::Event::Capabilities { capabilities } = event {
            use wayland_client::protocol::wl_seat::Capability;
            let caps = match capabilities {
                WEnum::Value(c) => c,
                _ => return,
            };
            if caps.contains(Capability::Pointer) {
                seat.get_pointer(qh, ());
            }
            if caps.contains(Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
        }
    }
}

// ─── Pointer dispatch ─────────────────────────────────────────────────────────

impl Dispatch<WlPointer, ()> for State {
    fn event(
        state: &mut Self,
        _pointer: &WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                state.pointer.x = surface_x;
                state.pointer.y = surface_y;
            }
            wl_pointer::Event::Button { button, state: btn_state, .. } => {
                use wayland_client::protocol::wl_pointer::ButtonState;
                let pressed =
                    matches!(btn_state, WEnum::Value(ButtonState::Pressed));
                match button {
                    0x110 => {
                        // BTN_LEFT
                        if pressed && !state.pointer.left_down {
                            state.pointer.left_clicked = true;
                        }
                        if !pressed && state.pointer.left_down {
                            state.pointer.left_released = true;
                        }
                        state.pointer.left_down = pressed;
                    }
                    0x111 => {
                        // BTN_RIGHT
                        if pressed && !state.pointer.right_down {
                            state.pointer.right_clicked = true;
                        }
                        state.pointer.right_down = pressed;
                    }
                    0x112 => {
                        // BTN_MIDDLE
                        if pressed && !state.pointer.middle_down {
                            state.pointer.middle_clicked = true;
                        }
                        state.pointer.middle_down = pressed;
                    }
                    _ => {}
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                use wayland_client::protocol::wl_pointer::Axis;
                if matches!(axis, WEnum::Value(Axis::VerticalScroll)) {
                    if value > 0.0 {
                        state.pointer.scroll_delta -= 1;
                    } else {
                        state.pointer.scroll_delta += 1;
                    }
                }
            }
            _ => {}
        }
    }
}

// ─── Keyboard dispatch ────────────────────────────────────────────────────────

impl Dispatch<WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _kb: &WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We only care about key press/release.  The `key` field from Wayland
        // is the Linux evdev code (scancode - 8 already handled by Wayland),
        // so it matches directly against the constants in input.rs.
        if let wl_keyboard::Event::Key { key, state: key_state, .. } = event {
            use wayland_client::protocol::wl_keyboard::KeyState;
            let pressed = matches!(key_state, WEnum::Value(KeyState::Pressed));
            state.keys.push_back(WaylandKeyEvent { code: key, pressed });
        }
    }
}

// ─── Public backend ───────────────────────────────────────────────────────────

pub struct WaylandBackend {
    conn: Connection,
    eq: EventQueue<State>,
    pub state: State,
}

impl WaylandBackend {
    /// Connect to the Wayland display and create a full-screen layer-shell
    /// surface.  `fallback_w`/`fallback_h` are the dimensions used if the
    /// compositor does not send a configure event in the initial roundtrip.
    pub fn new(fallback_w: u32, fallback_h: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::connect_to_env()?;
        let (globals, eq) = registry_queue_init::<State>(&conn)?;
        let qh = eq.handle();

        let mut state = State::new(fallback_w, fallback_h);

        // Bind the globals we need.
        let compositor: WlCompositor = globals.bind(&qh, 1..=4, ())?;
        let shm: WlShm = globals.bind(&qh, 1..=1, ())?;
        let layer_shell: ZwlrLayerShellV1 = globals.bind(&qh, 1..=4, ())?;
        let seat: WlSeat = globals.bind(&qh, 1..=7, ())?;

        state.compositor = Some(compositor.clone());
        state.shm = Some(shm.clone());
        state.layer_shell = Some(layer_shell.clone());
        state.seat = Some(seat.clone());

        // Get pointer and keyboard immediately so we don't miss the capabilities event.
        // seat.get_pointer/get_keyboard requires capabilities; request them after roundtrip.

        // Create the wl_surface and layer surface.
        let surface = compositor.create_surface(&qh, ());
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            "leviathan".to_string(),
            &qh,
            (),
        );

        // Anchor to all four edges → fills the entire output.
        layer_surface.set_anchor(
            Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        );
        // Exclusive zone -1: do not push other surfaces aside.
        layer_surface.set_exclusive_zone(-1);
        // size 0,0 with all anchors = compositor decides the size.
        layer_surface.set_size(0, 0);
        layer_surface.set_keyboard_interactivity(
            zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
        );
        surface.commit();

        state.surface = Some(surface);
        state.layer_surface = Some(layer_surface);

        let mut backend = WaylandBackend { conn, eq, state };

        // Two roundtrips: first collects globals + seat capabilities, second
        // receives the layer-surface configure.
        backend.eq.roundtrip(&mut backend.state)?;
        backend.eq.roundtrip(&mut backend.state)?;

        // Allocate shm buffers now that we know the configured size.
        let w = backend.state.width;
        let h = backend.state.height;
        if w == 0 || h == 0 {
            return Err(format!("Compositor sent zero-size configure ({}x{})", w, h).into());
        }
        backend.alloc_shm(w, h)?;

        println!(
            "leviathan-ui: Wayland backend ready ({}x{}, layer-shell OVERLAY)",
            w, h
        );
        Ok(backend)
    }

    fn alloc_shm(&mut self, w: u32, h: u32) -> Result<(), Box<dyn std::error::Error>> {
        let stride = w as usize * 4; // XRGB8888 → 4 bytes/px
        let frame_size = stride * h as usize;
        let total = frame_size * 2; // double buffer

        // Create an anonymous fd via memfd_create (preferred) or a tempfile.
        let fd: OwnedFd = create_anon_fd(total)?;
        let file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
        // Keep the OwnedFd alive so the fd isn't closed.
        let _owned = fd;

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                total,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        } as *mut u8;
        if ptr as isize == -1 {
            return Err("wl_shm mmap failed".into());
        }

        let shm = self.state.shm.as_ref().ok_or("wl_shm not available")?;
        let qh = self.eq.handle();

        let pool = shm.create_pool(file.as_fd(), total as i32, &qh, ());

        let buf0 = pool.create_buffer(
            0,
            w as i32,
            h as i32,
            stride as i32,
            wayland_client::protocol::wl_shm::Format::Xrgb8888,
            &qh,
            (),
        );
        let buf1 = pool.create_buffer(
            frame_size as i32,
            w as i32,
            h as i32,
            stride as i32,
            wayland_client::protocol::wl_shm::Format::Xrgb8888,
            &qh,
            (),
        );

        self.state.shm_buf = Some(ShmBuf {
            _file: file,
            _pool: pool,
            buffers: [buf0, buf1],
            ptr,
            frame_size,
            front: 0,
        });
        Ok(())
    }

    /// Copy the BGRA/BGRX `backbuffer` into the front shm buffer and commit
    /// the surface.  The pixel layout BGRA matches wl_shm `Xrgb8888` on
    /// little-endian (B=byte0, G=byte1, R=byte2, X=byte3).
    pub fn present(&mut self, backbuffer: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let buf = self.state.shm_buf.as_mut().ok_or("shm not allocated")?;
        let offset = buf.current_offset();
        let len = buf.frame_size.min(backbuffer.len());
        unsafe {
            std::ptr::copy_nonoverlapping(
                backbuffer.as_ptr(),
                buf.ptr.add(offset),
                len,
            );
        }
        let wl_buf = buf.current_buf().clone();
        buf.flip();

        let surface = self.state.surface.as_ref().ok_or("no surface")?;
        surface.attach(Some(&wl_buf), 0, 0);
        surface.damage_buffer(
            0,
            0,
            self.state.width as i32,
            self.state.height as i32,
        );
        surface.commit();
        self.conn.flush()?;
        Ok(())
    }

    /// Dispatch pending Wayland events non-blockingly.
    ///
    /// Clears per-frame pointer transients first, then drains anything that
    /// arrived since last call.  Returns `false` if the connection is broken
    /// or the compositor closed our surface.
    pub fn poll_events(&mut self) -> bool {
        self.state.pointer.clear_transient();

        // Dispatch what is already in the queue.
        if self.eq.dispatch_pending(&mut self.state).is_err() {
            return false;
        }

        // Do a non-blocking poll on the Wayland fd.
        let raw_fd = self.conn.as_fd().as_raw_fd();
        let mut pfd = libc::pollfd { fd: raw_fd, events: libc::POLLIN, revents: 0 };
        unsafe { libc::poll(&mut pfd as *mut _, 1, 0) };

        if pfd.revents & libc::POLLIN != 0 {
            if let Some(guard) = self.conn.prepare_read() {
                let _ = guard.read();
            }
            if self.eq.dispatch_pending(&mut self.state).is_err() {
                return false;
            }
        }

        !self.state.done
    }

    pub fn width(&self) -> u32 { self.state.width }
    pub fn height(&self) -> u32 { self.state.height }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Create an anonymous, unlinked file descriptor of `size` bytes.
fn create_anon_fd(size: usize) -> Result<OwnedFd, Box<dyn std::error::Error>> {
    // Try memfd_create first (Linux 3.17+).
    let name = c"leviathan-shm";
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd >= 0 {
        unsafe { libc::ftruncate(fd, size as libc::off_t) };
        return Ok(unsafe { OwnedFd::from_raw_fd(fd) });
    }

    // Fallback: O_TMPFILE on /dev/shm or /tmp.
    for dir in &["/dev/shm", "/tmp"] {
        let path = format!("{}\0", dir);
        let fd = unsafe {
            libc::open(
                path.as_ptr() as *const libc::c_char,
                libc::O_TMPFILE | libc::O_RDWR | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd >= 0 {
            unsafe { libc::ftruncate(fd, size as libc::off_t) };
            return Ok(unsafe { OwnedFd::from_raw_fd(fd) });
        }
    }
    Err("could not create anonymous shm fd".into())
}

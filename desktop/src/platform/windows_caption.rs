//! A native caption revealed by translating page containers without reflow.
//! A cached alpha surface exposes the owner's acrylic. Native buttons retain
//! keyboard/accessibility behavior on a separate, nearly transparent input host.
#[path = "windows_caption_paint.rs"]
mod paint;
use tauri::WebviewWindow;
use windows::core::w;
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{
        Controls::*,
        HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
        Input::KeyboardAndMouse::*,
        Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
const ID: usize = 0x5a434150;
const HIDE: usize = ID + 1;
const FADE: usize = ID + 2;
const CLOSE_SETTLED: usize = ID + 3;
struct State {
    owner: HWND,
    strip: HWND,
    edge: HWND,
    buttons: [HWND; 3],
    expanded: bool,
    suppressed: bool,
    page_left: Option<f64>,
    window: WebviewWindow,
    previous_focus: HWND,
    laying_out: bool,
    hot_button: HWND,
    sizing: bool,
    changing_state: bool,
    opacity: u8,
    fade_from: u8,
    fade_started: std::time::Instant,
    pressed_button: HWND,
    drag_start: Option<POINT>,
    motion_paused: Option<std::time::Instant>,
    surface: Option<paint::Surface>,
    geometry: Option<(i32, i32, i32, i32, i32, u32)>,
    render_failed: bool,
}

pub fn install(window: &WebviewWindow) -> bool {
    let Ok(owner) = window.hwnd() else {
        return false;
    };
    // SAFETY: creation and subclass installation run on Tauri's UI thread.
    unsafe {
        let owner = HWND(owner.0);
        let Ok(strip) = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT,
            w!("STATIC"),
            w!(""),
            WS_POPUP,
            0,
            0,
            0,
            0,
            Some(owner),
            None,
            None,
            None,
        ) else {
            return false;
        };
        // Alpha zero is excluded from layered-window hit testing. One alpha unit
        // retains pointer events without a visible border or painted caption rail.
        let Ok(edge) = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            w!("STATIC"),
            w!(""),
            WS_POPUP | WS_CLIPCHILDREN | WINDOW_STYLE(0x0100),
            0,
            0,
            0,
            0,
            Some(owner),
            None,
            None,
            None,
        ) else {
            let _ = DestroyWindow(strip);
            return false;
        };
        let _ = SetLayeredWindowAttributes(edge, COLORREF(0), 1, LWA_ALPHA);
        let Some(surface) = paint::Surface::new(GetDpiForWindow(owner)) else {
            let _ = DestroyWindow(edge);
            let _ = DestroyWindow(strip);
            return false;
        };
        let mut buttons = [HWND::default(); 3];
        for (index, title) in [w!("Minimize"), w!("Maximize or restore"), w!("Close")]
            .into_iter()
            .enumerate()
        {
            let Ok(button) = CreateWindowExW(
                WS_EX_NOPARENTNOTIFY,
                w!("BUTTON"),
                title,
                WS_CHILD | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                0,
                0,
                0,
                0,
                Some(edge),
                Some(HMENU((index + 1) as *mut _)),
                None,
                None,
            ) else {
                let _ = DestroyWindow(edge);
                let _ = DestroyWindow(strip);
                return false;
            };
            buttons[index] = button;
        }
        let pointer = Box::into_raw(Box::new(State {
            owner,
            strip,
            edge,
            buttons,
            expanded: false,
            suppressed: false,
            page_left: None,
            window: window.clone(),
            previous_focus: HWND::default(),
            laying_out: false,
            hot_button: HWND::default(),
            sizing: false,
            changing_state: false,
            opacity: 0,
            fade_from: 0,
            fade_started: std::time::Instant::now(),
            pressed_button: HWND::default(),
            drag_start: None,
            motion_paused: None,
            surface: Some(surface),
            geometry: None,
            render_failed: false,
        }));
        let mut installed =
            SetWindowSubclass(owner, Some(owner_proc), ID, pointer as usize).as_bool();
        installed &= SetWindowSubclass(strip, Some(strip_proc), ID, pointer as usize).as_bool();
        installed &= SetWindowSubclass(edge, Some(edge_proc), ID, pointer as usize).as_bool();
        for button in buttons {
            installed &=
                SetWindowSubclass(button, Some(button_proc), ID, pointer as usize).as_bool();
        }
        if !installed {
            let _ = RemoveWindowSubclass(owner, Some(owner_proc), ID);
            let _ = DestroyWindow(edge);
            let _ = DestroyWindow(strip);
            drop(Box::from_raw(pointer));
            return false;
        }
        layout(pointer);
        true
    }
}
/// Hides the caption and its reveal edge while page content fills the
/// screen. Runs on the window's UI thread.
pub fn set_suppressed(window: &WebviewWindow, suppressed: bool) {
    let Ok(owner) = window.hwnd() else {
        return;
    };
    let mut data = 0usize;
    // SAFETY: UI-thread lookup of this module's own subclass; its data is the
    // live State pointer until WM_NCDESTROY removes the subclass.
    unsafe {
        if GetWindowSubclass(
            HWND(owner.0),
            Some(owner_proc),
            ID,
            Some(&mut data as *mut usize),
        )
        .as_bool()
            && data != 0
        {
            let state = data as *mut State;
            (*state).suppressed = suppressed;
            layout(state);
        }
    }
}

/// The shell's admitted chrome boundary, in logical coordinates. Internal
/// pages need only the corner controls; web pages get a band over the page.
pub fn set_page_left(window: &WebviewWindow, page_left: Option<f64>) {
    let Ok(owner) = window.hwnd() else {
        return;
    };
    unsafe {
        let mut data = 0usize;
        if GetWindowSubclass(HWND(owner.0), Some(owner_proc), ID, Some(&mut data)).as_bool()
            && data != 0
        {
            let state = data as *mut State;
            if (*state).page_left != page_left {
                (*state).page_left = page_left;
                layout(state);
            }
        }
    }
}
unsafe fn visible(hwnd: HWND, show: bool) {
    if (GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_VISIBLE.0 != 0) != show {
        let _ = ShowWindow(hwnd, if show { SW_SHOWNA } else { SW_HIDE });
    }
}
unsafe fn render(p: *mut State) -> bool {
    if (*p).opacity == 0 {
        return true;
    }
    let dpi = GetDpiForWindow((*p).owner).max(96);
    let (width, height, _, _) = paint::dimensions(dpi);
    let mut rect = RECT::default();
    let _ = GetClientRect((*p).owner, &mut rect);
    let mut origin = POINT::default();
    let _ = ClientToScreen((*p).owner, &mut origin);
    let shown = (height as f64 * f64::from((*p).opacity) / 255.0).round() as i32;
    let foreground = GetForegroundWindow();
    let index = |hwnd| (*p).buttons.iter().position(|button| *button == hwnd);
    let appearance = paint::Appearance {
        dpi,
        dark: !matches!((*p).window.theme(), Ok(tauri::Theme::Light)),
        active: foreground == (*p).owner || foreground == (*p).edge,
        maximized: IsZoomed((*p).owner).as_bool(),
        hot: index((*p).hot_button),
        pressed: index((*p).pressed_button),
        focused: index(GetFocus()),
        contrast: super::menus::high_contrast().then(|| {
            [
                GetSysColor(COLOR_WINDOW),
                GetSysColor(COLOR_WINDOWTEXT),
                GetSysColor(COLOR_HIGHLIGHT),
                GetSysColor(COLOR_HIGHLIGHTTEXT),
            ]
        }),
    };
    let Some(mut surface) = (*p).surface.take() else {
        return false;
    };
    let ok = surface.present(
        (*p).strip,
        POINT {
            x: origin.x + rect.right - width,
            y: origin.y,
        },
        shown,
        (*p).opacity,
        appearance,
    );
    (*p).surface = Some(surface);
    if !ok && !(*p).render_failed {
        (*p).render_failed = true;
        crate::write_diagnostic(format_args!("caption: alpha surface presentation failed"));
    }
    ok
}
unsafe fn layout(p: *mut State) {
    if (*p).laying_out {
        return;
    }
    (*p).laying_out = true;
    let owner = (*p).owner;
    let dpi = GetDpiForWindow(owner).max(96);
    let scale = dpi as f64 / 96.0;
    let (controls_width, height, button_width, padding) = paint::dimensions(dpi);
    let button_height = height - padding * 2;
    let mut rect = RECT::default();
    let _ = GetClientRect(owner, &mut rect);
    let left = (*p)
        .page_left
        .map(|left| (left * scale).round() as i32)
        .unwrap_or(rect.right - controls_width)
        .clamp(0, (rect.right - controls_width).max(0));
    let mut origin = POINT::default();
    let _ = ClientToScreen(owner, &mut origin);
    // Owned popup z-order follows the owner. Hover may reveal an inactive
    // window, but must never raise it over another application's foreground.
    let show = !(*p).changing_state
        && !(*p).suppressed
        && IsWindowVisible(owner).as_bool()
        && IsWindowEnabled(owner).as_bool()
        && !IsIconic(owner).as_bool();
    if !show {
        (*p).expanded = false;
        (*p).opacity = 0;
        (*p).pressed_button = HWND::default();
        (*p).drag_start = None;
        (*p).motion_paused = None;
        let capture = GetCapture();
        if capture == (*p).edge || (*p).buttons.contains(&capture) {
            let _ = ReleaseCapture();
        }
        let _ = KillTimer(Some((*p).strip), FADE);
        let _ = KillTimer(Some((*p).strip), HIDE);
    }
    let progress = f64::from((*p).opacity) / 255.0;
    let displacement = (height as f64 * progress).round() as i32;
    drift_engine::set_window_caption_offset(
        owner.0 as isize,
        if (*p).page_left.is_some() {
            displacement as f64 / scale
        } else {
            0.0
        },
        (*p).page_left.is_some() && (*p).opacity > 0 && (*p).opacity < 255,
    );
    let geometry = (origin.x, origin.y, rect.right, left, displacement, dpi);
    if (*p).geometry != Some(geometry) {
        (*p).geometry = Some(geometry);
        let reveal_height = (8.0 * scale).round() as i32;
        let _ = SetWindowPos(
            (*p).edge,
            None,
            origin.x,
            origin.y,
            rect.right.max(1),
            displacement.max(reveal_height),
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER,
        );
        let region = CreateRectRgn(0, 0, rect.right.max(1), reveal_height);
        let band = CreateRectRgn(left, 0, rect.right.max(1), displacement);
        let _ = CombineRgn(Some(region), Some(region), Some(band), RGN_OR);
        let _ = DeleteObject(band.into());
        if SetWindowRgn((*p).edge, Some(region), false) == 0 {
            let _ = DeleteObject(region.into());
        }
        for (index, hwnd) in (*p).buttons.into_iter().enumerate() {
            let _ = SetWindowPos(
                hwnd,
                None,
                rect.right - controls_width + padding + index as i32 * button_width,
                displacement - height + padding,
                button_width,
                button_height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
    let controls_visible = show && displacement > 0 && render(p);
    visible((*p).strip, controls_visible);
    for button in (*p).buttons {
        visible(button, controls_visible);
    }
    visible((*p).edge, show);
    (*p).laying_out = false;
}
// The top target forwards native resize gestures: the child WebView otherwise
// consumes this frameless client border before the owner sees it.
unsafe fn resize_border(owner: HWND) -> i32 {
    if IsZoomed(owner).as_bool()
        || GetWindowLongPtrW(owner, GWL_STYLE) as u32 & WS_THICKFRAME.0 == 0
    {
        return 0;
    }
    let dpi = GetDpiForWindow(owner).max(96);
    // Only the visible outer rim belongs to resize hit testing. The full
    // padded sizing frame extends into the caption and the page gutter.
    (2 * GetSystemMetricsForDpi(SM_CYBORDER, dpi)).max(1)
}
fn top_frame_hit(x: i32, y: i32, width: i32, border: i32, corner: i32) -> Option<u32> {
    if border <= 0 || x < 0 || x >= width || y < 0 || y >= border {
        return None;
    }
    Some(if x < corner {
        HTTOPLEFT
    } else if x >= width - corner {
        HTTOPRIGHT
    } else {
        HTTOP
    })
}
fn resize_direction(hit: u32) -> Option<u32> {
    match hit {
        HTTOP => Some(WMSZ_TOP),
        HTTOPLEFT => Some(WMSZ_TOPLEFT),
        HTTOPRIGHT => Some(WMSZ_TOPRIGHT),
        HTRIGHT => Some(WMSZ_RIGHT),
        _ => None,
    }
}
unsafe fn resize_hit(p: *mut State, l: LPARAM) -> Option<u32> {
    let mut point = POINT {
        x: l.0 as i16 as i32,
        y: (l.0 >> 16) as i16 as i32,
    };
    let _ = ScreenToClient((*p).owner, &mut point);
    let mut rect = RECT::default();
    let _ = GetClientRect((*p).owner, &mut rect);
    let border = resize_border((*p).owner);
    let dpi = GetDpiForWindow((*p).owner).max(96);
    let corner = 2
        * (GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi)
            + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi));
    top_frame_hit(point.x, point.y, rect.right, border, corner).or_else(|| {
        // Preserve the right resize gutter even while the caption is revealed.
        (border > 0
            && point.x >= rect.right - border
            && point.x < rect.right
            && point.y >= border
            && point.y < rect.bottom - border)
            .then_some(HTRIGHT)
    })
}
unsafe fn prepare_state_change(p: *mut State) {
    // Remove both owned popups before focus repaint and the system animation.
    (*p).changing_state = true;
    let _ = ShowWindow((*p).strip, SW_HIDE);
    let _ = ShowWindow((*p).edge, SW_HIDE);
    (*p).expanded = false;
    (*p).opacity = 0;
    (*p).hot_button = HWND::default();
    (*p).fade_from = 0;
    (*p).pressed_button = HWND::default();
    (*p).drag_start = None;
    (*p).motion_paused = None;
    drift_engine::set_window_caption_offset((*p).owner.0 as isize, 0.0, false);
    let _ = KillTimer(Some((*p).strip), FADE);
    let _ = KillTimer(Some((*p).strip), HIDE);
}
unsafe fn transition(p: *mut State, expanded: bool) {
    if (*p).expanded == expanded {
        return;
    }
    (*p).expanded = expanded;
    let mut enabled = windows::core::BOOL(1);
    let _ = SystemParametersInfoW(
        SPI_GETCLIENTAREAANIMATION,
        0,
        Some((&mut enabled as *mut windows::core::BOOL).cast()),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    );
    if !enabled.as_bool() || super::menus::high_contrast() {
        (*p).opacity = if expanded { 255 } else { 0 };
        let _ = KillTimer(Some((*p).strip), FADE);
    } else {
        (*p).fade_from = (*p).opacity;
        (*p).fade_started = std::time::Instant::now();
        SetTimer(Some((*p).strip), FADE, 15, None);
    }
    layout(p);
}
unsafe fn reveal(p: *mut State) {
    if (*p).suppressed || (*p).changing_state || (*p).sizing {
        return;
    }
    let _ = KillTimer(Some((*p).strip), HIDE);
    transition(p, true);
}
unsafe fn button_at_cursor(p: *mut State) -> HWND {
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    button_at_point(p, cursor)
}
fn button_hit(rects: &[RECT; 3], point: POINT) -> Option<usize> {
    rects.iter().position(|rect| {
        point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
    })
}
unsafe fn button_at_point(p: *mut State, point: POINT) -> HWND {
    if (*p).opacity == 0 || (*p).suppressed {
        return HWND::default();
    }
    let mut visible_bounds = RECT::default();
    let _ = GetWindowRect((*p).edge, &mut visible_bounds);
    if !PtInRect(&visible_bounds, point).as_bool() {
        // During reveal, part of each child lies above the clipped input host.
        // Releasing there must not activate an invisible part of the button.
        return HWND::default();
    }
    let mut rects = [RECT::default(); 3];
    for (button, rect) in (*p).buttons.into_iter().zip(&mut rects) {
        let _ = GetWindowRect(button, rect);
    }
    button_hit(&rects, point).map_or(HWND::default(), |index| (*p).buttons[index])
}
unsafe fn update_hot(p: *mut State, hot: HWND) {
    if (*p).hot_button != hot {
        (*p).hot_button = hot;
        let _ = render(p);
    }
}
unsafe fn pause_motion(p: *mut State) {
    (*p).motion_paused
        .get_or_insert_with(std::time::Instant::now);
    let _ = KillTimer(Some((*p).strip), FADE);
}
unsafe fn resume_motion(p: *mut State) {
    if let Some(paused) = (*p).motion_paused.take() {
        (*p).fade_started += paused.elapsed();
        if (*p).opacity != if (*p).expanded { 255 } else { 0 } {
            SetTimer(Some((*p).strip), FADE, 15, None);
        }
    }
}
// The same press/capture/release path covers the entire button, regardless of
// whether Windows delivered a client or nonclient message (Snap's HTMAXBUTTON).
unsafe fn pointer_input(
    p: *mut State,
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> Option<LRESULT> {
    match msg {
        WM_LBUTTONDOWN | WM_NCLBUTTONDOWN => {
            let button = button_at_cursor(p);
            if !button.0.is_null() && resize_hit_from_cursor(p).is_none() {
                let _ = SetForegroundWindow((*p).owner);
                (*p).pressed_button = button;
                pause_motion(p);
                SetCapture(hwnd);
                let _ = render(p);
                return Some(LRESULT(0));
            }
        }
        WM_LBUTTONUP | WM_NCLBUTTONUP if !(*p).pressed_button.0.is_null() => {
            let pressed = (*p).pressed_button;
            let activate = button_at_cursor(p) == pressed;
            (*p).pressed_button = HWND::default();
            resume_motion(p);
            let _ = ReleaseCapture();
            let _ = render(p);
            SetTimer(Some((*p).strip), HIDE, 450, None);
            if activate {
                let index = (*p)
                    .buttons
                    .iter()
                    .position(|button| *button == pressed)
                    .unwrap();
                let _ = SendMessageW(
                    (*p).strip,
                    WM_COMMAND,
                    Some(WPARAM(index + 1)),
                    Some(LPARAM(pressed.0 as isize)),
                );
            }
            return Some(LRESULT(0));
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            let pressed = (*p).pressed_button;
            (*p).pressed_button = HWND::default();
            (*p).drag_start = None;
            resume_motion(p);
            if !pressed.0.is_null() {
                let _ = render(p);
            }
            SetTimer(Some((*p).strip), HIDE, 450, None);
        }
        WM_MOUSEMOVE | WM_NCMOUSEMOVE => {
            if let Some(start) = (*p).drag_start {
                let mut point = POINT::default();
                let _ = GetCursorPos(&mut point);
                if (point.x - start.x).abs() >= GetSystemMetrics(SM_CXDRAG)
                    || (point.y - start.y).abs() >= GetSystemMetrics(SM_CYDRAG)
                {
                    (*p).drag_start = None;
                    let _ = ReleaseCapture();
                    let _ = PostMessageW(
                        Some((*p).owner),
                        WM_SYSCOMMAND,
                        WPARAM((SC_MOVE | HTCAPTION) as usize),
                        LPARAM(0),
                    );
                    return Some(LRESULT(0));
                }
            }
            reveal(p);
            track(hwnd, msg == WM_NCMOUSEMOVE);
            let hot = button_at_cursor(p);
            update_hot(p, hot);
            if hot == (*p).buttons[1] && msg == WM_NCMOUSEMOVE {
                // Let the owner offer Snap layouts; clicks stay on our explicit
                // capture path instead of DefWindowProc tracking another HWND.
                let _ = SendMessageW((*p).owner, msg, Some(w), Some(l));
            }
            return Some(LRESULT(0));
        }
        WM_MOUSELEAVE | WM_NCMOUSELEAVE => {
            update_hot(p, HWND::default());
            SetTimer(Some((*p).strip), HIDE, 450, None);
        }
        _ => {}
    }
    None
}
unsafe fn resize_hit_from_cursor(p: *mut State) -> Option<u32> {
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    resize_hit(
        p,
        LPARAM(((cursor.y as u16 as u32) << 16 | cursor.x as u16 as u32) as isize),
    )
}
unsafe extern "system" fn edge_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let p = data as *mut State;
    if let Some(result) = pointer_input(p, hwnd, msg, w, l) {
        return result;
    }
    match msg {
        WM_NCHITTEST => {
            if let Some(hit) = resize_hit(p, l) {
                return LRESULT(hit as isize);
            }
            if button_at_point(
                p,
                POINT {
                    x: l.0 as i16 as i32,
                    y: (l.0 >> 16) as i16 as i32,
                },
            ) == (*p).buttons[1]
            {
                return LRESULT(HTMAXBUTTON as isize);
            }
        }
        WM_NCLBUTTONDOWN if resize_direction(w.0 as u32).is_some() => {
            let _ = ReleaseCapture();
            let _ = PostMessageW(
                Some((*p).owner),
                WM_SYSCOMMAND,
                WPARAM((SC_SIZE | resize_direction(w.0 as u32).unwrap()) as usize),
                l,
            );
            return LRESULT(0);
        }
        WM_SETCURSOR if l.0 as u16 == HTCLIENT as u16 => {
            if let Ok(cursor) = LoadCursorW(None, IDC_ARROW) {
                SetCursor(Some(cursor));
            }
            return LRESULT(1);
        }
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_COMMAND => return SendMessageW((*p).strip, msg, Some(w), Some(l)),
        WM_DRAWITEM => return LRESULT(1),
        // Keep the edge useful as a native drag handle.
        WM_LBUTTONDOWN => {
            // Entering the move loop on the first click would hide the band
            // before a second click could maximize it.
            let mut point = POINT::default();
            let _ = GetCursorPos(&mut point);
            let _ = SetForegroundWindow((*p).owner);
            (*p).drag_start = Some(point);
            pause_motion(p);
            SetCapture(hwnd);
            return LRESULT(0);
        }
        WM_LBUTTONUP if (*p).drag_start.take().is_some() => {
            let _ = ReleaseCapture();
            return LRESULT(0);
        }
        WM_LBUTTONDBLCLK => {
            let command = if IsZoomed((*p).owner).as_bool() {
                SC_RESTORE
            } else {
                SC_MAXIMIZE
            };
            let _ = PostMessageW(
                Some((*p).owner),
                WM_SYSCOMMAND,
                WPARAM(command as usize),
                LPARAM(0),
            );
            return LRESULT(0);
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            FillRect(dc, &paint.rcPaint, HBRUSH(GetStockObject(BLACK_BRUSH).0));
            let _ = EndPaint(hwnd, &paint);
            return LRESULT(0);
        }
        WM_NCDESTROY => {
            let _ = RemoveWindowSubclass(hwnd, Some(edge_proc), ID);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, w, l)
}
unsafe fn track(hwnd: HWND, nonclient: bool) {
    let mut tracking = TRACKMOUSEEVENT {
        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE
            | if nonclient {
                TME_NONCLIENT
            } else {
                TRACKMOUSEEVENT_FLAGS(0)
            },
        hwndTrack: hwnd,
        ..Default::default()
    };
    let _ = TrackMouseEvent(&mut tracking);
}
unsafe extern "system" fn owner_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let p = data as *mut State;
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(owner_proc), ID);
        drift_engine::forget_window_caption_offset(hwnd.0 as isize);
        // Owned popups are explicitly torn down before freeing subclass state.
        if IsWindow(Some((*p).strip)).as_bool() {
            let _ = DestroyWindow((*p).strip);
        }
        if IsWindow(Some((*p).edge)).as_bool() {
            let _ = DestroyWindow((*p).edge);
        }
        drop(Box::from_raw(p));
        return DefSubclassProc(hwnd, msg, w, l);
    }
    if msg == WM_CLOSE {
        prepare_state_change(p);
        // A vetoed close must leave an operable window. This timer belongs to
        // the owner and is discarded automatically if close destroys it.
        SetTimer(Some(hwnd), CLOSE_SETTLED, 100, None);
        return DefSubclassProc(hwnd, msg, w, l);
    }
    if msg == WM_SYSCOMMAND && w.0 as u32 & 0xfff0 == SC_CLOSE {
        // DefWindowProc can destroy the subclass state synchronously.
        return DefSubclassProc(hwnd, msg, w, l);
    }
    if msg == WM_TIMER && w.0 == CLOSE_SETTLED {
        let _ = KillTimer(Some(hwnd), CLOSE_SETTLED);
        (*p).changing_state = false;
        layout(p);
        return LRESULT(0);
    }
    if !(*p).suppressed
        && !(*p).changing_state
        && !(*p).sizing
        && msg == WM_SYSKEYDOWN
        && w.0 == VK_F10.0 as usize
    {
        reveal(p);
        // Keyboard focus requires a fully visible native target immediately.
        (*p).opacity = 255;
        let _ = KillTimer(Some((*p).strip), FADE);
        layout(p);
        let _ = SetFocus(Some((*p).buttons[0]));
        return LRESULT(0);
    }
    let state_command = msg == WM_SYSCOMMAND
        && matches!(w.0 as u32 & 0xfff0, SC_MINIMIZE | SC_MAXIMIZE | SC_RESTORE);
    if state_command {
        prepare_state_change(p);
    }
    if msg == WM_ENTERSIZEMOVE {
        (*p).sizing = true;
        pause_motion(p);
        layout(p);
    } else if msg == WM_EXITSIZEMOVE {
        (*p).sizing = false;
        resume_motion(p);
        SetTimer(Some((*p).strip), HIDE, 450, None);
        layout(p);
    }
    if msg == WM_NCHITTEST {
        if let Some(hit) = resize_hit(p, l) {
            return LRESULT(hit as isize);
        }
    }
    if msg == WM_NCHITTEST && (*p).expanded {
        let point = POINT {
            x: l.0 as i16 as i32,
            y: (l.0 >> 16) as i16 as i32,
        };
        let mut rect = RECT::default();
        let _ = GetWindowRect((*p).buttons[1], &mut rect);
        if PtInRect(&rect, point).as_bool() {
            return LRESULT(HTMAXBUTTON as isize);
        }
    }
    let result = DefSubclassProc(hwnd, msg, w, l);
    // Keep the owned caption hidden through all synchronous size/focus messages.
    // WM_SIZE alone is too early: Windows may still be animating the owner.
    if state_command {
        (*p).changing_state = false;
        layout(p);
    }
    if matches!(
        msg,
        WM_SIZE
            | WM_DPICHANGED
            | WM_WINDOWPOSCHANGED
            | WM_PARENTNOTIFY
            | WM_ACTIVATE
            | WM_SHOWWINDOW
    ) {
        layout(p);
    }
    if matches!(msg, WM_THEMECHANGED | WM_SETTINGCHANGE) {
        let _ = render(p);
    }
    result
}
unsafe extern "system" fn strip_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let p = data as *mut State;
    match msg {
        WM_NCHITTEST => {
            if let Some(hit) = resize_hit(p, l) {
                return LRESULT(hit as isize);
            }
            return LRESULT(HTCAPTION as isize);
        }
        WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK if w.0 == HTCAPTION as usize => {
            return SendMessageW((*p).owner, msg, Some(w), Some(l));
        }
        WM_NCLBUTTONDOWN if resize_direction(w.0 as u32).is_some() => {
            let _ = ReleaseCapture();
            let _ = PostMessageW(
                Some((*p).owner),
                WM_SYSCOMMAND,
                WPARAM((SC_SIZE | resize_direction(w.0 as u32).unwrap()) as usize),
                l,
            );
            return LRESULT(0);
        }
        WM_SETCURSOR if l.0 as u16 == HTCLIENT as u16 => {
            if let Ok(cursor) = LoadCursorW(None, IDC_ARROW) {
                SetCursor(Some(cursor));
            }
            return LRESULT(1);
        }
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_CLOSE => {
            let _ = PostMessageW(
                Some((*p).owner),
                WM_SYSCOMMAND,
                WPARAM(SC_CLOSE as usize),
                LPARAM(0),
            );
            return LRESULT(0);
        }
        WM_MOUSEMOVE => {
            reveal(p);
            track(hwnd, false);
        }
        WM_NCMOUSEMOVE => {
            reveal(p);
            track(hwnd, true);
        }
        WM_MOUSELEAVE | WM_NCMOUSELEAVE => {
            SetTimer(Some(hwnd), HIDE, 450, None);
        }
        WM_TIMER if w.0 == FADE => {
            // A press owns a stable target even when it begins mid-reveal.
            if (*p).motion_paused.is_some() {
                return LRESULT(0);
            }
            let t = ((*p).fade_started.elapsed().as_secs_f64() / 0.22).min(1.0);
            let eased = 1.0 - (1.0 - t).powi(3);
            let target = if (*p).expanded { 255.0 } else { 0.0 };
            (*p).opacity = (f64::from((*p).fade_from)
                + (target - f64::from((*p).fade_from)) * eased)
                .round() as u8;
            layout(p);
            if t >= 1.0 {
                let _ = KillTimer(Some(hwnd), FADE);
                if !(*p).expanded {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    layout(p);
                }
            }
            return LRESULT(0);
        }
        WM_TIMER if w.0 == HIDE => {
            let _ = KillTimer(Some(hwnd), HIDE);
            // Release, focus loss and the end of the move loop reschedule the
            // dismissal. No polling timer runs while a gesture owns the band.
            if (*p).sizing
                || !(*p).pressed_button.0.is_null()
                || (*p).drag_start.is_some()
                || (*p).buttons.contains(&GetFocus())
            {
                return LRESULT(0);
            }
            let mut cursor = POINT::default();
            let _ = GetCursorPos(&mut cursor);
            let mut rect = RECT::default();
            let _ = GetWindowRect((*p).edge, &mut rect);
            rect.left += (*p).geometry.map_or(0, |(_, _, _, left, _, _)| left);
            let mut edge = RECT::default();
            let _ = GetWindowRect((*p).edge, &mut edge);
            edge.bottom =
                edge.top + (8.0 * GetDpiForWindow((*p).owner).max(96) as f64 / 96.0).round() as i32;
            edge.top += resize_border((*p).owner);
            // Moving along the reveal strip must not dismiss the controls.
            let hovered = WindowFromPoint(cursor);
            let on_caption = hovered == (*p).edge || (*p).buttons.contains(&hovered);
            if !on_caption
                || (!PtInRect(&rect, cursor).as_bool() && !PtInRect(&edge, cursor).as_bool())
            {
                transition(p, false);
            } else {
                // The input host stays visible, so a leave event can dismiss
                // the band without a polling timer during a stationary hover.
                track(hovered, button_at_cursor(p) == (*p).buttons[1]);
            }
        }
        WM_COMMAND if (w.0 >> 16) == BN_CLICKED as usize => {
            let target = if IsWindow(Some((*p).previous_focus)).as_bool() {
                (*p).previous_focus
            } else {
                (*p).owner
            };
            let command = match w.0 & 0xffff {
                1 => SC_MINIMIZE,
                2 => {
                    if IsZoomed((*p).owner).as_bool() {
                        SC_RESTORE
                    } else {
                        SC_MAXIMIZE
                    }
                }
                3 => SC_CLOSE,
                _ => return DefSubclassProc(hwnd, msg, w, l),
            };
            // Retire visible pixels before focus/default-button processing,
            // including Close, which used to repaint all three child buttons.
            prepare_state_change(p);
            let _ = SetFocus(Some(target));
            let _ = PostMessageW(
                Some((*p).owner),
                WM_SYSCOMMAND,
                WPARAM(command as usize),
                LPARAM(0),
            );
            return LRESULT(0);
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut paint);
            let _ = EndPaint(hwnd, &paint);
            return LRESULT(1);
        }
        WM_NCDESTROY => {
            let _ = KillTimer(Some(hwnd), HIDE);
            let _ = KillTimer(Some(hwnd), FADE);
            let _ = RemoveWindowSubclass(hwnd, Some(strip_proc), ID);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, w, l)
}
unsafe extern "system" fn button_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let p = data as *mut State;
    if let Some(result) = pointer_input(p, hwnd, msg, w, l) {
        return result;
    }
    match msg {
        BM_CLICK => {
            // Accessibility invocation has no cursor location. Route the
            // requested native button directly instead of simulating a hit.
            if let Some(index) = (*p).buttons.iter().position(|button| *button == hwnd) {
                return SendMessageW(
                    (*p).strip,
                    WM_COMMAND,
                    Some(WPARAM(index + 1)),
                    Some(LPARAM(hwnd.0 as isize)),
                );
            }
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            // These controls supply semantics/input only. The alpha surface
            // owns every visible pixel, including focus and pressed feedback.
            let mut paint = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut paint);
            let _ = EndPaint(hwnd, &paint);
            return LRESULT(0);
        }
        WM_NCHITTEST => {
            if let Some(hit) = resize_hit(p, l) {
                return LRESULT(hit as isize);
            }
            if hwnd == (*p).buttons[1] {
                return LRESULT(HTMAXBUTTON as isize);
            }
        }
        WM_NCLBUTTONDOWN if resize_direction(w.0 as u32).is_some() => {
            let _ = ReleaseCapture();
            let _ = PostMessageW(
                Some((*p).owner),
                WM_SYSCOMMAND,
                WPARAM((SC_SIZE | resize_direction(w.0 as u32).unwrap()) as usize),
                l,
            );
            return LRESULT(0);
        }
        WM_SETCURSOR if l.0 as u16 == HTCLIENT as u16 => {
            if let Ok(cursor) = LoadCursorW(None, IDC_ARROW) {
                SetCursor(Some(cursor));
            }
            return LRESULT(1);
        }
        WM_SETFOCUS => {
            let previous = HWND(w.0 as *mut _);
            if !(*p).buttons.contains(&previous) {
                (*p).previous_focus = previous;
            }
            let _ = render(p);
        }
        WM_KILLFOCUS => {
            if (*p).pressed_button == hwnd {
                (*p).pressed_button = HWND::default();
                resume_motion(p);
            }
            if (*p).hot_button == hwnd {
                (*p).hot_button = HWND::default();
            }
            SetTimer(Some((*p).strip), HIDE, 450, None);
            let _ = render(p);
        }
        WM_KEYDOWN if w.0 == VK_SPACE.0 as usize => {
            (*p).pressed_button = hwnd;
            (*p).hot_button = hwnd;
            let _ = render(p);
        }
        WM_KEYUP if w.0 == VK_SPACE.0 as usize => {
            (*p).pressed_button = HWND::default();
            let _ = render(p);
        }
        WM_KEYDOWN if w.0 == VK_ESCAPE.0 as usize => {
            let target = if IsWindow(Some((*p).previous_focus)).as_bool() {
                (*p).previous_focus
            } else {
                (*p).owner
            };
            let _ = SetFocus(Some(target));
            transition(p, false);
            return LRESULT(0);
        }
        WM_KEYDOWN if w.0 == VK_LEFT.0 as usize || w.0 == VK_RIGHT.0 as usize => {
            let index = (*p).buttons.iter().position(|b| *b == hwnd).unwrap_or(0);
            let next = (index + if w.0 == VK_LEFT.0 as usize { 2 } else { 1 }) % 3;
            let _ = SetFocus(Some((*p).buttons[next]));
            return LRESULT(0);
        }
        WM_NCDESTROY => {
            let _ = RemoveWindowSubclass(hwnd, Some(button_proc), ID);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, w, l)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_button_rectangles_hit_at_fractional_dpi_and_negative_monitor_coordinates() {
        for dpi in [96, 120, 144, 192] {
            let (_, outer_height, width, padding) = paint::dimensions(dpi);
            let height = outer_height - padding * 2;
            let rects = std::array::from_fn(|index| RECT {
                left: -800 + index as i32 * width,
                right: -800 + (index as i32 + 1) * width,
                top: -100,
                bottom: -100 + height,
            });
            for (index, rect) in rects.iter().enumerate() {
                for x in rect.left..rect.right {
                    for y in rect.top..rect.bottom {
                        assert_eq!(button_hit(&rects, POINT { x, y }), Some(index));
                    }
                }
            }
            assert_eq!(
                button_hit(
                    &rects,
                    POINT {
                        x: rects[2].right,
                        y: -90
                    }
                ),
                None
            );
            assert_eq!(
                button_hit(
                    &rects,
                    POINT {
                        x: -790,
                        y: rects[0].bottom
                    }
                ),
                None
            );
        }
    }
    #[test]
    fn top_resize_band_precedes_reveal_and_preserves_corners() {
        for border in [2, 3, 4] {
            assert_eq!(top_frame_hit(200, 0, 800, border, 16), Some(HTTOP));
            assert_eq!(top_frame_hit(200, border - 1, 800, border, 16), Some(HTTOP));
            assert_eq!(top_frame_hit(200, border, 800, border, 16), None);
            assert_eq!(top_frame_hit(1, 1, 800, border, 16), Some(HTTOPLEFT));
            assert_eq!(top_frame_hit(799, 1, 800, border, 16), Some(HTTOPRIGHT));
        }
        // The top of a caption button (3 DIP inset) is never a resize target.
        assert_eq!(top_frame_hit(760, 3, 800, 2, 16), None);
        assert_eq!(top_frame_hit(790, 1, 800, 2, 16), Some(HTTOPRIGHT));
        assert_eq!(top_frame_hit(200, 0, 800, 0, 16), None);
        assert_eq!(top_frame_hit(-1, 0, 800, 8, 16), None);
        assert_eq!(top_frame_hit(800, 0, 800, 8, 16), None);
    }
}

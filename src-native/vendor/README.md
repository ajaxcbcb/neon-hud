# Windows pointer and hidden-window repairs

`winit/` is the complete published winit 0.30.13 crate, downloaded from
https://static.crates.io/crates/winit/winit-0.30.13.crate. Its original SHA-256 is
`a6755fa58a9f8350bd1e472d4c3fcc25f824ec358933bba33306d0b63df5978d`,
matching the registry checksum previously recorded in the native Cargo.lock.
All upstream files and the Apache 2.0 license are retained.

Windows `event_loop.rs` is modified so WM_MOUSEMOVE emits
CursorMoved when re-entering a window, even if its client coordinate matches
the cached position from before CursorLeft. The egui input adapter clears its
position on CursorLeft and ignores button events until a new position arrives.
Mouse movement inside the window retains the original duplicate suppression.
Windows redraw delivery is also modified in `window.rs`, `window_state.rs` and
the WM_PAINT handler in `event_loop.rs`. Explicit redraw requests for a hidden
window asynchronously post WM_PAINT, with at most one pending notification.
Visible windows retain upstream RedrawWindow behavior. The handler clears the
pending notification and rearms through the same helper, preserving worker,
tray and adaptive timer callbacks without showing the HUD or adding a thread.
Mac and other platform implementations are unchanged.

The native manifest patches crates.io to this local source. The locked builds
and real Windows right-click regression keep the original cursor targets,
including the same-coordinate left-to-right re-entry, all eight edges/corners,
menu actions and Escape dismissal. Hidden lifecycle checks require bounded
callbacks and normal auto-exit through the existing save/stop path after Hide.
No synthetic cursor or shutdown workaround is used.

Remove this patch when an upstream release provides equivalent behavior, after
the same native interaction regression passes.

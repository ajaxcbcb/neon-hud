# Windows pointer re-entry repair

`winit/` is the complete published winit 0.30.13 crate, downloaded from
https://static.crates.io/crates/winit/winit-0.30.13.crate. Its original SHA-256 is
`a6755fa58a9f8350bd1e472d4c3fcc25f824ec358933bba33306d0b63df5978d`,
matching the registry checksum previously recorded in the native Cargo.lock.
All upstream files and the Apache 2.0 license are retained.

The only source modification is in Windows `event_loop.rs`: WM_MOUSEMOVE emits
CursorMoved when re-entering a window, even if its client coordinate matches
the cached position from before CursorLeft. The egui input adapter clears its
position on CursorLeft and ignores button events until a new position arrives.
Mouse movement inside the window retains the original duplicate suppression.
Mac and other platform implementations are unchanged.

The native manifest patches crates.io to this local source. The locked builds
and real Windows right-click regression keep the original cursor targets,
including the same-coordinate left-to-right re-entry, all eight edges/corners,
menu actions and Escape dismissal. No synthetic cursor workaround is used.

Remove this patch when an upstream release provides equivalent behavior, after
the same native interaction regression passes.

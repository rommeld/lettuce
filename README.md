# lettuce

**Building a GPU-accelerated terminal emulator in Rust using [GPUI](https://www.gpui.rs) (Zed's rendering framework).** The architecture follows [Ghostty's](https://github.com/ghostty-org) core design patterns—VT parser state machine, efficient cell buffers, and PTY abstraction—while leveraging GPUI's unified rendering layer to avoid the cross-platform UI complexity of native toolkits.

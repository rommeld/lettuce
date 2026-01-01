use anyhow::Result;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let pty_system = native_pty_system();

    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let cmd = CommandBuilder::new("/bin/bash");
    let _child = pair.slave.spawn_command(cmd)?;

    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;
    writeln!(writer, "echo hello")?;
    writer.flush()?;

    thread::sleep(Duration::from_millis(200));

    let mut buffer = [0u8; 4096];
    let n = reader.read(&mut buffer)?;
    print!("{}", String::from_utf8_lossy(&buffer[..n]));

    Ok(())
}

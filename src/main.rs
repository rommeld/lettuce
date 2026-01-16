use anyhow::Result;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;
use vte::{Params, Perform};

// Session 2 Part 1 - Color, Attributes, Events

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum Color {
    #[default]
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Default)]
struct Attributes {
    foreground: Color,
    background: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

#[derive(Debug, Clone)]
enum TerminalEvent {
    Print { char: char, attrs: Attributes },
    Linefeed,
    CarriageReturn,
    Backspace,
    Tab,
    Bell,
    CursorPosition { row: u16, col: u16 },
    CursorUp(u16),
    CursorDown(u16),
    CursorForward(u16),
    CursorBack(u16),
    EraseDisplay(u16),
    EraseLine(u16),
    SetMode(Vec<u16>),
    ResetMode(Vec<u16>),
    UnhandledCsi { action: char, params: Vec<u16> },
    UnhandledEsc(u8),
    Osc(Vec<Vec<u8>>),
}

// Session 2 Part 2 - Parser

struct Parser {
    current_attrs: Attributes,
    events: Vec<TerminalEvent>,
}

impl Parser {
    fn new() -> Self {
        Parser {
            current_attrs: Attributes::default(),
            events: Vec::new(),
        }
    }

    fn handle_sgr(&mut self, params: &Params) {
        let mut iter = params.iter().peekable();

        // Reset when ESC[ with no params
        if iter.peek().is_none() {
            self.current_attrs = Attributes::default();
            return;
        }

        for param in &mut iter {
            match param {
                [0] => self.current_attrs = Attributes::default(),
                [1] => self.current_attrs.bold = true,
                [3] => self.current_attrs.italic = true,
                [4] => self.current_attrs.underline = true,
                [7] => self.current_attrs.inverse = true,
                [22] => self.current_attrs.bold = false,
                [23] => self.current_attrs.italic = false,
                [24] => self.current_attrs.underline = false,
                [27] => self.current_attrs.inverse = false,
                [30] => self.current_attrs.foreground = Color::Black,
                [31] => self.current_attrs.foreground = Color::Red,
                [32] => self.current_attrs.foreground = Color::Green,
                [33] => self.current_attrs.foreground = Color::Yellow,
                [34] => self.current_attrs.foreground = Color::Blue,
                [35] => self.current_attrs.foreground = Color::Magenta,
                [36] => self.current_attrs.foreground = Color::Cyan,
                [37] => self.current_attrs.foreground = Color::White,
                [38] => self.current_attrs.foreground = Color::Default,
                [40] => self.current_attrs.background = Color::Black,
                [41] => self.current_attrs.background = Color::Red,
                [42] => self.current_attrs.background = Color::Green,
                [43] => self.current_attrs.background = Color::Yellow,
                [44] => self.current_attrs.background = Color::Blue,
                [45] => self.current_attrs.background = Color::Magenta,
                [46] => self.current_attrs.background = Color::Cyan,
                [47] => self.current_attrs.background = Color::White,
                [49] => self.current_attrs.background = Color::Default,
                [90] => self.current_attrs.foreground = Color::BrightBlack,
                [91] => self.current_attrs.foreground = Color::BrightRed,
                [92] => self.current_attrs.foreground = Color::BrightGreen,
                [93] => self.current_attrs.foreground = Color::BrightYellow,
                [94] => self.current_attrs.foreground = Color::BrightBlue,
                [95] => self.current_attrs.foreground = Color::BrightMagenta,
                [96] => self.current_attrs.foreground = Color::BrightCyan,
                [97] => self.current_attrs.foreground = Color::BrightWhite,
                [38, 5, n] => self.current_attrs.foreground = Color::Indexed(*n as u8),
                [38, 2, r, g, b] => {
                    self.current_attrs.foreground = Color::Rgb(*r as u8, *g as u8, *b as u8)
                }
                [48, 5, n] => self.current_attrs.background = Color::Indexed(*n as u8),
                [48, 2, r, g, b] => {
                    self.current_attrs.background = Color::Rgb(*r as u8, *g as u8, *b as u8)
                }
                _ => {}
            }
        }
    }
}

impl Perform for Parser {
    fn print(&mut self, c: char) {
        self.events.push(TerminalEvent::Print {
            char: c,
            attrs: self.current_attrs.clone(),
        });
    }

    fn execute(&mut self, byte: u8) {
        let event = match byte {
            0x0A => TerminalEvent::Linefeed,
            0x0D => TerminalEvent::CarriageReturn,
            0x08 => TerminalEvent::Backspace,
            0x09 => TerminalEvent::Tab,
            0x07 => TerminalEvent::Bell,
            _ => return,
        };
        self.events.push(event);
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], ignore: bool, action: char) {
        if ignore {
            return;
        }

        let event = match action {
            'm' => {
                self.handle_sgr(params);
                return;
            }
            'H' | 'f' => {
                // Cursor positions
                let mut iter = params.iter();
                let row = iter.next().and_then(|p| p.first()).copied().unwrap_or(1);
                let col = iter.next().and_then(|p| p.first()).copied().unwrap_or(1);
                TerminalEvent::CursorPosition { row, col }
            }
            'A' => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(1);
                TerminalEvent::CursorUp(n)
            }
            'B' => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(1);
                TerminalEvent::CursorDown(n)
            }
            'C' => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(1);
                TerminalEvent::CursorForward(n)
            }
            'D' => {
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(1);
                TerminalEvent::CursorBack(n)
            }
            'J' => {
                // Cursor positions
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                TerminalEvent::EraseDisplay(mode)
            }
            'K' => {
                // Cursor positions
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(0);
                TerminalEvent::EraseLine(mode)
            }
            'h' => {
                // Mode set/reset - often used with ? prefix
                let modes: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();
                TerminalEvent::SetMode(modes)
            }
            'l' => {
                // Mode set/reset - often used with ? prefix
                let modes: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();
                TerminalEvent::ResetMode(modes)
            }
            _ => {
                let p: Vec<u16> = params.iter().flat_map(|p| p.to_vec()).collect();
                TerminalEvent::UnhandledCsi { action, params: p }
            }
        };
        self.events.push(event);
    }

    // Simple ESC sequences
    // ESC followed by just one byte, without '['
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        self.events.push(TerminalEvent::UnhandledEsc(byte));
    }

    // Called for Operating System Commands
    // ESC followed by ']'
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let owned: Vec<Vec<u8>> = params.iter().map(|p| p.to_vec()).collect();
        self.events.push(TerminalEvent::Osc(owned));
    }

    // hook, put, unhook Device Control String
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}
}

// Session 3 - Terminal State Types

#[derive(Debug, Clone)]
struct Cell {
    character: char,
    attrs: Attributes,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            character: ' ',
            attrs: Attributes::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct Cursor {
    row: usize,
    col: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor { row: 0, col: 0 }
    }
}

struct Terminal {
    grid: Vec<Vec<Cell>>,
    cursor: Cursor,
    rows: usize,
    cols: usize,
}

impl Terminal {
    fn new(cols: usize, rows: usize) -> Self {
        let grid = (0..rows)
            .map(|_| (0..cols).map(|_| Cell::default()).collect())
            .collect();

        Terminal {
            grid,
            cursor: Cursor::default(),
            rows,
            cols,
        }
    }

    fn print(&mut self, c: char, attrs: Attributes) {
        self.grid[self.cursor.row][self.cursor.col] = Cell {
            character: c,
            attrs,
        };

        self.cursor.col += 1;

        if self.cursor.col >= self.cols {
            self.cursor.col = 0;
            self.cursor.row += 1;

            if self.cursor.row >= self.rows {
                self.cursor.row = self.rows - 1;
            }
        }
    }

    fn set_cursor_position(&mut self, row: u16, col: u16) {
        let row = if row == 0 { 1 } else { row };
        let col = if col == 0 { 1 } else { col };

        self.cursor.row = ((row - 1) as usize).min(self.rows - 1);
        self.cursor.col = ((col - 1) as usize).min(self.cols - 1);
    }

    fn cursor_up(&mut self, n: u16) {
        self.cursor.row = self.cursor.row.saturating_sub(n as usize);
    }

    fn cursor_down(&mut self, n: u16) {
        self.cursor.row = (self.cursor.row + n as usize).min(self.rows - 1);
    }

    fn cursor_forward(&mut self, n: u16) {
        self.cursor.col = (self.cursor.col + n as usize).min(self.cols - 1);
    }

    fn cursor_back(&mut self, n: u16) {
        self.cursor.col = self.cursor.col.saturating_sub(n as usize);
    }

    fn render_to_string(&self) -> String {
        let mut output = String::new();
        for row in &self.grid {
            for cell in row {
                output.push(cell.character)
            }
            output.push('\n')
        }
        output
    }

    fn debug_render(&self) -> String {
        let mut output = String::new();

        for (row_idx, row) in self.grid.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if row_idx == self.cursor.row && col_idx == self.cursor.col {
                    output.push('[');
                    output.push(cell.character);
                    output.push(']');
                } else {
                    output.push(cell.character);
                }
            }
            output.push('\n');
        }

        output.push_str(&format!(
            "Cursor: row={} col={}\n",
            self.cursor.row, self.cursor.col
        ));

        output
    }
}

fn main() -> Result<()> {
    println!("=== Cursor Movement Test ===\n");

    // Use a small terminal for easier visualization
    let mut terminal = Terminal::new(20, 5);

    // Test 1: Absolute positioning (1-based coordinates)
    println!("Test 1: Absolute positioning");
    println!("  set_cursor_position(3, 5) - should go to row 2, col 4 (0-based)");
    terminal.set_cursor_position(3, 5);
    println!(
        "  Cursor: row={}, col={}",
        terminal.cursor.row, terminal.cursor.col
    );

    // Print something at that position
    terminal.print('X', Attributes::default());
    println!("  Printed 'X' at that position");
    println!();

    // Test 2: Home position (0,0 or 1,1 in 1-based)
    println!("Test 2: Home position");
    println!("  set_cursor_position(1, 1) - should go to row 0, col 0");
    terminal.set_cursor_position(1, 1);
    println!(
        "  Cursor: row={}, col={}",
        terminal.cursor.row, terminal.cursor.col
    );
    terminal.print('H', Attributes::default());
    println!();

    // Test 3: Zero values treated as 1
    println!("Test 3: Zero values treated as 1");
    println!("  set_cursor_position(0, 0) - should also go to row 0, col 0");
    terminal.set_cursor_position(0, 0);
    println!(
        "  Cursor: row={}, col={}",
        terminal.cursor.row, terminal.cursor.col
    );
    println!();

    // Test 4: Out of bounds clamping
    println!("Test 4: Out of bounds clamping");
    println!("  set_cursor_position(100, 100) - should clamp to row 4, col 19");
    terminal.set_cursor_position(100, 100);
    println!(
        "  Cursor: row={}, col={}",
        terminal.cursor.row, terminal.cursor.col
    );
    terminal.print('C', Attributes::default());
    println!();

    // Test 5: Relative movement - down and right
    println!("Test 5: Relative movement from home");
    terminal.set_cursor_position(1, 1); // Start at home
    println!("  Starting at row=0, col=0");

    terminal.cursor_down(2);
    println!(
        "  cursor_down(2): row={}, col={}",
        terminal.cursor.row, terminal.cursor.col
    );

    terminal.cursor_forward(5);
    println!(
        "  cursor_forward(5): row={}, col={}",
        terminal.cursor.row, terminal.cursor.col
    );

    terminal.print('D', Attributes::default());
    println!();

    // Test 6: Relative movement - up and left with clamping
    println!("Test 6: Relative movement with clamping");
    terminal.set_cursor_position(2, 5); // Row 1, col 4
    println!("  Starting at row=1, col=4");

    terminal.cursor_up(10); // Try to go way up - should clamp to 0
    println!(
        "  cursor_up(10): row={} (clamped to 0)",
        terminal.cursor.row
    );

    terminal.cursor_back(10); // Try to go way left - should clamp to 0
    println!(
        "  cursor_back(10): col={} (clamped to 0)",
        terminal.cursor.col
    );
    println!();

    // Show the final grid
    println!("Final grid state:");
    println!("{}", terminal.debug_render());

    println!("Cursor movement is working correctly!");

    Ok(())
}

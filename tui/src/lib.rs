//! messily modified version of `tui` (MIT licensed) with async support for events
//!
//! (note: still uses synchronous stdout api since crossterm doesn't have tokio support
//! and no way i'm porting crossterm to tokio)

#[macro_use]
extern crate log;

pub extern crate crossterm;
pub extern crate tui;

use std::io::{Stdout, Write};
use std::mem;

use anyhow::Result;
use crossterm::{
    cursor::{MoveTo, Show},
    execute, queue,
    style::{
        Attribute as CAttribute, Color as CColor, Print, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
    terminal::{self, Clear, ClearType},
};
use futures::future::Future;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    style::{Color, Modifier},
    widgets::Widget,
};

pub struct Terminal<W> {
    stdout: W,
    buffers: [Buffer; 2],
    current: usize,
    hidden_cursor: bool,
    viewport: Viewport,
}

impl<W: Write> Terminal<W> {
    pub fn new(stdout: W) -> Result<Self> {
        let (width, height) = terminal::size()?;
        let size = Rect::new(0, 0, width, height);
        Terminal::with_options(
            stdout,
            TerminalOptions {
                viewport: Viewport {
                    area: size,
                    resize_behavior: ResizeBehavior::Auto,
                },
            },
        )
    }

    pub fn with_options(stdout: W, options: TerminalOptions) -> Result<Self> {
        Ok(Terminal {
            stdout,
            buffers: [
                Buffer::empty(options.viewport.area),
                Buffer::empty(options.viewport.area),
            ],
            current: 0,
            hidden_cursor: false,
            viewport: options.viewport,
        })
    }

    pub fn pre_draw(&mut self) -> Result<()> {
        self.autoresize()?;
        Ok(())
    }

    pub fn post_draw(&mut self) -> Result<()> {
        self.flush()?;

        self.buffers[1 - self.current].reset();
        self.current = 1 - self.current;

        self.stdout.flush()?;
        Ok(())
    }

    pub async fn draw<F, F2>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Frame<W>) -> F2,
        F2: Future<Output = ()>,
    {
        self.autoresize()?;

        let mut frame = self.get_frame();
        f(&mut frame).await;
        self.flush()?;

        self.buffers[1 - self.current].reset();
        self.current = 1 - self.current;

        self.stdout.flush()?;
        Ok(())
    }

    fn autoresize(&mut self) -> Result<()> {
        if self.viewport.resize_behavior == ResizeBehavior::Auto {
            let size = self.size()?;
            if size != self.viewport.area {
                self.resize(size)?;
            }
        };
        Ok(())
    }

    fn resize(&mut self, area: Rect) -> Result<()> {
        self.buffers[self.current].resize(area);
        self.buffers[1 - self.current].resize(area);
        self.viewport.area = area;
        self.clear()
    }

    fn clear(&mut self) -> Result<()> {
        execute!(self.stdout, Clear(ClearType::All))?;
        // Reset the back buffer to make sure the next update will redraw everything.
        self.buffers[1 - self.current].reset();
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let previous_buffer = &self.buffers[1 - self.current];
        let current_buffer = &self.buffers[self.current];
        let updates = previous_buffer
            .diff(current_buffer)
            .into_iter()
            .map(|(a, b, c)| (a, b, c.clone()))
            .collect::<Vec<_>>();
        self.draw_backend(updates.into_iter())
    }

    pub fn get_frame(&mut self) -> Frame<W> {
        Frame {
            terminal: self,
            cursor_position: None,
        }
    }

    fn draw_backend<I>(&mut self, content: I) -> Result<()>
    where
        I: Iterator<Item = (u16, u16, Cell)>,
    {
        let mut fg = Color::Reset;
        let mut bg = Color::Reset;
        let mut modifier = Modifier::empty();
        let mut last_pos: Option<(u16, u16)> = None;
        for (x, y, cell) in content {
            // Move the cursor if the previous location was not (x - 1, y)
            if !matches!(last_pos, Some(p) if x == p.0 + 1 && y == p.1) {
                queue!(self.stdout, MoveTo(x, y))?;
            }
            last_pos = Some((x, y));
            if cell.modifier != modifier {
                let diff = ModifierDiff {
                    from: modifier,
                    to: cell.modifier,
                };
                diff.queue(&mut self.stdout)?;
                modifier = cell.modifier;
            }
            if cell.fg != fg {
                let color = color_conv(cell.fg);
                queue!(self.stdout, SetForegroundColor(color))?;
                fg = cell.fg;
            }
            if cell.bg != bg {
                let color = color_conv(cell.bg);
                queue!(self.stdout, SetBackgroundColor(color))?;
                bg = cell.bg;
            }

            queue!(self.stdout, Print(&cell.symbol))?;
        }

        queue!(
            self.stdout,
            SetForegroundColor(CColor::Reset),
            SetBackgroundColor(CColor::Reset),
            SetAttribute(CAttribute::Reset)
        )?;

        Ok(())
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current]
    }

    pub fn show_cursor(&mut self) -> Result<()> {
        execute!(self.stdout, Show)?;
        self.hidden_cursor = false;
        Ok(())
    }

    pub fn set_cursor(&mut self, x: u16, y: u16) -> Result<()> {
        execute!(self.stdout, MoveTo(x, y))?;
        Ok(())
    }

    pub fn size(&self) -> Result<Rect> {
        let (width, height) = terminal::size()?;
        Ok(Rect::new(0, 0, width, height))
    }
}

fn color_conv(color: Color) -> CColor {
    match color {
        Color::Reset => CColor::Reset,
        Color::Black => CColor::Black,
        Color::Red => CColor::DarkRed,
        Color::Green => CColor::DarkGreen,
        Color::Yellow => CColor::DarkYellow,
        Color::Blue => CColor::DarkBlue,
        Color::Magenta => CColor::DarkMagenta,
        Color::Cyan => CColor::DarkCyan,
        Color::Gray => CColor::Grey,
        Color::DarkGray => CColor::DarkGrey,
        Color::LightRed => CColor::Red,
        Color::LightGreen => CColor::Green,
        Color::LightBlue => CColor::Blue,
        Color::LightYellow => CColor::Yellow,
        Color::LightMagenta => CColor::Magenta,
        Color::LightCyan => CColor::Cyan,
        Color::White => CColor::White,
        Color::Indexed(i) => CColor::AnsiValue(i),
        Color::Rgb(r, g, b) => CColor::Rgb { r, g, b },
    }
}

pub struct Frame<'a, W> {
    terminal: &'a mut Terminal<W>,
    cursor_position: Option<(u16, u16)>,
}

impl<'a, W: Write> Frame<'a, W> {
    pub fn set_cursor(&mut self, x: u16, y: u16) {
        self.cursor_position = Some((x, y));
    }

    pub fn size(&self) -> Rect {
        self.terminal.viewport.area
    }

    pub fn render_widget<W2>(&mut self, widget: W2, area: Rect)
    where
        W2: Widget,
    {
        widget.render(area, self.terminal.current_buffer_mut());
    }
}

#[derive(Debug)]
struct ModifierDiff {
    pub from: Modifier,
    pub to: Modifier,
}

impl ModifierDiff {
    fn queue<W: Write>(&self, w: &mut W) -> Result<()> {
        //use crossterm::Attribute;
        let removed = self.from - self.to;
        if removed.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CAttribute::NoReverse))?;
        }
        if removed.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CAttribute::NormalIntensity))?;
            if self.to.contains(Modifier::DIM) {
                queue!(w, SetAttribute(CAttribute::Dim))?;
            }
        }
        if removed.contains(Modifier::ITALIC) {
            queue!(w, SetAttribute(CAttribute::NoItalic))?;
        }
        if removed.contains(Modifier::UNDERLINED) {
            queue!(w, SetAttribute(CAttribute::NoUnderline))?;
        }
        if removed.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CAttribute::NormalIntensity))?;
        }
        if removed.contains(Modifier::CROSSED_OUT) {
            queue!(w, SetAttribute(CAttribute::NotCrossedOut))?;
        }
        if removed.contains(Modifier::SLOW_BLINK) || removed.contains(Modifier::RAPID_BLINK) {
            queue!(w, SetAttribute(CAttribute::NoBlink))?;
        }

        let added = self.to - self.from;
        if added.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CAttribute::Reverse))?;
        }
        if added.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CAttribute::Bold))?;
        }
        if added.contains(Modifier::ITALIC) {
            queue!(w, SetAttribute(CAttribute::Italic))?;
        }
        if added.contains(Modifier::UNDERLINED) {
            queue!(w, SetAttribute(CAttribute::Underlined))?;
        }
        if added.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CAttribute::Dim))?;
        }
        if added.contains(Modifier::CROSSED_OUT) {
            queue!(w, SetAttribute(CAttribute::CrossedOut))?;
        }
        if added.contains(Modifier::SLOW_BLINK) {
            queue!(w, SetAttribute(CAttribute::SlowBlink))?;
        }
        if added.contains(Modifier::RAPID_BLINK) {
            queue!(w, SetAttribute(CAttribute::RapidBlink))?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ResizeBehavior {
    Fixed,
    Auto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    area: Rect,
    resize_behavior: ResizeBehavior,
}

#[derive(Debug, Clone, PartialEq)]
/// Options to pass to [`Terminal::with_options`]
pub struct TerminalOptions {
    /// Viewport used to draw to the terminal
    pub viewport: Viewport,
}

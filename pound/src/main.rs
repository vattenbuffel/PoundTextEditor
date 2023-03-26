use std::io::{self, stdout, Write};
use std::time::Duration;
use textwrap::wrap;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::ClearType;
use crossterm::{cursor, event, execute, queue, terminal};

const VERSION: &str = "1.0.0";

struct CleanUp;
impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Failed to disable raw mode")
    }
}

struct Reader;
impl Reader {
    fn read_event(&self) -> crossterm::Result<Event> {
        loop {
            if event::poll(Duration::from_millis(500))? {
                return Ok(event::read()?);
            }
        }
    }
}

struct CursorController {
    x: usize,
    y: usize,
    x_max: usize,
    y_max: usize,
}

impl CursorController {
    fn new(win_size: (usize, usize)) -> CursorController {
        return Self {
            x: 0,
            y: 0,
            x_max: win_size.0 - 1,
            y_max: win_size.1 - 1,
        };
    }

    fn move_cursor(&mut self, direction: KeyCode) {
        match direction {
            KeyCode::Up => self.y = self.y.saturating_sub(1),
            KeyCode::Down => self.y = std::cmp::min(self.y + 1, self.y_max),
            KeyCode::Left => self.x = self.x.saturating_sub(1),
            KeyCode::Right => self.x = std::cmp::min(self.x + 1, self.x_max),
            _ => unreachable!(),
        }
    }
}

struct EditorContents {
    content: String,
}
impl EditorContents {
    fn new() -> Self {
        return Self {
            content: String::new(),
        };
    }

    fn push(&mut self, ch: char) {
        self.content.push(ch)
    }

    fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }

}
impl io::Write for EditorContents {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                return Ok(s.len());
            }
            Err(_) => Err(io::ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let out = write!(stdout(), "{}", self.content);
        stdout().flush()?;
        self.content.clear();
        return out;
    }
}

struct Output {
    win_size: (usize, usize),
    editor_contents: EditorContents,
    cursor_controller: CursorController,
}
impl Output {
    fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap();

        return Self {
            win_size,
            editor_contents: EditorContents::new(),
            cursor_controller: CursorController::new(win_size),
        };
    }

    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    fn draw_rows(&mut self) {
        let screen_colums = self.win_size.0;
        let screen_rows = self.win_size.1;

        for i in 0..screen_rows {
            if i == screen_rows / 3 {
                let welcome = format!("Pound editor -- Version {}", VERSION);
                if welcome.len() > screen_colums {
                    let wrapped_welcome_weird = wrap(&welcome, screen_colums);
                    for string in wrapped_welcome_weird{
                        self.editor_contents.push_str(&string.to_string());
                    }
                } else {
                    let mut padding = (screen_colums - welcome.len()) / 2;
                    if padding != 0 {
                        self.editor_contents.push('~');
                        padding -= 1;
                    }
                    (0..padding).for_each(|_| self.editor_contents.push(' '));
                    self.editor_contents.push_str(&welcome);
                }
            } else {
                self.editor_contents.push('~');
            }

            queue!(
                self.editor_contents,
                terminal::Clear(ClearType::UntilNewLine)
            )
            .unwrap();
            if i < screen_rows - 1 {
                self.editor_contents.push_str("\r\n");
            }
        }
    }

    fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor_controller.move_cursor(direction);
    }

    fn refresh_screen(&mut self) -> crossterm::Result<()> {
        queue!(self.editor_contents, cursor::Hide, cursor::MoveTo(0, 0))?;
        Self::clear_screen()?;
        self.draw_rows();
        let cursor_x = self.cursor_controller.x as u16;
        let cursor_y = self.cursor_controller.y as u16;
        queue!(
            self.editor_contents,
            cursor::MoveTo(cursor_x, cursor_y),
            cursor::Show
        )?;
        return self.editor_contents.flush();
    }

    fn process_resize(&mut self, x: usize, y: usize) {
        self.win_size = (x, y);
        self.cursor_controller.x_max = x - 1;
        self.cursor_controller.y_max = y - 1;
        self.cursor_controller.x =
            std::cmp::min(self.cursor_controller.x, self.cursor_controller.x_max);
        self.cursor_controller.y =
            std::cmp::min(self.cursor_controller.y, self.cursor_controller.y_max);
    }
}

struct Editor {
    reader: Reader,
    output: Output,
}

impl Editor {
    fn new() -> Self {
        return Self {
            reader: Reader,
            output: Output::new(),
        };
    }

    fn process_keypress(&mut self, key_event: KeyEvent) -> crossterm::Result<bool> {
        match key_event {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
            } => {
                return Ok(false);
            }
            KeyEvent {
                code: direction @ (KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right),
                modifiers: KeyModifiers::NONE,
            } => {
                self.output.move_cursor(direction);
            }
            _ => {}
        }

        return Ok(true);
    }

    fn process_resize(&mut self, x: usize, y: usize) {
        self.output.process_resize(x, y);
    }

    fn process_event(&mut self) -> crossterm::Result<bool> {
        let result = match self.reader.read_event()? {
            Event::Resize(x, y) => {
                self.process_resize(x as usize, y as usize);
                true
            }
            Event::Key(key_event) => self.process_keypress(key_event)?,
            _ => true,
        };

        Ok(result)
    }

    fn run(&mut self) -> crossterm::Result<bool> {
        self.output.refresh_screen()?;
        return self.process_event();
    }
}

fn main() -> crossterm::Result<()> {
    let _ = CleanUp;
    terminal::enable_raw_mode()?;
    let mut editor = Editor::new();

    while editor.run()? {}
    Output::clear_screen()?;

    return Ok(());
}

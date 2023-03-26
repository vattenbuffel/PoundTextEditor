use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::{event, terminal};
use std::time::Duration;

struct CleanUp;
impl Drop for CleanUp {
    fn drop(&mut self){
        terminal::disable_raw_mode().expect("Failed to disable raw mode")
    }
}

fn main() -> crossterm::Result<()> {
    let _ = CleanUp;

    terminal::enable_raw_mode()?;

    loop {
        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event {
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: event::KeyModifiers::NONE,
                    } => break,
                    _ => {
                        // todo
                    }
                }

                println!("{:?}\r", key_event);
            }
        } else {
            println!("no inputs yet\r");
        }
    }

    return Ok(());
}

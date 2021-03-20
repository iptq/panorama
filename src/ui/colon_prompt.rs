use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use super::input::{HandlesInput, InputResult};
use super::TermType;

#[derive(Clone, Default, Debug)]
pub struct ColonPrompt {
    pub value: String,
}

impl ColonPrompt {
    pub fn init(term: TermType) -> Result<Self> {
        let s = term.size()?;
        term.set_cursor(1, s.height - 1)?;
        term.show_cursor()?;
        Ok(ColonPrompt::default())
    }
}

impl Drop for ColonPrompt {
    fn drop(&mut self) {}
}

impl HandlesInput for ColonPrompt {
    fn handle_key(&mut self, term: TermType, evt: KeyEvent) -> Result<InputResult> {
        let KeyEvent { code, .. } = evt;
        match code {
            KeyCode::Esc => return Ok(InputResult::Pop),
            KeyCode::Char(c) => {
                let mut b = [0; 2];
                self.value += c.encode_utf8(&mut b);
            }
            KeyCode::Enter => {
                let cmd = self.value.clone();
                self.value.clear();
                debug!("executing colon command: {:?}", cmd);
                return Ok(InputResult::Pop);
            }
            KeyCode::Backspace => {
                let mut new_len = self.value.len();
                if new_len > 0 {
                    new_len -= 1;
                    self.value.truncate(new_len);
                } else {
                    return Ok(InputResult::Pop);
                }
            }
            _ => {}
        }

        Ok(InputResult::Ok)
    }
}

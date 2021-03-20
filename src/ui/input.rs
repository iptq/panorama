use std::any::Any;
use std::fmt::Debug;
use std::sync::{
    atomic::{AtomicBool, AtomicI8, Ordering},
    Arc,
};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use downcast_rs::Downcast;

use super::colon_prompt::ColonPrompt;
use super::TermType;

pub trait HandlesInput: Any + Debug + Downcast {
    fn handle_key(&mut self, term: TermType, evt: KeyEvent) -> Result<InputResult> {
        Ok(InputResult::Ok)
    }
}

downcast_rs::impl_downcast!(HandlesInput);

pub enum InputResult {
    Ok,

    /// Push a new state
    Push(Box<dyn HandlesInput>),

    /// Pops a state from the stack
    Pop,
}

#[derive(Debug)]
pub struct BaseInputHandler(pub Arc<AtomicBool>, pub Arc<AtomicI8>);

impl HandlesInput for BaseInputHandler {
    fn handle_key(&mut self, term: TermType, evt: KeyEvent) -> Result<InputResult> {
        let KeyEvent { code, .. } = evt;
        match code {
            KeyCode::Char('q') => self.0.store(true, Ordering::Relaxed),
            KeyCode::Char('j') => self.1.store(1, Ordering::Relaxed),
            KeyCode::Char('k') => self.1.store(-1, Ordering::Relaxed),
            KeyCode::Char(':') => {
                let colon_prompt = Box::new(ColonPrompt::init(term)?);
                return Ok(InputResult::Push(colon_prompt));
            }
            _ => {}
        }

        Ok(InputResult::Ok)
    }
}

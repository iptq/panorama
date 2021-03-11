/// Commands that are sent from the UI to the scripting VM
pub enum UiCommand {
    HSplit,
    VSplit,
    OpenMessage,
}

/// Updates that are sent from the scripting VM to the UI
pub enum UiUpdate {}

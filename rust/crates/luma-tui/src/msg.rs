use luma_protocol::Event;

#[derive(Clone, Debug)]
pub enum Msg {
    KeyChar(char),
    Backspace,
    DeleteForward,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    /// Kill from start of prompt through character before cursor (readline Ctrl-u).
    ClearToStart,
    /// Delete the word before the cursor (readline Ctrl-w).
    DeleteWordBack,
    Submit,
    SelectNext,
    SelectPrev,
    SelectPageUp,
    SelectPageDown,
    /// ActionPicker: 1-based digit → select and run that action.
    PickActionDigit(usize),
    OpenHelp,
    OpenDoctor,
    OpenSettings,
    OpenCommands,
    OpenActions,
    ToggleSetting,
    FocusNext,
    HistoryOlder,
    HistoryNewer,
    Quit,
    Cancel,
    Redraw,
    Engine(Event),
    Resize {
        width: u16,
        height: u16,
    },
    Tick,
    /// Fire after input debounce quiet period.
    FlushSearch,
    /// Soft-refresh Hub windows while the empty Hub is visible.
    RefreshHub,
    /// Broadcast subscriber lagged — resync UI from engine.
    BroadcastLagged,
    /// Toggle stacked preview on narrow terminals.
    TogglePreview,
    /// Toggle doctor Summary / Raw JSON.
    ToggleDoctorRaw,
    /// Terminal regained focus (switch back to Luma).
    FocusGained,
}

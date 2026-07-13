use luma_protocol::Event;

#[derive(Clone, Debug)]
pub enum Msg {
    KeyChar(char),
    Backspace,
    Submit,
    SelectNext,
    SelectPrev,
    OpenHelp,
    OpenDoctor,
    OpenActions,
    Quit,
    Cancel,
    Redraw,
    Engine(Event),
    Resize,
    Tick,
    /// Fire after input debounce quiet period.
    FlushSearch,
}

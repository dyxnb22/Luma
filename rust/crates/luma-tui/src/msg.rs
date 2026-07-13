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
    Quit,
    Cancel,
    Redraw,
    Engine(Event),
    Resize,
    Tick,
}

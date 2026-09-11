/// Structured status tone — render colors from this, not string parsing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusTone {
    #[default]
    Neutral,
    Success,
    Progress,
    Warning,
    Error,
    Permission,
}

#[derive(Clone, Debug)]
pub struct StatusLine {
    pub text: String,
    pub tone: StatusTone,
}

impl StatusLine {
    pub fn set(&mut self, text: impl Into<String>, tone: StatusTone) {
        self.text = text.into();
        self.tone = tone;
    }
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            text: "Ready".into(),
            tone: StatusTone::Neutral,
        }
    }
}

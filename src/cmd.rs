pub enum ClackAudioHostCommand {
    Help,
    Note(u16),
    StopNote,
    Quit,
    Invalid
}

impl ClackAudioHostCommand {
    fn try_parse_from_tokens(tokens: Vec<&str>) -> Option<Self> {
        match *tokens.get(0)? {
            "help" | "h" | "?" => Some(Self::Help),
            "note" | "n" => Some(Self::Note(tokens.get(1)?.parse::<u16>().ok()?)),
            "stop" | "s" => Some(Self::StopNote),
            "quit" | "q" => Some(Self::Quit),
            _ => None
        }
    }
}

impl From<&str> for ClackAudioHostCommand {
    fn from(value: &str) -> Self {
        Self::try_parse_from_tokens(
            value.split_whitespace().collect()
        ).unwrap_or(Self::Invalid)
    }
}

pub fn print_help() {
    println!("help|h|?      - Show this help message");
    println!("note|n <note> - Play a MIDI note");
    println!("stop|s        - Stop playing the current note");
}
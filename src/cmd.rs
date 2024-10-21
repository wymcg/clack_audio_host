pub enum ClackAudioHostCommand {
    Help,
    StartNote(u16),
    StopNote(u16),
    ListFeatures,
    Quit,
    Invalid,
}

impl ClackAudioHostCommand {
    fn try_parse_from_tokens(tokens: Vec<&str>) -> Option<Self> {
        match *tokens.get(0)? {
            "help" | "h" | "?" => Some(Self::Help),
            "note" | "n" => Some(Self::StartNote(tokens.get(1)?.parse::<u16>().ok()?)),
            "stop" | "s" => Some(Self::StopNote(tokens.get(1)?.parse::<u16>().ok()?)),
            "quit" | "q" => Some(Self::Quit),
            "lsf" => Some(Self::ListFeatures),
            _ => None,
        }
    }
}

impl From<&str> for ClackAudioHostCommand {
    fn from(value: &str) -> Self {
        Self::try_parse_from_tokens(value.split_whitespace().collect()).unwrap_or(Self::Invalid)
    }
}

pub fn print_help() {
    println!("help|h|?      - Show this help message");
    println!("note|n <note> - Start playing a note");
    println!("stop|s <note> - Stop playing a note");
    println!("lsf           - List plugin features");
}

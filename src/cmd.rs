pub enum ClackAudioHostCommand {
    Help,
    StartNote(u16),
    StopNote(u16),
    ListFeatures,
    ListParams,
    ParamInfo(u32),
    SetParam(u32, f64),
    Quit,
    Invalid,
}

impl ClackAudioHostCommand {
    fn try_parse_from_tokens(tokens: Vec<&str>) -> Option<Self> {
        match *tokens.get(0)? {
            "help" | "h" | "?" => Some(Self::Help),
            "nb" => Some(Self::StartNote(tokens.get(1)?.parse::<u16>().ok()?)),
            "ne" => Some(Self::StopNote(tokens.get(1)?.parse::<u16>().ok()?)),
            "q" => Some(Self::Quit),
            "lsf" => Some(Self::ListFeatures),
            "lsp" => Some(Self::ListParams),
            "pi" => Some(Self::ParamInfo(tokens.get(1)?.parse::<u32>().ok()?)),
            "ps" => Some(Self::SetParam(tokens.get(1)?.parse::<u32>().ok()?, tokens.get(2)?.parse::<f64>().ok()?)),
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
    println!("help|h|?                  - Show this help message");
    println!("nb <note>                 - Start playing a note");
    println!("ne <note>                 - Stop playing a note");
    println!("pi <param_id>             - Show information about a specific parameter");
    println!("lsf                       - List plugin features");
    println!("lsp                       - List plugin parameters");
    println!("ps <param_id> <new_value> - Set a parameter");
}

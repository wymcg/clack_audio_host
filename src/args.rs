use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version)]
pub(crate) struct ClackAudioHostArgs {
    /// Path to a CLAP plugin
    pub path: String,

    #[arg(short, long)]
    pub verbose: bool,
    
    #[arg(short, long, default_value="false")]
    /// Attempt to connect the plugin to system JACK I/O
    pub use_system_io: bool,
}

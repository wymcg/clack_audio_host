mod args;

use crate::args::ClackAudioHostArgs;

use clap::Parser;
use clack_host::prelude::*;


struct ClackAudioHostShared;

impl SharedHandler for ClackAudioHostShared {
    fn request_restart(&self) {
        /* For now, this is empty! */
    }

    fn request_process(&self) {
        /* For now, this is empty! */
    }

    fn request_callback(&self) {
        /* For now, this is empty! */
    }
}

struct ClackAudioHost;

impl HostHandlers for ClackAudioHost {
    type Shared<'a> = ();
    type MainThread<'a> = ();
    type AudioProcessor<'a> = ();
}

fn main() {
    let args = ClackAudioHostArgs::parse();


}

mod args;

use crate::args::ClackAudioHostArgs;
use clack_host::events::event_types::NoteOnEvent;
use clack_host::events::Match::All;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use clack_host::prelude::*;
use clap::Parser;
use jack::{
    contrib::ClosureProcessHandler, AudioIn, AudioOut, Client, Control, Port, ProcessScope,
};
use log::{debug, error, info, warn};

const HOST_NAME: &str = env!("CARGO_PKG_NAME");
const HOST_VENDOR: &str = env!("CARGO_PKG_AUTHORS");
const HOST_URL: &str = "https://github.com/wymcg/clack_audio_host";
const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

const PLUGIN_CONFIG_MIN_FRAMES: u32 = 1;
const PLUGIN_CONFIG_MAX_FRAMES: u32 = 4096;

struct ClackAudioHostShared;

impl<'a> SharedHandler<'a> for ClackAudioHostShared {
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
    type Shared<'a> = ClackAudioHostShared;
    type MainThread<'a> = ();
    type AudioProcessor<'a> = ();
}

fn main() {
    env_logger::init();

    info!("Starting {HOST_NAME} v{HOST_VERSION}");

    // Parse command line args
    let args = ClackAudioHostArgs::parse();

    // Set up the JACK client
    let (client, _status) = Client::new("clack_audio_host", jack::ClientOptions::NO_START_SERVER)
        .expect("Unable to create JACK client!");
    let mut port_out_l = client
        .register_port("out_l", AudioOut::default())
        .expect("Unable to create left output port!");
    let mut port_out_r = client
        .register_port("out_r", AudioOut::default())
        .expect("Unable to create right output port!");
    client
        .set_buffer_size(PLUGIN_CONFIG_MAX_FRAMES)
        .expect("Unable to set client buffer size!");

    // Create a host information object
    let host_info = HostInfo::new(HOST_NAME, HOST_VENDOR, HOST_URL, HOST_VERSION)
        .expect("Unable to create host information!");

    // Load the plugin and get the plugin factory
    let bundle = match unsafe { PluginBundle::load(&args.path) } {
        Ok(bundle) => bundle,
        Err(e) => {
            error!("Unable to load plugin bundle.");
            debug!("Error: {e}");
            return;
        }
    };
    let plugin_factory = match bundle.get_plugin_factory() {
        Some(factory) => factory,
        None => {
            error!("Plugin bundle does not contain a plugin factory.");
            return;
        }
    };

    // Pull the first plugin descriptor
    if plugin_factory.plugin_count() < 1 {
        error!("Plugin bundle contains no plugins.");
        return;
    } else if plugin_factory.plugin_count() > 1 {
        warn!("Plugin bundle contains more than one plugin. Only the first plugin will be loaded.");
    }
    let plugin_descriptor = plugin_factory
        .plugin_descriptor(0)
        .expect("Unable to pull the first plugin descriptor!");

    // Create an instance of the plugin
    let mut plugin_instance = match PluginInstance::<ClackAudioHost>::new(
        |_| ClackAudioHostShared,
        |_| (),
        &bundle,
        plugin_descriptor.id().expect("Unable to get plugin ID!"),
        &host_info,
    ) {
        Ok(instance) => instance,
        Err(e) => {
            error!("Unable to create an instance of the plugin.");
            debug!("Error: {e}");
            return;
        }
    };

    // Create the audio processor
    let audio_processor = match plugin_instance.activate(
        |_, _| (),
        PluginAudioConfiguration {
            sample_rate: client.sample_rate() as f64,
            min_frames_count: PLUGIN_CONFIG_MIN_FRAMES,
            max_frames_count: PLUGIN_CONFIG_MAX_FRAMES,
        },
    ) {
        Ok(processor) => processor,
        Err(e) => {
            error!("Unable to create an audio processor.");
            debug!("Error: {e}");
            return;
        }
    };

    // Create event I/O buffers
    let note_event = NoteOnEvent::new(0, Pckn::new(0u16, 0u16, 60u16, 0u32), 4.2); // Middle C!
    let input_events_buffer = [note_event];
    let mut output_events_buffer = EventBuffer::new();

    // Create audio I/O buffers/ports
    let mut input_audio_buffers = [[0.0f32; PLUGIN_CONFIG_MAX_FRAMES as usize]; 2];
    let mut output_audio_buffers = [[0.0f32; PLUGIN_CONFIG_MAX_FRAMES as usize]; 2];
    let mut input_ports = AudioPorts::with_capacity(2, 1);
    let mut output_ports = AudioPorts::with_capacity(2, 1);

    let mut audio_processor = audio_processor
        .start_processing()
        .expect("Unable to start processing audio.");

    let process_handler = ClosureProcessHandler::new(move |_client, process_scope| -> Control {
        let input_events = InputEvents::from_buffer(&input_events_buffer);
        let mut output_events = OutputEvents::from_buffer(&mut output_events_buffer);

        let input_audio = input_ports.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_input_only(
                input_audio_buffers
                    .iter_mut()
                    .map(|b| InputChannel::constant(b)),
            ),
        }]);
        let mut output_audio = output_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only(
                output_audio_buffers.iter_mut().map(|b| b.as_mut_slice()),
            ),
        }]);

        if let Err(e) = audio_processor.process(
            &input_audio,
            &mut output_audio,
            &input_events,
            &mut output_events,
            None,
            None,
        ) {
            error!("Unable to process plugin audio.");
            debug!("Error: {e}");
            return Control::Quit;
        }

        // Write output buffers to the JACK output ports
        port_out_l
            .as_mut_slice(process_scope)
            .copy_from_slice(&output_audio_buffers[0]);
        port_out_r
            .as_mut_slice(process_scope)
            .copy_from_slice(&output_audio_buffers[1]);

        Control::Continue
    });

    let _active_client = client
        .activate_async((), process_handler)
        .expect("Unable to activate client");

    // Keep the main thread alive
    loop {
        std::thread::park();
    }

    info!("Done.");
}

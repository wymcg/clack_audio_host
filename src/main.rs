mod args;

use clack_host::events::event_types::NoteOnEvent;
use crate::args::ClackAudioHostArgs;

use clap::Parser;
use clack_host::prelude::*;
use log::{debug, error, info, warn};

const HOST_NAME: &str = env!("CARGO_PKG_NAME");
const HOST_VENDOR: &str = env!("CARGO_PKG_AUTHORS");
const HOST_URL: &str = "https://github.com/wymcg/clack_audio_host";
const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

const PLUGIN_CONFIG_SAMPLE_RATE: f64 = 48_000.0;
const PLUGIN_CONFIG_MIN_FRAMES: u32 = 256;
const PLUGIN_CONFIG_MAX_FRAMES: u32 = 1024;

struct ClackAudioHostShared;

impl <'a> SharedHandler<'a> for ClackAudioHostShared {
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

    // Create a host information object
    let host_info = HostInfo::new(HOST_NAME, HOST_VENDOR, HOST_URL, HOST_VERSION).expect("Unable to create host information!");

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
    let plugin_descriptor = plugin_factory.plugin_descriptor(0).expect("Unable to pull the first plugin descriptor!");

    // Create an instance of the plugin
    let mut plugin_instance = match PluginInstance::<ClackAudioHost>::new(
        |_| ClackAudioHostShared,
        |_| (),
        &bundle,
        plugin_descriptor.id().expect("Unable to get plugin ID!"),
        &host_info
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
        PluginAudioConfiguration {sample_rate: PLUGIN_CONFIG_SAMPLE_RATE, min_frames_count: PLUGIN_CONFIG_MIN_FRAMES, max_frames_count: PLUGIN_CONFIG_MAX_FRAMES }
    ) {
        Ok(processor) => processor,
        Err(e) => {
            error!("Unable to create an audio processor.");
            debug!("Error: {e}");
            return;
        }
    };

    // Create event I/O buffers
    let note_event = NoteOnEvent::new(0, Pckn::new(0u16, 0u16, 12u16, 60u32), 4.2);
    let input_events_buffer = [note_event];
    let mut output_events_buffer = EventBuffer::new();

    // Create audio I/O buffers/ports
    let mut input_audio_buffers = [[0.0f32; PLUGIN_CONFIG_MAX_FRAMES as usize]; 2];
    let mut output_audio_buffers = [[0.0f32; PLUGIN_CONFIG_MAX_FRAMES as usize]; 2];
    let mut input_ports = AudioPorts::with_capacity(2, 1);
    let mut output_ports = AudioPorts::with_capacity(2, 1);

    let audio_processor = std::thread::scope(|s| s.spawn(|| {
        // Start audio processing thread
        let mut audio_processor = audio_processor.start_processing().expect("Unable to start processing audio.");

        let input_events = InputEvents::from_buffer(&input_events_buffer);
        let mut output_events = OutputEvents::from_buffer(&mut output_events_buffer);

        let input_audio = input_ports.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_input_only(
                input_audio_buffers.iter_mut().map(|b| InputChannel::constant(b))
            )
        }]);
        let mut output_audio = output_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only(
                output_audio_buffers.iter_mut().map(|b| b.as_mut_slice())
            )
        }]);

        let status = match audio_processor.process(
            &input_audio,
            &mut output_audio,
            &input_events,
            &mut output_events,
            None,
            None
        ) {
            Ok(status) => status,
            Err(e) => {
                error!("Error processing audio.");
                debug!("Error: {e}");
                return audio_processor.stop_processing();
            }
        };

        audio_processor.stop_processing()
    }).join().expect("Unable to join thread!"));

    debug!("Contents of output audio buffers: {output_audio_buffers:?}");

    plugin_instance.deactivate(audio_processor);

    info!("Done.");
}

mod args;
mod cmd;
mod midi;

use crate::args::ClackAudioHostArgs;
use crate::cmd::ClackAudioHostCommand;
use std::ops::Deref;

use crate::midi::add_raw_midi_to_event_buffer;
use clack_extensions::params::{ParamInfoBuffer, PluginParams};
use clack_host::events::event_types::{NoteOffEvent, NoteOnEvent, ParamValueEvent};
use clack_host::events::Match::All;
use clack_host::prelude::*;
use clack_host::utils::Cookie;
use clap::Parser;
use jack::{contrib::ClosureProcessHandler, AudioIn, AudioOut, Client, Control, MidiIn, RawMidi};
use linefeed::{Interface, ReadResult};
use std::sync::{Arc, Mutex};

const HOST_NAME: &str = env!("CARGO_PKG_NAME");
const HOST_VENDOR: &str = env!("CARGO_PKG_AUTHORS");
const HOST_URL: &str = "https://github.com/wymcg/clack_audio_host";
const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

const NOTE_VELOCITY: f64 = 100.0;

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
    println!("{HOST_NAME} v{HOST_VERSION}");

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
    let port_in_l = client
        .register_port("in_l", AudioIn::default())
        .expect("Unable to create left audio in port!");
    let port_in_r = client
        .register_port("in_r", AudioIn::default())
        .expect("Unable to create right audio in port!");
    let midi_in = client
        .register_port("midi_in", MidiIn::default())
        .expect("Unable to create MIDI in port!");
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
            eprintln!("Unable to load plugin bundle.");
            if args.verbose {
                eprintln!("Error: {e}");
            }
            return;
        }
    };
    let plugin_factory = match bundle.get_plugin_factory() {
        Some(factory) => factory,
        None => {
            eprintln!("Plugin bundle does not contain a plugin factory.");
            return;
        }
    };

    // Pull the first plugin descriptor
    if plugin_factory.plugin_count() < 1 {
        eprintln!("Plugin bundle contains no plugins.");
        return;
    } else if plugin_factory.plugin_count() > 1 {
        println!(
            "Plugin bundle contains more than one plugin. Only the first plugin will be loaded."
        );
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
            eprintln!("Unable to create an instance of the plugin.");
            if args.verbose {
                eprintln!("Error: {e}");
            }
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
            eprintln!("Unable to create an audio processor.");
            if args.verbose {
                eprintln!("Error: {e}");
            }
            return;
        }
    };

    // Create event I/O buffers
    let input_events_buffer = Arc::new(Mutex::new(EventBuffer::new()));
    let mut output_events_buffer = EventBuffer::new();

    // Create audio I/O buffers/ports
    let mut input_audio_buffers = [[0.0f32; PLUGIN_CONFIG_MAX_FRAMES as usize]; 2];
    let mut output_audio_buffers = [[0.0f32; PLUGIN_CONFIG_MAX_FRAMES as usize]; 2];
    let mut input_ports = AudioPorts::with_capacity(2, 1);
    let mut output_ports = AudioPorts::with_capacity(2, 1);

    // Create a copy of the input events buffer mutex for the JACK client process handler to use
    let thread_input_events_buffer = input_events_buffer.clone();

    // Start the audio processor
    let mut audio_processor = audio_processor
        .start_processing()
        .expect("Unable to start processing audio.");

    // Create the process handler for the JACK client
    let process_handler = ClosureProcessHandler::new(move |_client, process_scope| -> Control {
        let mut output_events = OutputEvents::from_buffer(&mut output_events_buffer);

        // Copy over input audio to the input audio buffers
        input_audio_buffers[0].copy_from_slice(port_in_l.as_slice(process_scope));
        input_audio_buffers[1].copy_from_slice(port_in_r.as_slice(process_scope));

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

        {
            let mut event_buffer = thread_input_events_buffer.lock().unwrap();
            for raw_midi in midi_in.iter(process_scope) {
                if let Err(e) = add_raw_midi_to_event_buffer(&mut *event_buffer, raw_midi, 1, 0) {
                    eprintln!("Unable to handle MIDI event: {e}");
                }
            }
        }

        if let Err(_e) = audio_processor.process(
            &input_audio,
            &mut output_audio,
            &(thread_input_events_buffer.lock().unwrap().as_input()),
            &mut output_events,
            None,
            None,
        ) {
            eprintln!("Unable to process plugin audio.");
            return Control::Quit;
        }

        // Write output buffers to the JACK output ports
        port_out_l
            .as_mut_slice(process_scope)
            .copy_from_slice(&output_audio_buffers[0]);
        port_out_r
            .as_mut_slice(process_scope)
            .copy_from_slice(&output_audio_buffers[1]);

        // Clear the input events buffer
        thread_input_events_buffer.lock().unwrap().clear();

        Control::Continue
    });

    // Start the JACK client
    let _active_client = client
        .activate_async((), process_handler)
        .expect("Unable to activate client");

    // Set up the REPL interface
    let interface = Interface::new(HOST_NAME).expect("Unable to create interface!");
    interface
        .set_prompt(">> ")
        .expect("Unable to set interface prompt!");

    // Run the command REPL
    while let ReadResult::Input(line) = interface.read_line().expect("Unable to read line") {
        match ClackAudioHostCommand::from(line.as_str()) {
            ClackAudioHostCommand::Help => {
                cmd::print_help();
            }
            ClackAudioHostCommand::ParamInfo(param_id) => {
                let mut plugin_handle = plugin_instance.plugin_handle();
                let plugin_params = match plugin_handle.get_extension::<PluginParams>() {
                    Some(p) => p,
                    None => {
                        println!("No plugin parameters found.");
                        continue;
                    }
                };

                for param_idx in 0..plugin_params.count(&mut plugin_handle) {
                    let mut param_info_buffer = ParamInfoBuffer::new();
                    if let Some(info) = plugin_params.get_info(
                        &mut plugin_handle,
                        param_idx,
                        &mut param_info_buffer,
                    ) {
                        if info.id != param_id {
                            continue;
                        }

                        println!(
                            "Name: {}",
                            String::from_utf8(Vec::from(info.name))
                                .unwrap_or("Unknown".to_string())
                        );

                        let module_name =
                            String::from_utf8(Vec::from(info.module)).unwrap_or(String::new());
                        if !module_name.is_empty() {
                            println!("Module: {module_name}");
                        }

                        println!(
                            "Value: {:?}",
                            plugin_params
                                .get_value(&mut plugin_handle, ClapId::from(info.id))
                                .unwrap()
                        );

                        println!(
                            "Range: ({}, {}) [default: {}]",
                            info.min_value, info.max_value, info.default_value
                        );
                        break;
                    }
                }
            }
            ClackAudioHostCommand::ListFeatures => {
                let features: Vec<String> = plugin_descriptor
                    .features()
                    .map(|cstr| cstr.to_string_lossy().to_string())
                    .collect();
                println!("{}", features.join(", "));
            }
            ClackAudioHostCommand::ListParams => {
                let mut plugin_handle = plugin_instance.plugin_handle();
                let plugin_params = match plugin_handle.get_extension::<PluginParams>() {
                    Some(p) => p,
                    None => {
                        println!("No plugin parameters found.");
                        continue;
                    }
                };

                for param_idx in 0..plugin_params.count(&mut plugin_handle) {
                    let mut param_info_buffer = ParamInfoBuffer::new();
                    let param = plugin_params
                        .get_info(&mut plugin_handle, param_idx, &mut param_info_buffer)
                        .unwrap();
                    let param_name =
                        String::from_utf8(Vec::from(param.name)).unwrap_or("Unknown".to_string());
                    println!("{:3}: {}", param.id, param_name);
                }
            }
            ClackAudioHostCommand::SetParam(param_id, value) => input_events_buffer
                .lock()
                .unwrap()
                .push(&ParamValueEvent::new(
                    0,
                    ClapId::from(param_id),
                    Pckn::new(0u16, 0u16, All, All),
                    value,
                    Cookie::empty(),
                )),
            ClackAudioHostCommand::Invalid => {
                eprintln!("Invalid command. See 'help' for usage information.")
            }
            ClackAudioHostCommand::Quit => break,
        };
        println!();
    }
}

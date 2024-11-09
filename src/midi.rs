use clack_host::events::event_types::{NoteOffEvent, NoteOnEvent};
use clack_host::events::Match::All;
use clack_host::events::Pckn;
use clack_host::prelude::EventBuffer;
use jack::RawMidi;

#[derive(Clone)]
enum MidiEventType {
    NoteOff,
    NoteOn,
    NoteAftertouch,
    Controller,
    ProgramChange,
    ChannelAftertouch,
    PitchBend,
    Unknown,
}

impl From<u8> for MidiEventType {
    fn from(value: u8) -> Self {
        match value {
            0x8 => MidiEventType::NoteOff,
            0x9 => MidiEventType::NoteOn,
            0xA => MidiEventType::NoteAftertouch,
            0xB => MidiEventType::Controller,
            0xC => MidiEventType::ProgramChange,
            0xD => MidiEventType::ChannelAftertouch,
            0xE => MidiEventType::PitchBend,
            _ => MidiEventType::Unknown,
        }
    }
}

pub fn add_raw_midi_to_event_buffer<'a>(
    event_buffer: &'a mut EventBuffer,
    raw_midi: RawMidi<'a>,
    port: u16,
    note_id: u32,
) -> Result<(), &'a str> {
    let event_type: MidiEventType = MidiEventType::from((raw_midi.bytes[0] & 0xF0) >> 4);
    let midi_channel: u8 = raw_midi.bytes[0] & 0x0F;

    match event_type {
        MidiEventType::NoteOff => event_buffer.push(&NoteOffEvent::new(
            raw_midi.time,
            Pckn::new(port, midi_channel, raw_midi.bytes[1], All),
            raw_midi.bytes[2] as f64 / 127.0,
        )),
        MidiEventType::NoteOn => event_buffer.push(&NoteOnEvent::new(
            raw_midi.time,
            Pckn::new(port, midi_channel, raw_midi.bytes[1], All),
            raw_midi.bytes[2] as f64 / 127.0,
        )),
        MidiEventType::NoteAftertouch => {
            return Err("Note aftertouch events are not yet supported!")
        }
        MidiEventType::Controller => return Err("Controller events are not yet supported!"),
        MidiEventType::ProgramChange => return Err("Program change events are not yet supported!"),
        MidiEventType::ChannelAftertouch => {
            return Err("Channel aftertouch events are not yet supported!")
        }
        MidiEventType::PitchBend => return Err("Pitch bend events are not yet supported!"),
        MidiEventType::Unknown => return Err("Unknown MIDI event type!"),
    };

    Ok(())
}

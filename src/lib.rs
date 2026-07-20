// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{Emitter, Manager};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Clone, serde::Serialize)]
struct MidiEvent {
    status: u8,
    data1: u8,
    data2: u8,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
struct OscEvent {
    address: String,
    args: Vec<serde_json::Value>,
}

fn setup_midi(app_handle: tauri::AppHandle) -> anyhow::Result<()> {
    let midi_in = midir::MidiInput::new("shekere-midi-scan")?;
    let ports = midi_in.ports();

    for port in ports {
        let port_name = midi_in.port_name(&port)?;
        let handle_clone = app_handle.clone();

        // Re-create MidiInput for each connection because .connect() consumes it
        let midi_in_for_conn = midir::MidiInput::new("shekere-midi-input")?;

        // Connect to the port
        let _conn = midi_in_for_conn.connect(
            &port,
            &port_name,
            move |_timestamp, message, _| {
                if message.len() >= 3 {
                    let event = MidiEvent {
                        status: message[0],
                        data1: message[1],
                        data2: message[2],
                    };
                    let _ = handle_clone.emit("midi-event", event);
                }
            },
            (),
        );

        if let Ok(conn) = _conn {
            Box::leak(Box::new(conn));
            println!("Connected to MIDI port: {}", port_name);
        }
    }

    Ok(())
}

fn setup_osc(app_handle: tauri::AppHandle) -> anyhow::Result<()> {
    use std::net::{Ipv4Addr, UdpSocket};

    let addr = (Ipv4Addr::UNSPECIFIED, 2020);
    let socket = UdpSocket::bind(addr)?;
    println!("OSC listening on port 2020");

    let mut buf = [0u8; 16384];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, _addr)) => {
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                handle_packet(packet, &app_handle);
            }
            Err(e) => {
                eprintln!("Error receiving UDP packet: {}", e);
            }
        }
    }
}

fn osc_argument_to_json(argument: rosc::OscType) -> serde_json::Value {
    use rosc::OscType;

    match argument {
        OscType::Float(value) => serde_json::json!(value),
        OscType::Double(value) => serde_json::json!(value),
        OscType::Int(value) => serde_json::json!(value),
        OscType::Long(value) => serde_json::json!(value),
        OscType::String(value) => serde_json::json!(value),
        OscType::Bool(value) => serde_json::json!(value),
        OscType::Nil => serde_json::Value::Null,
        OscType::Blob(value) => serde_json::json!(value),
        _ => serde_json::Value::Null,
    }
}

fn visit_osc_packet<F>(packet: rosc::OscPacket, emit_event: &mut F)
where
    F: FnMut(OscEvent),
{
    use rosc::OscPacket;

    match packet {
        OscPacket::Message(msg) => {
            let args = msg.args.into_iter().map(osc_argument_to_json).collect();

            emit_event(OscEvent {
                address: msg.addr,
                args,
            });
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                visit_osc_packet(packet, emit_event);
            }
        }
    }
}

fn handle_packet(packet: rosc::OscPacket, app_handle: &tauri::AppHandle) {
    visit_osc_packet(packet, &mut |event| {
        let _ = app_handle.emit("osc-event", event);
    });
}

#[cfg(test)]
mod tests {
    use super::{osc_argument_to_json, visit_osc_packet, OscEvent};
    use rosc::{OscBundle, OscMessage, OscPacket, OscType};
    use serde_json::json;

    fn message(address: &str, args: Vec<OscType>) -> OscPacket {
        OscPacket::Message(OscMessage {
            addr: address.to_string(),
            args,
        })
    }

    #[test]
    fn converts_supported_osc_arguments_to_json() {
        let arguments = vec![
            OscType::Float(1.25),
            OscType::Double(2.5),
            OscType::Int(-3),
            OscType::Long(4),
            OscType::String("five".to_string()),
            OscType::Bool(true),
            OscType::Nil,
            OscType::Blob(vec![6, 7]),
        ];

        let values: Vec<_> = arguments.into_iter().map(osc_argument_to_json).collect();

        assert_eq!(
            values,
            vec![
                json!(1.25),
                json!(2.5),
                json!(-3),
                json!(4),
                json!("five"),
                json!(true),
                serde_json::Value::Null,
                json!([6, 7]),
            ]
        );
    }

    #[test]
    fn converts_unsupported_osc_arguments_to_null() {
        assert_eq!(
            osc_argument_to_json(OscType::Char('x')),
            serde_json::Value::Null
        );
        assert_eq!(osc_argument_to_json(OscType::Inf), serde_json::Value::Null);
    }

    #[test]
    fn emits_message_address_and_converted_arguments() {
        let packet = message(
            "/test",
            vec![OscType::String("value".to_string()), OscType::Int(12)],
        );
        let mut events = Vec::new();

        visit_osc_packet(packet, &mut |event| events.push(event));

        assert_eq!(
            events,
            vec![OscEvent {
                address: "/test".to_string(),
                args: vec![json!("value"), json!(12)],
            }]
        );
    }

    #[test]
    fn visits_nested_bundle_messages_in_source_order() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: (0, 1).into(),
            content: vec![
                message("/first", vec![]),
                OscPacket::Bundle(OscBundle {
                    timetag: (0, 2).into(),
                    content: vec![
                        message("/second", vec![OscType::Bool(false)]),
                        message("/third", vec![OscType::Nil]),
                    ],
                }),
            ],
        });
        let mut events = Vec::new();

        visit_osc_packet(packet, &mut |event| events.push(event));

        assert_eq!(
            events
                .iter()
                .map(|event| event.address.as_str())
                .collect::<Vec<_>>(),
            vec!["/first", "/second", "/third"]
        );
        assert_eq!(events[1].args, vec![json!(false)]);
        assert_eq!(events[2].args, vec![serde_json::Value::Null]);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn({
                let handle = handle.clone();
                move || {
                    if let Err(e) = setup_midi(handle) {
                        eprintln!("MIDI setup error: {}", e);
                    }
                }
            });
            std::thread::spawn({
                let handle = handle.clone();
                move || {
                    if let Err(e) = setup_osc(handle) {
                        eprintln!("OSC setup error: {}", e);
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

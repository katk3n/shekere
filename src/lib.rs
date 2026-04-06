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

#[derive(Clone, serde::Serialize)]
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
    use std::net::{UdpSocket, Ipv4Addr};

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

fn handle_packet(packet: rosc::OscPacket, app_handle: &tauri::AppHandle) {
    use rosc::{OscPacket, OscType};

    match packet {
        OscPacket::Message(msg) => {
            let args = msg.args.into_iter().map(|arg| match arg {
                OscType::Float(f) => serde_json::json!(f),
                OscType::Double(d) => serde_json::json!(d),
                OscType::Int(i) => serde_json::json!(i),
                OscType::Long(l) => serde_json::json!(l),
                OscType::String(s) => serde_json::json!(s),
                OscType::Bool(b) => serde_json::json!(b),
                OscType::Nil => serde_json::Value::Null,
                OscType::Blob(b) => serde_json::json!(b),
                _ => serde_json::Value::Null,
            }).collect();

            let event = OscEvent {
                address: msg.addr,
                args,
            };
            let _ = app_handle.emit("osc-event", event);
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                handle_packet(packet, app_handle);
            }
        }
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

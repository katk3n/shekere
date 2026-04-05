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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Err(e) = setup_midi(handle) {
                    eprintln!("MIDI setup error: {}", e);
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

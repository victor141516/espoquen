#![windows_subsystem = "windows"]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use enigo::{Enigo, Keyboard, Settings};
use esponquen::{get_hotkey, set_hotkey};
use once_cell::sync::Lazy;
use rdev::{Event, EventType, Key as RdevKey, grab};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu},
};
use winit::event_loop::{ControlFlow, EventLoop};

#[cfg(target_os = "windows")]
fn show_console() {
    unsafe {
        let _ = winapi::um::consoleapi::AllocConsole();
        let _ = winapi::um::wincon::SetConsoleTitleA(b"Esponquen Console\0".as_ptr() as *const i8);
    }
}

#[cfg(not(target_os = "windows"))]
fn show_console() {
    // Console is already visible on non-Windows platforms
}

// Global state for recording
static RECORDING_STATE: Lazy<Arc<Mutex<RecordingState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(RecordingState {
        is_recording: false,
        audio_data: Vec::new(),
        sample_rate: 16000,
    }))
});

// Global state for app status
static APP_STATUS: Lazy<Arc<Mutex<AppStatus>>> =
    Lazy::new(|| Arc::new(Mutex::new(AppStatus::LoadingModel)));

// Global state for provider info (for menu display)
static PROVIDER_INFO: Lazy<Arc<Mutex<String>>> =
    Lazy::new(|| Arc::new(Mutex::new(String::from("Initializing..."))));

struct RecordingState {
    is_recording: bool,
    audio_data: Vec<f32>,
    sample_rate: u32,
}

#[derive(Clone, Debug)]
enum AppStatus {
    LoadingModel,
    WaitingForHotkey,
    Recording,
    Transcribing,
}

impl AppStatus {
    fn to_tooltip(&self) -> String {
        match self {
            AppStatus::LoadingModel => "Esponquen - Loading model...".to_string(),
            AppStatus::WaitingForHotkey => {
                format!("Esponquen - Ready (Press {:?})", get_hotkey())
            }
            AppStatus::Recording => format!(
                "Esponquen - Recording... (Press {:?} to stop)",
                get_hotkey()
            ),
            AppStatus::Transcribing => "Esponquen - Transcribing...".to_string(),
        }
    }
}

fn set_status(status: AppStatus, tray_icon: &TrayIcon) {
    let mut app_status = APP_STATUS.lock().unwrap();
    *app_status = status.clone();
    let tooltip = status.to_tooltip();
    drop(app_status);

    tray_icon.set_tooltip(Some(tooltip)).ok();

    // Set the appropriate icon based on status
    let icon_path = match status {
        AppStatus::LoadingModel => "./icons/loading.ico",
        AppStatus::WaitingForHotkey => "./icons/not-recording.ico",
        AppStatus::Recording => "./icons/recording.ico",
        AppStatus::Transcribing => "./icons/not-recording.ico",
    };

    if let Ok(icon) = Icon::from_path(icon_path, Some((32, 32))) {
        tray_icon.set_icon(Some(icon)).ok();
    }
}

fn main() {
    // Check if --console flag is present
    let args: Vec<String> = std::env::args().collect();
    let show_console_flag = args.iter().any(|arg| arg == "--console");

    if show_console_flag {
        show_console();
    }

    // Only print if console is visible
    if show_console_flag {
        println!("Speech-to-Text Desktop App with Tray Icon");
        println!("==========================================");
        println!();
    }

    // Create event loop for tray icon
    let event_loop = EventLoop::new().unwrap();

    // Create tray icon menu
    let tray_menu = Menu::new();

    // Define available hotkeys (F1-F12)
    let hotkey_options = vec![
        ("F1", RdevKey::F1),
        ("F2", RdevKey::F2),
        ("F3", RdevKey::F3),
        ("F4", RdevKey::F4),
        ("F5", RdevKey::F5),
        ("F6", RdevKey::F6),
        ("F7", RdevKey::F7),
        ("F8", RdevKey::F8),
        ("F9", RdevKey::F9),
        ("F10", RdevKey::F10),
        ("F11", RdevKey::F11),
        ("F12", RdevKey::F12),
    ];

    // Create hotkey submenu and store menu items
    let hotkey_submenu = Submenu::new("Set Hotkey", true);
    let mut hotkey_map: HashMap<MenuId, (String, RdevKey)> = HashMap::new();

    for (name, key) in &hotkey_options {
        let menu_item = MenuItem::new(*name, true, None);
        hotkey_submenu.append(&menu_item).ok();
        hotkey_map.insert(menu_item.id().clone(), (name.to_string(), *key));
    }

    tray_menu.append(&hotkey_submenu).ok();
    tray_menu.append(&PredefinedMenuItem::separator()).ok();

    // Add provider info menu item (disabled, just for display)
    let provider_info = PROVIDER_INFO.lock().unwrap().clone();
    let provider_item = MenuItem::new(format!("Running on: {}", provider_info), false, None);
    tray_menu.append(&provider_item).ok();

    tray_menu.append(&PredefinedMenuItem::separator()).ok();

    let quit_item = MenuItem::new("Quit", true, None);
    tray_menu.append(&quit_item).ok();
    let quit_id = quit_item.id().clone();

    // Load initial icon
    let loading_icon = Icon::from_path("./icons/loading.ico", Some((32, 32)))
        .expect("Failed to load loading.ico icon");

    // Create tray icon
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Esponquen - Loading model...")
        .with_icon(loading_icon)
        .build()
        .expect("Failed to create tray icon");

    // Load the model with GPU acceleration if available
    println!("Loading Parakeet model...");
    set_status(AppStatus::LoadingModel, &tray_icon);

    // Try GPU providers in order of preference
    let providers_to_try = vec![
        #[cfg(target_os = "windows")]
        Some("dml".to_string()), // DirectML - works with any GPU on Windows
        #[cfg(not(target_os = "windows"))]
        Some("cuda".to_string()), // CUDA for NVIDIA GPUs on Linux/Mac
        None, // CPU fallback
    ];

    let mut recognizer = None;
    let mut used_provider = String::from("CPU");

    for provider in providers_to_try {
        println!(
            "Trying provider: {:?}",
            provider.as_ref().unwrap_or(&"CPU".to_string())
        );

        let config = TransducerConfig {
            decoder: "./model/decoder.int8.onnx".to_string(),
            encoder: "./model/encoder.int8.onnx".to_string(),
            joiner: "./model/joiner.int8.onnx".to_string(),
            tokens: "./model/tokens.txt".to_string(),
            num_threads: if provider.is_none() { 4 } else { 1 }, // Use more threads for CPU
            sample_rate: 16_000,
            feature_dim: 80,
            debug: false,
            model_type: "nemo_transducer".to_string(),
            provider: provider.clone(),
            ..Default::default()
        };

        match TransducerRecognizer::new(config) {
            Ok(rec) => {
                used_provider = provider.unwrap_or_else(|| "CPU".to_string());
                println!(
                    "✓ Model loaded successfully with {} provider\n",
                    used_provider
                );
                recognizer = Some(rec);
                break;
            }
            Err(e) => {
                if provider.is_some() {
                    println!(
                        "  ⚠ {} provider not available: {}",
                        provider.as_ref().unwrap(),
                        e
                    );
                    println!("  Trying next provider...\n");
                } else {
                    eprintln!("✗ Failed to initialize recognizer even with CPU: {}", e);
                    eprintln!("\nMake sure the model files exist:");
                    eprintln!("  - ./model/encoder.int8.onnx");
                    eprintln!("  - ./model/decoder.int8.onnx");
                    eprintln!("  - ./model/joiner.int8.onnx");
                    eprintln!("  - ./model/tokens.txt");
                    std::process::exit(1);
                }
            }
        }
    }

    let recognizer = recognizer.unwrap();

    // Store provider info globally for menu display
    {
        let provider_display = if used_provider != "CPU" {
            format!("GPU: {}", used_provider.to_uppercase())
        } else {
            "CPU (4 threads)".to_string()
        };
        let mut provider_info = PROVIDER_INFO.lock().unwrap();
        *provider_info = provider_display;
    }

    if used_provider != "CPU" {
        println!("🚀 GPU acceleration enabled ({})!", used_provider);
        println!("   Transcription should be faster and won't freeze the system.\n");
    } else {
        println!("ℹ️  Running on CPU (no GPU acceleration available)");
        println!("   Transcription may cause brief system slowdowns.\n");
    }

    // Recreate menu with updated provider info
    let updated_menu = Menu::new();

    // Recreate hotkey submenu
    let hotkey_submenu_updated = Submenu::new("Set Hotkey", true);
    let mut hotkey_map_updated: HashMap<MenuId, (String, RdevKey)> = HashMap::new();

    for (name, key) in &hotkey_options {
        let menu_item = MenuItem::new(*name, true, None);
        hotkey_submenu_updated.append(&menu_item).ok();
        hotkey_map_updated.insert(menu_item.id().clone(), (name.to_string(), *key));
    }

    updated_menu.append(&hotkey_submenu_updated).ok();
    updated_menu.append(&PredefinedMenuItem::separator()).ok();

    // Add provider info with actual value
    let provider_info_text = PROVIDER_INFO.lock().unwrap().clone();
    let provider_item_updated =
        MenuItem::new(format!("Running on: {}", provider_info_text), false, None);
    updated_menu.append(&provider_item_updated).ok();

    updated_menu.append(&PredefinedMenuItem::separator()).ok();

    let quit_item_updated = MenuItem::new("Quit", true, None);
    updated_menu.append(&quit_item_updated).ok();
    let quit_id_updated = quit_item_updated.id().clone();

    // Update the tray icon menu
    tray_icon.set_menu(Some(Box::new(updated_menu)));

    // Use updated hotkey_map and quit_id
    let hotkey_map = hotkey_map_updated;
    let quit_id = quit_id_updated;

    set_status(AppStatus::WaitingForHotkey, &tray_icon);

    println!("Instructions:");
    println!("  - Press {:?} to start/stop recording", get_hotkey());
    println!("  - Audio will be recorded from your default microphone");
    println!("  - After stopping, text will be typed automatically");
    println!("  - Right-click tray icon to change hotkey or quit");
    println!("  - Hotkey presses are captured and won't trigger default actions\n");

    // Set up audio recording
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");

    println!(
        "Using input device: {}",
        device.name().unwrap_or_else(|_| "Unknown".to_string())
    );

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");
    let sample_rate = config.sample_rate().0;

    // Update the recording state sample rate
    {
        let mut state = RECORDING_STATE.lock().unwrap();
        state.sample_rate = sample_rate;
    }

    println!("Sample rate: {} Hz\n", sample_rate);
    println!("Ready! Press {:?} to start recording...\n", get_hotkey());

    // Start audio input stream
    let recording_state = Arc::clone(&RECORDING_STATE);
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut state = recording_state.lock().unwrap();
                if state.is_recording {
                    state.audio_data.extend_from_slice(data);
                }
            },
            |err| eprintln!("Stream error: {}", err),
            None,
        ),
        _ => {
            eprintln!("Unsupported sample format");
            std::process::exit(1);
        }
    }
    .expect("Failed to build input stream");

    stream.play().expect("Failed to play stream");

    // Create channel for status updates
    let (status_tx, status_rx): (Sender<AppStatus>, Receiver<AppStatus>) = channel();

    // Listen for keyboard events with grab (blocks default actions)
    let recognizer = Arc::new(Mutex::new(recognizer));
    let recognizer_clone = Arc::clone(&recognizer);

    thread::spawn(move || {
        if let Err(error) =
            grab(move |event: Event| handle_keyboard_event(event, &recognizer_clone, &status_tx))
        {
            eprintln!("Error listening to keyboard events: {:?}", error);
        }
    });

    // Handle menu events
    let menu_channel = MenuEvent::receiver();

    event_loop
        .run(move |_event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            // Check for status updates from keyboard thread
            if let Ok(status) = status_rx.try_recv() {
                set_status(status, &tray_icon);
            }

            // Check for menu events
            if let Ok(event) = menu_channel.try_recv() {
                if event.id == quit_id {
                    println!("\nQuitting...");
                    elwt.exit();
                } else if let Some((name, key)) = hotkey_map.get(&event.id) {
                    set_hotkey(*key);
                    set_status(AppStatus::WaitingForHotkey, &tray_icon);
                    println!("\nHotkey changed to {}", name);
                }
            }
        })
        .ok();
}

fn handle_keyboard_event(
    event: Event,
    recognizer: &Arc<Mutex<TransducerRecognizer>>,
    status_tx: &Sender<AppStatus>,
) -> Option<Event> {
    if let EventType::KeyPress(key) = event.event_type {
        // Check if the pressed key matches the configured hotkey
        let configured_hotkey = get_hotkey();
        if key == configured_hotkey {
            let mut state = RECORDING_STATE.lock().unwrap();

            if state.is_recording {
                // Stop recording
                println!("\n⏹ Recording stopped. Transcribing...");
                state.is_recording = false;
                status_tx.send(AppStatus::Transcribing).ok();

                // Take the audio data
                let audio_data = std::mem::take(&mut state.audio_data);
                let sample_rate = state.sample_rate;
                drop(state); // Release the lock

                if audio_data.is_empty() {
                    println!("✗ No audio recorded");
                    status_tx.send(AppStatus::WaitingForHotkey).ok();
                    return None; // Block the key event
                }

                println!(
                    "  Audio length: {:.2} seconds",
                    audio_data.len() as f32 / sample_rate as f32
                );

                // Transcribe
                let mut rec = recognizer.lock().unwrap();
                let text = rec.transcribe(sample_rate, &audio_data);
                drop(rec);

                println!("✓ Transcription: {}", text);

                if !text.trim().is_empty() {
                    println!("⌨ Typing text...");
                    type_text(&text);
                    println!("✓ Done!\n");
                } else {
                    println!("✗ No text to type\n");
                }

                status_tx.send(AppStatus::WaitingForHotkey).ok();
                println!("Ready! Press {:?} to start recording...", get_hotkey());
            } else {
                // Start recording
                state.audio_data.clear();
                state.is_recording = true;
                status_tx.send(AppStatus::Recording).ok();
                println!("\n🔴 Recording... (Press {:?} to stop)", get_hotkey());
            }

            // Return None to block the key event from propagating
            return None;
        }
    }

    // Return Some(event) to allow the key event to propagate
    Some(event)
}

fn type_text(text: &str) {
    // Small delay to ensure focus is on the right window
    thread::sleep(std::time::Duration::from_millis(100));

    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.text(text).ok();
}

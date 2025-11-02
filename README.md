# Esponquen - Speech-to-Text Desktop App

A cross-platform desktop speech-to-text application for Windows and Linux.

## Features

- 🎙️ **Configurable Hotkey Recording**: Customizable hotkey (default F9) to start/stop audio recording
- 🎤 **Microphone Input**: Records from your default microphone
- 🤖 **AI Transcription**: Uses Parakeet (NeMo Transducer) model for speech recognition
- ⌨️ **Auto-typing**: Automatically types the transcribed text where your cursor is
- 🔧 **API Access**: Programmatic hotkey configuration via library functions
- 📊 **System Tray Icon**: Runs in the background with dynamic status icons, tooltip, and menu

## Prerequisites

### Windows

To build this project on Windows, you need:

#### 1. Install LLVM (for libclang)

Download and install LLVM from: https://releases.llvm.org/

After installation, add LLVM to your PATH:

- Default path: `C:\Program Files\LLVM\bin`
- Or set the `LIBCLANG_PATH` environment variable to point to the LLVM bin directory

#### 2. Install CMake

Download from: https://cmake.org/download/

#### 3. Install Visual Studio Build Tools

Download from: https://visualstudio.microsoft.com/downloads/

- Install "Desktop development with C++" workload

### Linux/Ubuntu

To build this project on Ubuntu/Debian-based systems:

```bash
# Install build essentials and dependencies
sudo apt update
sudo apt install -y build-essential cmake pkg-config
sudo apt install -y libx11-dev libxtst-dev libxdo-dev
sudo apt install -y libasound2-dev
sudo apt install -y libgtk-3-dev
sudo apt install -y llvm clang libclang-dev

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Required system libraries:**

- **X11 and XTest**: For keyboard event handling (`libx11-dev`, `libxtst-dev`)
- **xdotool**: For keyboard simulation (`libxdo-dev`)
- **ALSA**: For audio recording (`libasound2-dev`)
- **GTK3**: For system tray icon (`libgtk-3-dev`)
- **LLVM/Clang**: For sherpa-rs compilation

## Model Setup

Before running the app, you need to download the Parakeet model files:

1. Download a Parakeet/NeMo Transducer model from: https://github.com/k2-fsa/sherpa-onnx/releases
2. Create a `model` directory in the project root
3. Place the following files in `./model/`:
   - `encoder.int8.onnx`
   - `decoder.int8.onnx`
   - `joiner.int8.onnx`
   - `tokens.txt`

## Building the Project

### Development Build

```bash
cargo build --release
```

### Distribution Package

#### Windows

```bash
build-dist.bat
```

#### Linux/Ubuntu

```bash
chmod +x build-dist.sh
./build-dist.sh
```

Both scripts will:

1. Build the release binary
2. Create a `dist/` directory
3. Copy the executable
4. Copy the `model/` directory
5. Copy the `icons/` directory
6. Copy any required files

The complete package will be in the `dist/` folder, ready to distribute or move to another location.

## Running the App

### Windows

```bash
cargo run --release
# Or
target\release\esponquen.exe
```

### Linux

```bash
cargo run --release
# Or
./target/release/esponquen
```

### Debug Mode with Console

To see console output (useful for debugging), use the `--console` flag:

```bash
cargo run --release -- --console
```

Or with the executable:

```bash
target/release/esponquen.exe --console
```

This will show all console messages including model loading status, recording notifications, transcription results, and error messages.

## How to Use

1. **Start the app** - The model will load automatically and a tray icon will appear
2. **Position your cursor** - Click in any text field where you want the transcribed text to appear
3. **Press F9** - Start recording (tray tooltip shows "Recording...")
4. **Speak** - Say what you want to transcribe
5. **Press F9 again** - Stop recording (tray shows "Transcribing...")
6. **Wait** - The text will be automatically typed where your cursor is

### System Tray

The app runs in the system tray with a dynamic icon and status tooltip:

- 🔄 **Loading model...** - Initial startup (loading.png icon)
- ✅ **Ready (Press F9)** - Waiting for hotkey press (not-recording.png icon)
- 🔴 **Recording... (Press F9 to stop)** - Currently recording (recording.png icon)
- ⚙️ **Transcribing...** - Processing audio (not-recording.png icon)

**Icon Files Required:**
Place icon files in the `icons/` directory:

- `loading.png` - Displayed during model loading
- `not-recording.png` - Displayed when ready/transcribing
- `recording.png` - Displayed while recording

**Tray Menu:**

- Right-click the tray icon to access the menu
- **Set Hotkey** submenu: Choose F1-F12 (any function key)
- **Quit**: Exit the application

**Important:** When you press the configured hotkey, the app attempts to capture it and prevent its default action. However, some applications (especially web browsers like Chrome) may still detect the keypress due to their own keyboard event handling. For best results, use F9-F12 which are less commonly intercepted by other applications.

### Tips

- The app runs in the system tray - check your notification area
- Hover over the tray icon to see the current status
- By default, the app runs without a console window (use `--console` flag for debug output)
- Use in any application: text editors, chat apps, browsers, etc.
- Right-click tray icon and select "Quit" to exit properly

### Configuring the Hotkey

You can change the hotkey programmatically using the library API:

```rust
use esponquen::{set_hotkey, get_hotkey};
use rdev::Key;

// Change hotkey to F8
set_hotkey(Key::F8);

// Get current hotkey
let current = get_hotkey();
println!("Current hotkey: {:?}", current);
```

Available keys include: `F1-F12`, `KeyA-KeyZ`, `Num0-Num9`, and many more from the `rdev::Key` enum.

**Note:** The app uses `rdev`'s grab feature to capture hotkeys, which prevents them from triggering their default actions in other applications.

To run the example:

```bash
cargo run --example custom_hotkey
```

## Troubleshooting

### No microphone detected

- Check that a microphone is connected
- Ensure it's set as the default recording device in Windows Sound settings

### Model not loading

- Verify all model files are in `./model/` directory
- Check file names match exactly (case-sensitive)

### Text not typing

- Ensure the target window has focus before transcription completes
- Try clicking in the text field again after stopping recording

## Technical Details

### Libraries Used

- **sherpa-rs**: Speech recognition (Rust bindings for sherpa-onnx)
- **cpal**: Cross-platform audio I/O
- **rdev**: Keyboard event listening (hotkey detection)
- **enigo**: Keyboard simulation (auto-typing)
- **tray-icon**: System tray integration
- **winit**: Event loop for GUI

### Architecture

- Model loads once at startup for fast transcription
- Audio recorded in-memory with cpal
- Hotkey detected via rdev with grab feature (blocks default actions)
- Text output simulated with enigo

### GPU Acceleration

The app automatically detects and uses GPU acceleration when available:

- **Windows**: DirectML (works with any GPU - NVIDIA, AMD, Intel)
- **Linux**:
  - ROCm (AMD GPUs) - **tried first**
  - CUDA (NVIDIA GPUs) - fallback
- **Fallback**: CPU with 4 threads

**Linux AMD GPU Setup (ROCm):**

For AMD GPUs (including APUs like Ryzen with Radeon Graphics):

```bash
# Ubuntu 22.04/24.04 - Install ROCm
wget https://repo.radeon.com/amdgpu-install/latest/ubuntu/jammy/amdgpu-install_6.0.60000-1_all.deb
sudo apt install ./amdgpu-install_6.0.60000-1_all.deb
sudo amdgpu-install --usecase=rocm

# Verify ROCm installation
rocminfo
```

**Linux NVIDIA GPU Setup (CUDA):**

For NVIDIA GPUs:

```bash
# Ubuntu/Debian
sudo apt install nvidia-cuda-toolkit

# Verify CUDA installation
nvcc --version
```

The app will automatically try ROCm first, then CUDA, then fall back to CPU. Check with `--console` flag to see which provider is being used.

**Note:** ROCm works with most modern AMD GPUs including:

- Discrete AMD GPUs (RX 5000/6000/7000 series)
- AMD APUs (Ryzen with integrated Radeon Graphics)

## Resources

- sherpa-rs documentation: https://docs.rs/sherpa-rs/
- sherpa-onnx models: https://github.com/k2-fsa/sherpa-onnx
- sherpa-onnx documentation: https://k2-fsa.github.io/sherpa/onnx/

## License

MIT

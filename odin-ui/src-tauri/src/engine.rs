// Engine child process management.
//
// Spawns odin-engine as a child process, writes commands to stdin,
// reads stdout line-by-line in a background thread, and emits
// Tauri events to the frontend for each line.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use tauri::Emitter;

/// Manages the lifecycle of the engine child process.
pub struct EngineManager {
    child: Option<Child>,
    stdin: Option<BufWriter<std::process::ChildStdin>>,
}

impl EngineManager {
    pub fn new() -> Self {
        Self {
            child: None,
            stdin: None,
        }
    }

    /// Spawn the engine process and start the stdout reader thread.
    pub fn spawn(&mut self, app: tauri::AppHandle) -> Result<(), String> {
        // Kill any existing engine first
        if self.child.is_some() {
            self.kill()?;
        }

        let engine_path = Self::resolve_engine_path()?;

        let mut child = Command::new(&engine_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn engine at {}: {}", engine_path, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture engine stdin".to_string())?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture engine stdout".to_string())?;

        // Spawn a thread to read stdout line-by-line and emit events
        let app_handle = app.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let _ = app_handle.emit("engine-output", &line);
                    }
                    Err(_) => break,
                }
            }
            // Engine stdout closed — process likely exited
            let _ = app_handle.emit("engine-exit", 0);
        });

        self.stdin = Some(BufWriter::new(stdin));
        self.child = Some(child);

        Ok(())
    }

    /// Send a command line to the engine's stdin.
    pub fn send_command(&mut self, cmd: &str) -> Result<(), String> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| "Engine not running".to_string())?;

        writeln!(stdin, "{}", cmd).map_err(|e| format!("Failed to write to engine stdin: {}", e))?;
        stdin
            .flush()
            .map_err(|e| format!("Failed to flush engine stdin: {}", e))?;

        Ok(())
    }

    /// Kill the engine child process.
    pub fn kill(&mut self) -> Result<(), String> {
        // Drop stdin first to signal the engine
        self.stdin = None;

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        Ok(())
    }

    /// Resolve the path to the odin-engine binary.
    fn resolve_engine_path() -> Result<String, String> {
        // Development mode: look for the engine binary relative to the project
        let dev_paths = [
            // Cargo workspace: binary in project root target/
            // From odin-ui/src-tauri/ (where the Tauri binary runs during dev)
            "../../target/debug/odin-engine.exe",
            "../../target/debug/odin-engine",
            // From project root
            "target/debug/odin-engine.exe",
            "target/debug/odin-engine",
            // Per-crate target (non-workspace fallback)
            "../../odin-engine/target/debug/odin-engine.exe",
            "../../odin-engine/target/debug/odin-engine",
            "odin-engine/target/debug/odin-engine.exe",
            "odin-engine/target/debug/odin-engine",
        ];

        for path in &dev_paths {
            let p = std::path::Path::new(path);
            if p.exists() {
                return p
                    .canonicalize()
                    .map(|p| p.to_string_lossy().to_string())
                    .map_err(|e| format!("Failed to resolve engine path: {}", e));
            }
        }

        Err(format!(
            "Could not find odin-engine binary. Searched: {:?}. Run `cargo build` in odin-engine/ first.",
            dev_paths
        ))
    }
}

impl Drop for EngineManager {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

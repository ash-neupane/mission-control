use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::status::StatusDetector;

const PTY_READ_BUF_SIZE: usize = 4096;
const PTY_BATCH_INTERVAL_MS: u64 = 16;

/// Holds all active PTY master handles and their reader threads
pub struct PtyPool {
    pub masters: HashMap<String, Arc<Mutex<Box<dyn MasterPty + Send>>>>,
    pub children: HashMap<String, Box<dyn Child + Send>>,
    pub status_detectors: HashMap<String, Arc<Mutex<StatusDetector>>>,
}

impl Default for PtyPool {
    fn default() -> Self {
        Self::new()
    }
}

impl PtyPool {
    pub fn new() -> Self {
        PtyPool {
            masters: HashMap::new(),
            children: HashMap::new(),
            status_detectors: HashMap::new(),
        }
    }

    /// Spawn a new PTY session
    pub fn spawn(
        &mut self,
        session_id: &str,
        working_dir: &str,
        command: &str,
        args: &[&str],
        app_handle: AppHandle,
    ) -> Result<u32, String> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {e}"))?;

        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(*arg);
        }
        cmd.cwd(working_dir);

        // Inherit environment
        for (key, value) in std::env::vars() {
            cmd.env(key, value);
        }
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn command: {e}"))?;

        let child_pid = child.process_id().unwrap_or(0);

        // Drop the slave - we only need the master
        drop(pair.slave);

        let master = Arc::new(Mutex::new(pair.master));
        self.masters.insert(session_id.to_string(), master.clone());
        // Store the child handle so the process is reaped on drop (no zombie)
        self.children.insert(session_id.to_string(), child);

        // Create status detector for this session
        let detector = Arc::new(Mutex::new(StatusDetector::new(session_id.to_string())));
        self.status_detectors
            .insert(session_id.to_string(), detector.clone());

        // Spawn reader thread
        let sid = session_id.to_string();
        let reader_master = master.clone();

        thread::spawn(move || {
            let mut reader = {
                let master_lock = match reader_master.lock() {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("PTY master lock poisoned for {sid}: {e}");
                        return;
                    }
                };
                match master_lock.try_clone_reader() {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!("Failed to clone PTY reader for {sid}: {e}");
                        return;
                    }
                }
            };

            let mut buf = [0u8; PTY_READ_BUF_SIZE];
            let mut batch_buf: Vec<u8> = Vec::with_capacity(PTY_READ_BUF_SIZE * 2);
            let mut last_flush = std::time::Instant::now();

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // PTY closed — flush remaining
                        if !batch_buf.is_empty() {
                            let _ = app_handle.emit(
                                &format!("pty-output-{sid}"),
                                std::mem::take(&mut batch_buf),
                            );
                        }
                        break;
                    }
                    Ok(n) => {
                        let data = &buf[..n];
                        batch_buf.extend_from_slice(data);

                        // Feed to status detector
                        if let Ok(mut det) = detector.lock() {
                            det.feed(data, &app_handle);
                        }

                        // Batch: flush at frame interval or when buffer is large
                        let elapsed = last_flush.elapsed();
                        if elapsed >= Duration::from_millis(PTY_BATCH_INTERVAL_MS)
                            || batch_buf.len() > PTY_READ_BUF_SIZE
                        {
                            let _ = app_handle.emit(
                                &format!("pty-output-{sid}"),
                                std::mem::take(&mut batch_buf),
                            );
                            last_flush = std::time::Instant::now();
                        }
                    }
                    Err(e) => {
                        log::error!("PTY read error for {sid}: {e}");
                        break;
                    }
                }
            }

            // Notify frontend that the PTY process has exited (BUG-09 fix)
            if let Ok(mut det) = detector.lock() {
                det.mark_done(&app_handle);
            }

            log::info!("PTY reader thread ended for session {sid}");
        });

        Ok(child_pid)
    }

    /// Write data to a PTY
    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<(), String> {
        let master = self
            .masters
            .get(session_id)
            .ok_or_else(|| format!("Session {session_id} not found in PTY pool"))?;

        let master_lock = master
            .lock()
            .map_err(|e| format!("Failed to lock PTY master: {e}"))?;

        let mut writer = master_lock
            .take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {e}"))?;

        writer
            .write_all(data)
            .map_err(|e| format!("Failed to write to PTY: {e}"))?;

        Ok(())
    }

    /// Resize a PTY
    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let master = self
            .masters
            .get(session_id)
            .ok_or_else(|| format!("Session {session_id} not found in PTY pool"))?;

        let master_lock = master
            .lock()
            .map_err(|e| format!("Failed to lock PTY master: {e}"))?;

        master_lock
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to resize PTY: {e}"))?;

        Ok(())
    }

    /// Kill a PTY session. Sends SIGHUP immediately, then spawns a background
    /// thread to SIGKILL after a grace period. The PtyPool lock is NOT held
    /// during the sleep (BUG-02 fix).
    pub fn kill(&mut self, session_id: &str) -> Result<(), String> {
        let pid = self.children.get(session_id)
            .and_then(|c| c.process_id())
            .unwrap_or(0);

        if pid > 0 {
            // Send SIGHUP immediately
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGHUP,
            );

            // Schedule SIGKILL in background — don't block the caller
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(500));
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid as i32),
                    nix::sys::signal::Signal::SIGKILL,
                );
            });
        }

        // Remove entries — child handle is dropped here which reaps the zombie (BUG-01 fix)
        self.masters.remove(session_id);
        self.children.remove(session_id);
        self.status_detectors.remove(session_id);

        Ok(())
    }
}

pub struct PtyPoolState(pub Mutex<PtyPool>);

impl Default for PtyPoolState {
    fn default() -> Self {
        PtyPoolState(Mutex::new(PtyPool::new()))
    }
}

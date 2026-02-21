use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
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
    pub child_pids: HashMap<String, u32>,
    pub status_detectors: HashMap<String, Arc<Mutex<StatusDetector>>>,
}

impl PtyPool {
    pub fn new() -> Self {
        PtyPool {
            masters: HashMap::new(),
            child_pids: HashMap::new(),
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
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

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
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let child_pid = child.process_id().unwrap_or(0);

        // Drop the slave - we only need the master
        drop(pair.slave);

        let master = Arc::new(Mutex::new(pair.master));
        self.masters.insert(session_id.to_string(), master.clone());
        self.child_pids.insert(session_id.to_string(), child_pid);

        // Create status detector for this session
        let detector = Arc::new(Mutex::new(StatusDetector::new(session_id.to_string())));
        self.status_detectors
            .insert(session_id.to_string(), detector.clone());

        // Spawn reader thread
        let sid = session_id.to_string();
        let reader_master = master.clone();

        thread::spawn(move || {
            let mut reader = {
                let master_lock = reader_master.lock().unwrap();
                match master_lock.try_clone_reader() {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!("Failed to clone PTY reader for {}: {}", sid, e);
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
                        // PTY closed
                        if !batch_buf.is_empty() {
                            let _ = app_handle.emit(
                                &format!("pty-output-{}", sid),
                                batch_buf.clone(),
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
                                &format!("pty-output-{}", sid),
                                batch_buf.clone(),
                            );
                            batch_buf.clear();
                            last_flush = std::time::Instant::now();
                        }
                    }
                    Err(e) => {
                        log::error!("PTY read error for {}: {}", sid, e);
                        break;
                    }
                }
            }

            log::info!("PTY reader thread ended for session {}", sid);
        });

        Ok(child_pid)
    }

    /// Write data to a PTY
    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<(), String> {
        let master = self
            .masters
            .get(session_id)
            .ok_or_else(|| format!("Session {} not found in PTY pool", session_id))?;

        let master_lock = master
            .lock()
            .map_err(|e| format!("Failed to lock PTY master: {}", e))?;

        let mut writer = master_lock
            .take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {}", e))?;

        writer
            .write_all(data)
            .map_err(|e| format!("Failed to write to PTY: {}", e))?;

        Ok(())
    }

    /// Resize a PTY
    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let master = self
            .masters
            .get(session_id)
            .ok_or_else(|| format!("Session {} not found in PTY pool", session_id))?;

        let master_lock = master
            .lock()
            .map_err(|e| format!("Failed to lock PTY master: {}", e))?;

        master_lock
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to resize PTY: {}", e))?;

        Ok(())
    }

    /// Kill a PTY session
    pub fn kill(&mut self, session_id: &str) -> Result<(), String> {
        // Try to send SIGHUP first
        if let Some(pid) = self.child_pids.get(session_id) {
            let pid = *pid;
            if pid > 0 {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid as i32),
                    nix::sys::signal::Signal::SIGHUP,
                );

                // Wait briefly for graceful exit
                thread::sleep(Duration::from_millis(500));

                // Force kill if still running
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid as i32),
                    nix::sys::signal::Signal::SIGKILL,
                );
            }
        }

        self.masters.remove(session_id);
        self.child_pids.remove(session_id);
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

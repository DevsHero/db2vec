use std::process::{ Child, Command, Stdio };
use log::{ info, error };
use std::{ error::Error as StdError, io::{ BufRead, BufReader, Write }, time::{ Duration, Instant }, thread };
use crate::{cli::Args, util::spinner::start_operation_animation};
use std::sync::atomic::Ordering;
use std::sync::mpsc; 

pub struct ManagedProcess {
    child: Child,
    name: String,
}

impl ManagedProcess {
    pub fn new(child: Child, name: String) -> Self {
        info!("Started managed process '{}' (PID: {})", name, child.id());
        Self { child, name }
    }


    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn kill(&mut self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        info!("Manually terminating process '{}' (PID: {})", self.name, self.child.id());
        match self.child.kill() {
            Ok(_) => {
                info!("Successfully sent kill signal to process '{}'", self.name);
                Ok(())
            }
            Err(e) => {
                let err = format!("Failed to kill process '{}': {}", self.name, e);
                error!("{}", err);
                Err(err.into())
            }
        }
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        info!("Attempting to terminate managed process '{}' (PID: {})", self.name, self.child.id());
        match self.child.kill() { 
            Ok(_) => {
                info!("Successfully sent kill signal to process '{}'", self.name);
            }
            Err(e) =>
                error!("Failed to kill process '{}' (PID: {}): {}", self.name, self.child.id(), e),
        }
    }
}

pub fn start_and_wait_for_tei(
    args: &Args
) -> Result<(ManagedProcess, String), Box<dyn StdError + Send + Sync>> {

    println!("\n══════════════════════════════════════════════════════════════");
    println!("🚀 Starting local TEI embedding server with model: {}", args.embedding_model);
    println!("   This process can take 3-20 minutes on first run for model download");
    println!("══════════════════════════════════════════════════════════════\n");

    let (animation, counter) = start_operation_animation("Initializing TEI server");
    
    let model_id = if args.embedding_model.is_empty() {
        animation.stop(); 
        return Err("embedding_model must be specified when managing local TEI".into());
    } else {
        &args.embedding_model
    };

    let tei_binary = &args.tei_binary_path;
  

    info!("Starting TEI binary: '{}' with model '{}'", tei_binary, model_id);

    let mut command = Command::new(tei_binary);
    command
        .args(["--model-id", model_id,   "--auto-truncate"])
        .env("RUST_LOG", "info") 
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            animation.stop();
            return Err(format!("Failed to spawn TEI binary '{}': {}", tei_binary, e).into());
        }
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            animation.stop();
            return Err("Failed to capture TEI stdout".into());
        }
    };

    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            animation.stop();
            return Err("Failed to capture TEI stderr".into());
        }
    };

    let process_name = format!("tei-server-{}", child.id());
    let managed_process = ManagedProcess::new(child, process_name);

    println!("\nTEI Server Logs:");
    println!("----------------");

    let (tx, rx) = mpsc::channel();
    let tx_stderr = tx.clone();

    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                if let Err(_) = tx.send(line) {
                    break;
                }
            }
        }
    });

    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                if let Err(_) = tx_stderr.send(line) {
                    break;
                }
            }
        }
    });

    let start_time = Instant::now();
    let mut ready = false;
    let mut log_buffer = Vec::new();
    let timeout = Duration::from_secs(300); 
    let deadline = start_time + timeout;
    
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(line) => {
                log_buffer.push(line.clone());
                println!("  TEI: {}", line);
                
                if line.contains("Starting download") {
                    counter.store(20, Ordering::Relaxed);
                } else if line.contains("Model weights downloaded") {
                    counter.store(40, Ordering::Relaxed);
                } else if line.contains("Starting model backend") {
                    counter.store(60, Ordering::Relaxed);
                } else if line.contains("Warming up model") {
                    counter.store(80, Ordering::Relaxed);
                } else if line.contains("Starting HTTP server") {
                    counter.store(90, Ordering::Relaxed);
                } else if line.contains("Ready") {
                    counter.store(100, Ordering::Relaxed);
                    ready = true;
                    break;
                }
                
                let _ = std::io::stdout().flush();
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                continue;
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                println!("  ⚠️ TEI process may have terminated unexpectedly");
                break;
            }
        }
    }

    animation.stop();

    if ready {
        println!("\n✅ TEI server ready in {:?}! Continuing with processing...\n", start_time.elapsed());
        let tei_url = format!("http://localhost:{}", args.tei_local_port);
        Ok((managed_process, tei_url))
    } else if Instant::now() >= deadline {
        println!("\n❌ Timeout waiting for TEI server to become ready");
        
        if !log_buffer.is_empty() {
            let _ = std::fs::write("tei_timeout.log", log_buffer.join("\n"));
            println!("TEI logs saved to 'tei_timeout.log'");
        }
        
        Err("Timeout waiting for TEI server to become ready".into())
    } else {
        println!("\n❌ TEI server failed to start properly");
        
        if !log_buffer.is_empty() {
            let _ = std::fs::write("tei_failure.log", log_buffer.join("\n"));
            println!("TEI logs saved to 'tei_failure.log'");
        }
        
        Err("TEI server failed to report ready".into())
    }
}

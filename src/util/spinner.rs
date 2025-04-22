use std::io::{ stdout, Write };
use std::sync::{ Arc, Mutex };
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::thread::{ self, JoinHandle };
use std::time::Duration;

pub struct AnimationHandle {
    pub thread: JoinHandle<()>,
    pub stop_flag: Arc<Mutex<bool>>,
}

impl AnimationHandle {
    pub fn stop(self) {
        *self.stop_flag.lock().unwrap() = true;
        if let Err(e) = self.thread.join() {
            eprintln!("Failed to join animation thread: {:?}", e);
        }
    }
}

pub fn start_spinner_animation(
    counter: Arc<AtomicUsize>,
    total: usize,
    message: &str
) -> AnimationHandle {
    let stop_flag = Arc::new(Mutex::new(false));
    let stop_clone = stop_flag.clone();
    let message = message.to_string();

    let thread = thread::spawn(move || {
        let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let mut spinner_idx = 0;

        while !*stop_clone.lock().unwrap() {
            let count = counter.load(Ordering::Relaxed);
            spinner_idx = (spinner_idx + 1) % spinner_chars.len();

            print!(
                "\r{} {}... [{}/{}] ({}%)",
                spinner_chars[spinner_idx],
                message,
                count,
                total,
                (count * 100) / total.max(1)
            );

            let _ = stdout().flush();
            thread::sleep(Duration::from_millis(80));
        }

        print!("\r{}\r", " ".repeat(80));
        let _ = stdout().flush();
    });

    AnimationHandle { thread, stop_flag }
}

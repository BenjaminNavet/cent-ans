//! Off-main-thread decoding of relief pyramid tiles (lot ZG2, ADR 0036).
//!
//! godot-rust bindings are single-threaded: no Godot API may be called from a
//! worker thread. `ReliefDecoder` therefore runs its own native threads that
//! only decode PNG files into plain `Vec<u8>` (little-endian 16-bit samples);
//! the main thread polls finished tiles and converts them to
//! `PackedByteArray`. Rendering support only, no game rule.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use godot::classes::RefCounted;
use godot::prelude::*;

use crate::decode_png_file;

struct Job {
    id: i64,
    path: PathBuf,
}

struct Done {
    id: i64,
    result: Result<(Vec<u8>, u32, u32), String>,
    micros: u64,
}

/// Pool of native threads decoding 16-bit grayscale PNG tiles.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct ReliefDecoder {
    jobs: Option<Sender<Job>>,
    done: Option<Receiver<Done>>,
    workers: Vec<JoinHandle<()>>,
    pending: i64,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for ReliefDecoder {
    fn init(base: Base<RefCounted>) -> Self {
        ReliefDecoder {
            jobs: None,
            done: None,
            workers: Vec::new(),
            pending: 0,
            base,
        }
    }
}

#[godot_api]
impl ReliefDecoder {
    /// Starts `threads` decoding threads (clamped to 1..=16); later calls are
    /// ignored.
    #[func]
    fn start(&mut self, threads: i64) {
        if self.jobs.is_some() {
            return;
        }
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done_rx) = mpsc::channel::<Done>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        for _ in 0..threads.clamp(1, 16) {
            let job_rx = Arc::clone(&job_rx);
            let done_tx = done_tx.clone();
            self.workers
                .push(std::thread::spawn(move || worker_loop(&job_rx, &done_tx)));
        }
        self.jobs = Some(job_tx);
        self.done = Some(done_rx);
    }

    /// Queues the decoding of `path` under the caller's `id`. False if the
    /// pool is not started.
    #[func]
    fn request(&mut self, id: i64, path: GString) -> bool {
        let Some(jobs) = &self.jobs else {
            return false;
        };
        let job = Job {
            id,
            path: PathBuf::from(path.to_string()),
        };
        if jobs.send(job).is_err() {
            return false;
        }
        self.pending += 1;
        true
    }

    /// Up to `max` finished tiles, each `{id, bytes, width, height, ms, error}`
    /// (`bytes` empty and `error` set on failure).
    #[func]
    fn poll(&mut self, max: i64) -> VarArray {
        let mut out = VarArray::new();
        let Some(done) = &self.done else {
            return out;
        };
        while (out.len() as i64) < max {
            let Ok(item) = done.try_recv() else {
                break;
            };
            self.pending -= 1;
            let mut dict = VarDictionary::new();
            dict.set("id", &item.id.to_variant());
            dict.set("ms", &(item.micros as f64 / 1000.0).to_variant());
            match item.result {
                Ok((bytes, width, height)) => {
                    dict.set("bytes", &PackedByteArray::from(bytes).to_variant());
                    dict.set("width", &(width as i64).to_variant());
                    dict.set("height", &(height as i64).to_variant());
                    dict.set("error", &"".to_variant());
                }
                Err(error) => {
                    dict.set("bytes", &PackedByteArray::new().to_variant());
                    dict.set("width", &0i64.to_variant());
                    dict.set("height", &0i64.to_variant());
                    dict.set("error", &error.to_variant());
                }
            }
            out.push(&dict.to_variant());
        }
        out
    }

    /// Requests not yet returned by `poll`.
    #[func]
    fn pending(&self) -> i64 {
        self.pending
    }
}

impl Drop for ReliefDecoder {
    fn drop(&mut self) {
        // Closing the job channel ends every worker loop.
        self.jobs = None;
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn worker_loop(jobs: &Mutex<Receiver<Job>>, done: &Sender<Done>) {
    loop {
        let job = match jobs.lock() {
            Ok(receiver) => receiver.recv(),
            Err(_) => return,
        };
        let Ok(job) = job else {
            return;
        };
        let start = Instant::now();
        let result = decode_png_file(&job.path, png::ColorType::Grayscale, png::BitDepth::Sixteen);
        let item = Done {
            id: job.id,
            result,
            micros: start.elapsed().as_micros() as u64,
        };
        if done.send(item).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_decodes_a_real_tile_off_thread() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../data/map/height/h_8_7.png");
        if !path.exists() {
            return;
        }
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done_rx) = mpsc::channel::<Done>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let worker = std::thread::spawn(move || worker_loop(&job_rx, &done_tx));
        job_tx.send(Job { id: 7, path }).unwrap();
        job_tx
            .send(Job {
                id: 8,
                path: PathBuf::from("/nonexistent/tile.png"),
            })
            .unwrap();
        drop(job_tx);
        let first = done_rx.recv().unwrap();
        let second = done_rx.recv().unwrap();
        worker.join().unwrap();
        assert_eq!(first.id, 7);
        let (bytes, width, height) = first.result.unwrap();
        assert_eq!((width, height), (512, 512));
        assert_eq!(bytes.len(), 512 * 512 * 2);
        assert_eq!(second.id, 8);
        assert!(second.result.is_err());
    }
}

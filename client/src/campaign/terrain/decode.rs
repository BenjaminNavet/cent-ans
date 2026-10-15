//! 16-bit grayscale PNG decoding (relief tiles and the world heightmap) and the worker pool that
//! decodes tiles off the main thread: port of `core/crates/godot-bridge/src/relief_decoder.rs`
//! (the bridge is not a dependency of the client). Rendering support only.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Decoded grayscale 16-bit image: row-major samples.
pub struct Gray16 {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u16>,
}

/// Decodes a 16-bit grayscale PNG.
pub fn decode_gray16(path: &Path) -> Result<Gray16, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let info = reader.info();
    if info.color_type != png::ColorType::Grayscale || info.bit_depth != png::BitDepth::Sixteen {
        return Err(format!("{}: not a 16-bit grayscale PNG", path.display()));
    }
    let (width, height) = (info.width as usize, info.height as usize);
    let mut bytes = vec![0u8; reader.output_buffer_size().unwrap_or(width * height * 2)];
    reader
        .next_frame(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let data = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_be_bytes(*b))
        .collect();
    Ok(Gray16 {
        width,
        height,
        data,
    })
}

struct Job {
    key: i64,
    path: PathBuf,
}

/// A finished tile: `key` is the caller's id.
pub struct Decoded {
    pub key: i64,
    pub result: Result<Gray16, String>,
}

/// Pool of native threads decoding relief tiles.
pub struct TileDecoder {
    jobs: Option<Sender<Job>>,
    done: Mutex<Receiver<Decoded>>,
    workers: Vec<JoinHandle<()>>,
}

impl TileDecoder {
    pub fn start(threads: usize) -> TileDecoder {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done_rx) = mpsc::channel::<Decoded>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let workers = (0..threads.clamp(1, 16))
            .map(|_| {
                let job_rx = Arc::clone(&job_rx);
                let done_tx = done_tx.clone();
                std::thread::spawn(move || worker_loop(&job_rx, &done_tx))
            })
            .collect();
        TileDecoder {
            jobs: Some(job_tx),
            done: Mutex::new(done_rx),
            workers,
        }
    }

    /// Queues the decoding of `path` under `key`.
    pub fn request(&self, key: i64, path: PathBuf) -> bool {
        self.jobs
            .as_ref()
            .is_some_and(|jobs| jobs.send(Job { key, path }).is_ok())
    }

    /// Up to `max` finished tiles.
    pub fn poll(&self, max: usize) -> Vec<Decoded> {
        let mut out = Vec::new();
        if let Ok(done) = self.done.lock() {
            while out.len() < max {
                match done.try_recv() {
                    Ok(item) => out.push(item),
                    Err(_) => break,
                }
            }
        }
        out
    }
}

impl Drop for TileDecoder {
    fn drop(&mut self) {
        self.jobs = None;
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn worker_loop(jobs: &Mutex<Receiver<Job>>, done: &Sender<Decoded>) {
    loop {
        let job = match jobs.lock() {
            Ok(receiver) => receiver.recv(),
            Err(_) => return,
        };
        let Ok(job) = job else {
            return;
        };
        let item = Decoded {
            key: job.key,
            result: decode_gray16(&job.path),
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
    fn worker_decodes_a_real_tile_and_reports_errors() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let path = root.join("data/map/height/h_8_7.png");
        if !path.exists() {
            return;
        }
        let decoder = TileDecoder::start(2);
        assert!(decoder.request(7, path));
        assert!(decoder.request(8, PathBuf::from("/nonexistent/tile.png")));
        let mut got = Vec::new();
        for _ in 0..2000 {
            got.extend(decoder.poll(8));
            if got.len() == 2 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(got.len(), 2);
        let ok = got
            .iter()
            .find(|d| d.key == 7)
            .unwrap()
            .result
            .as_ref()
            .unwrap();
        assert_eq!((ok.width, ok.height, ok.data.len()), (512, 512, 512 * 512));
        assert!(got.iter().find(|d| d.key == 8).unwrap().result.is_err());
    }

    #[test]
    fn rejects_non_16_bit_files() {
        let dir = std::env::temp_dir().join("cent_ans_decode_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("eight.png");
        {
            let file = File::create(&path).unwrap();
            let mut enc = png::Encoder::new(file, 2, 2);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()
                .unwrap()
                .write_image_data(&[0, 1, 2, 3])
                .unwrap();
        }
        assert!(decode_gray16(&path).is_err());
        let path16 = dir.join("sixteen.png");
        {
            let file = File::create(&path16).unwrap();
            let mut enc = png::Encoder::new(file, 2, 1);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::Sixteen);
            enc.write_header()
                .unwrap()
                .write_image_data(&[0x01, 0x02, 0xff, 0xfe])
                .unwrap();
        }
        let image = decode_gray16(&path16).unwrap();
        assert_eq!(image.data, vec![0x0102, 0xfffe]);
    }
}

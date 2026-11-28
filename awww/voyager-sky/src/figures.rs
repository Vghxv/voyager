//! Constellation figures, decoded and mip-mapped off the render thread, with the last
//! few kept: drag mode asks for the one nearest the cursor every frame, and decoding a
//! 1024 px PNG there (tens of ms) would drop frames each time it changes.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::thread;

use crate::render::{mip_chain, Mips};

/// Decoded figures kept for going back and forth between neighbours (~5.6 MB each).
const KEEP: usize = 8;

struct Worker {
    requests: mpsc::Sender<usize>,
    results: mpsc::Receiver<(usize, Result<Mips, String>)>,
}

pub struct Figures {
    paths: Arc<Vec<PathBuf>>,
    /// None: load on the calling thread (stills and tests, so they come out the same every time)
    worker: Option<Worker>,
    cache: VecDeque<(usize, Arc<Mips>)>,
    pending: Option<usize>,
    /// a figure that would not load, so it is not tried every frame
    failed: Option<usize>,
}

fn load(path: &PathBuf) -> Result<Mips, String> {
    Ok(mip_chain(&crate::data::png(path)?))
}

impl Figures {
    /// `paths[i]` is constellation i's figure image. Loads on the calling thread.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Figures {
            paths: Arc::new(paths),
            worker: None,
            cache: VecDeque::new(),
            pending: None,
            failed: None,
        }
    }

    /// From now on, load on a thread of its own.
    pub fn in_background(&mut self) {
        let (requests, todo) = mpsc::channel::<usize>();
        let (done, results) = mpsc::channel();
        let paths = Arc::clone(&self.paths);
        thread::spawn(move || {
            while let Ok(mut id) = todo.recv() {
                while let Ok(newer) = todo.try_recv() {
                    id = newer; // the cursor moved on: only the newest one matters
                }
                if done.send((id, load(&paths[id]))).is_err() {
                    return;
                }
            }
        });
        self.worker = Some(Worker { requests, results });
    }

    /// Whether a figure is on its way (keep frames coming until it arrives).
    pub fn loading(&self) -> bool {
        self.pending.is_some()
    }

    /// Figure `id` if it is ready; otherwise start loading it and return None.
    pub fn get(&mut self, id: usize) -> Option<Arc<Mips>> {
        self.collect();
        if let Some(k) = self.cache.iter().position(|(i, _)| *i == id) {
            let hit = self.cache.remove(k).expect("in range");
            self.cache.push_front(hit.clone());
            return Some(hit.1);
        }
        if self.failed == Some(id) {
            return None;
        }
        match &self.worker {
            None => {
                let loaded = load(&self.paths[id]);
                self.store(id, loaded);
                self.cache
                    .front()
                    .filter(|(i, _)| *i == id)
                    .map(|(_, m)| m.clone())
            }
            Some(w) => {
                if self.pending != Some(id) && w.requests.send(id).is_ok() {
                    self.pending = Some(id);
                }
                None
            }
        }
    }

    fn collect(&mut self) {
        let Some(w) = &self.worker else { return };
        let done: Vec<_> = w.results.try_iter().collect();
        for (id, loaded) in done {
            if self.pending == Some(id) {
                self.pending = None;
            }
            self.store(id, loaded);
        }
    }

    fn store(&mut self, id: usize, loaded: Result<Mips, String>) {
        match loaded {
            Ok(m) => {
                self.cache.push_front((id, Arc::new(m)));
                self.cache.truncate(KEEP);
            }
            Err(e) => {
                eprintln!("voyager-sky: {e}");
                self.failed = Some(id);
            }
        }
    }
}

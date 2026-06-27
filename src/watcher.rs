use std::{collections::HashMap, os::fd::{AsFd, AsRawFd, FromRawFd}, sync::{Arc, Mutex, mpsc}};

use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify, WatchDescriptor};

use crate::model::Message;


pub struct Watcher {
    tx: mpsc::Sender<Message>,
    inotify: Arc<Inotify>,
    watches: Arc<Mutex<HashMap<WatchDescriptor, String>>>,
}

pub struct WatcherController {
    inotify: Arc<Inotify>,
    watches: Arc<Mutex<HashMap<WatchDescriptor, String>>>,
}

impl WatcherController {
    pub fn add(&mut self, path: String) {
        match self.inotify.add_watch(path.as_str(), AddWatchFlags::IN_MODIFY | AddWatchFlags::IN_DELETE) {
            Ok(wd) => { self.watches.lock().unwrap().insert(wd, path); },
            Err(e) => { tracing::error!("failed adding watch {path} {e:?}"); },
        }
    }
    pub fn remove(&mut self, path: String) {
        match self.watches.lock().unwrap().iter().find(|(_wd,p)| p == &&path) {
            Some((wd, _p)) => if let Err(e) = self.inotify.rm_watch(*wd) {
                tracing::error!("failed removing watch {path} {e:?}");
            },
            None => {},
        }
    }
}

impl Watcher {
    pub fn start(txm: mpsc::Sender<Message>) -> anyhow::Result<WatcherController> {
        let inotify = Arc::new(Inotify::init(InitFlags::empty())?);
        let watches = Arc::new(Mutex::new(HashMap::new()));
        let inotify_clone = inotify.clone();
        let watches_clone = watches.clone();
        std::thread::spawn(move || Self {
            tx: txm,
            inotify: inotify_clone,
            watches: watches_clone,
        }.thread());
        Ok(WatcherController { inotify, watches })
    }
    pub fn thread(self) {
        loop {
            for event in self.inotify.read_events().unwrap() {
                if let Some(path) = self.watches.lock().unwrap().get(&event.wd) {
                    tracing::debug!("inotify event {:?} on {path}", event.mask);
                    let _ = self.tx.send(Message::FileChangedOnDisk(path.clone()));
                }
            }
        }
    }
}

use std::fmt;
use std::path::{Path, PathBuf};
use std::thread::{current, JoinHandle, ThreadId};

use erg_common::dict::Dict;
use erg_common::shared::Shared;

use super::SharedModuleGraph;

#[derive(Debug)]
pub enum Promise {
    Running {
        parent: ThreadId,
        handle: JoinHandle<()>,
    },
    Joining,
    Finished,
}

impl fmt::Display for Promise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running { handle, .. } => {
                write!(f, "running on thread {:?}", handle.thread().id())
            }
            Self::Joining => write!(f, "joining"),
            Self::Finished => write!(f, "finished"),
        }
    }
}

impl Promise {
    pub fn running(handle: JoinHandle<()>) -> Self {
        Self::Running {
            parent: current().id(),
            handle,
        }
    }

    pub fn is_finished(&self) -> bool {
        match self {
            Self::Finished => true,
            Self::Joining => false,
            Self::Running { handle, .. } => handle.is_finished(),
        }
    }

    pub fn thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Finished | Self::Joining => None,
            Self::Running { handle, .. } => Some(handle.thread().id()),
        }
    }

    pub fn parent_thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Finished | Self::Joining => None,
            Self::Running { parent, .. } => Some(*parent),
        }
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Self::Joining)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedPromises {
    graph: SharedModuleGraph,
    pub(crate) path: PathBuf,
    promises: Shared<Dict<PathBuf, Promise>>,
}

impl fmt::Display for SharedPromises {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SharedPromises {{ ")?;
        for (path, promise) in self.promises.borrow().iter() {
            writeln!(f, "{}: {}, ", path.display(), promise)?;
        }
        write!(f, "}}")
    }
}

impl SharedPromises {
    pub fn new(graph: SharedModuleGraph, path: PathBuf) -> Self {
        Self {
            graph,
            path,
            promises: Shared::new(Dict::new()),
        }
    }

    pub fn insert(&self, path: PathBuf, handle: JoinHandle<()>) {
        if self.promises.borrow().get(&path).is_some() {
            // panic!("already registered: {}", path.display());
            return;
        }
        self.promises
            .borrow_mut()
            .insert(path, Promise::running(handle));
    }

    pub fn is_registered(&self, path: &Path) -> bool {
        self.promises.borrow().get(path).is_some()
    }

    pub fn is_finished(&self, path: &Path) -> bool {
        self.promises
            .borrow()
            .get(path)
            .is_some_and(|promise| promise.is_finished())
    }

    fn join_checked(&self, path: &Path, promise: Promise) -> std::thread::Result<()> {
        let Promise::Running{ handle, parent } = promise else {
            return Ok(());
        };
        if self.graph.ancestors(path).contains(&self.path) || handle.thread().id() == current().id()
        {
            // cycle detected, `self.path` must not in the dependencies
            // Erg analysis processes never join ancestor threads (although joining ancestors itself is allowed in Rust)
            *self.promises.borrow_mut().get_mut(path).unwrap() =
                Promise::Running { parent, handle };
            return Ok(());
        }
        let res = handle.join();
        *self.promises.borrow_mut().get_mut(path).unwrap() = Promise::Finished;
        res
    }

    pub fn join(&self, path: &Path) -> std::thread::Result<()> {
        while let Some(Promise::Joining) | None = self.promises.borrow().get(path) {
            std::thread::yield_now();
        }
        let promise = self.promises.borrow_mut().get_mut(path).unwrap().take();
        self.join_checked(path, promise)
    }

    pub fn join_children(&self) {
        let cur_id = std::thread::current().id();
        let mut promises = vec![];
        for (path, promise) in self.promises.borrow_mut().iter_mut() {
            if promise.parent_thread_id() != Some(cur_id) {
                continue;
            }
            promises.push((path.clone(), promise.take()));
        }
        for (path, promise) in promises {
            let _result = self.join_checked(&path, promise);
        }
    }

    pub fn join_all(&self) {
        let mut promises = vec![];
        for (path, promise) in self.promises.borrow_mut().iter_mut() {
            promises.push((path.clone(), promise.take()));
        }
        for (path, promise) in promises {
            let _result = self.join_checked(&path, promise);
        }
    }
}

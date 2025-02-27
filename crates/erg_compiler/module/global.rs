use std::path::{Path, PathBuf};

use erg_common::config::ErgConfig;

use crate::context::Context;

use super::cache::SharedModuleCache;
use super::errors::{SharedCompileErrors, SharedCompileWarnings};
use super::graph::SharedModuleGraph;
use super::impls::SharedTraitImpls;
use super::index::SharedModuleIndex;
use super::promise::SharedPromises;

#[derive(Debug, Clone, Default)]
pub struct SharedCompilerResource {
    pub mod_cache: SharedModuleCache,
    pub py_mod_cache: SharedModuleCache,
    pub index: SharedModuleIndex,
    pub graph: SharedModuleGraph,
    /// K: name of a trait, V: (type, monomorphised trait that the type implements)
    /// K: トレイトの名前, V: (型, その型が実装する単相化トレイト)
    /// e.g. { "Named": [(Type, Named), (Func, Named), ...], "Add": [(Nat, Add(Nat)), (Int, Add(Int)), ...], ... }
    pub trait_impls: SharedTraitImpls,
    pub promises: SharedPromises,
    pub errors: SharedCompileErrors,
    pub warns: SharedCompileWarnings,
}

impl SharedCompilerResource {
    /// Initialize the shared compiler resource.
    /// This API is normally called only once throughout the compilation phase.
    pub fn new(cfg: ErgConfig) -> Self {
        let graph = SharedModuleGraph::new();
        let self_ = Self {
            mod_cache: SharedModuleCache::new(),
            py_mod_cache: SharedModuleCache::new(),
            index: SharedModuleIndex::new(),
            graph: graph.clone(),
            trait_impls: SharedTraitImpls::new(),
            promises: SharedPromises::new(
                graph,
                cfg.input
                    .path()
                    .map_or(PathBuf::default(), |p| p.canonicalize().unwrap_or_default()),
            ),
            errors: SharedCompileErrors::new(),
            warns: SharedCompileWarnings::new(),
        };
        Context::init_builtins(cfg, self_.clone());
        self_
    }

    pub fn inherit(&self, path: PathBuf) -> Self {
        let mut _self = self.clone();
        _self.promises.path = path;
        _self
    }

    pub fn clear_all(&self) {
        self.mod_cache.initialize();
        self.py_mod_cache.initialize();
        self.index.initialize();
        self.graph.initialize();
        self.trait_impls.initialize();
        self.errors.clear();
        self.warns.clear();
    }

    pub fn clear(&self, path: &Path) {
        self.mod_cache.remove(path);
        self.py_mod_cache.remove(path);
        self.index.remove_path(path);
        self.graph.remove(path);
    }
}

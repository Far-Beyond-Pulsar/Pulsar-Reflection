//! Language-agnostic trait descriptors.
//!
//! A *trait* here is a contract authored as a project asset (`*.trait.json`)
//! or shipped by the engine (the role traits `GameMode`, `PlayerController`,
//! ...). It is not a Rust `trait`: any language a plugin brings (Blueprints,
//! a scripting language, a native crate) can declare that an asset implements
//! it and can ask who does. This module holds only the *published result* so
//! every plugin reads one answer; the host (`engine_fs::TraitDb`) owns how
//! it is derived from project files.
//!
//! Types are plain strings and every value is `serde`-serializable, so a
//! non-Rust plugin can read a [`TraitSnapshot`] as JSON
//! ([`TraitSnapshot::to_json`]) instead of linking these types.
//!
//! The registry is part of the shared [`runtime`](crate::runtime) like every
//! other registry here: a plugin's copy of this crate reads the editor's.
//! Contents are scoped per project (`scope` is the project root as the host
//! spells it), because an editor can have several projects open.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

/// One parameter of a trait method. `type_name` is the type as the trait
/// asset spells it (`f32`, `Vec<String>`, `my_crate::Thing`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitParamDescriptor {
    pub name: String,
    pub type_name: String,
}

/// One method a trait requires of its implementers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitMethodDescriptor {
    pub name: String,
    pub params: Vec<TraitParamDescriptor>,
    /// `()` for a method that returns nothing.
    pub return_type: String,
    pub doc: Option<String>,
    /// An implementer may omit a method that has a default body.
    pub has_default: bool,
}

/// A published trait.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitDescriptor {
    /// Stable identity. Survives renames and moves of the asset; `path` does
    /// not.
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    /// Current project-relative asset path, when the trait is a file.
    pub path: Option<String>,
    /// Engine-owned: its method contract is fixed by `contract_version`.
    pub builtin: bool,
    pub contract_version: u32,
    /// Ids of the traits this one extends.
    pub supertraits: Vec<String>,
    pub methods: Vec<TraitMethodDescriptor>,
}

/// An asset that implements a trait.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitImplementer {
    /// Project-relative path of the implementing asset.
    pub asset: String,
    /// What kind of asset: `blueprint`, or a plugin's own kind.
    pub kind: String,
    /// Set when the asset implements a trait that extends this one, and
    /// names that trait's id.
    pub via: Option<String>,
}

/// An immutable view of one project's traits: what [`TraitRegistry`] hands
/// out. All lookups take a trait *reference*: an id, a project-relative
/// path, or a name that is unique among the project's traits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitSnapshot {
    traits: BTreeMap<String, TraitDescriptor>,
    /// Trait id -> implementers, effective (inherited ones included).
    implementers: BTreeMap<String, Vec<TraitImplementer>>,
    /// Asset path -> ids of the traits it implements, effective.
    by_asset: BTreeMap<String, Vec<String>>,
}

impl TraitSnapshot {
    /// A snapshot of `traits` and the `(trait id, implementer)` pairs that
    /// hold between them and assets. Pairs naming an unknown trait are
    /// dropped.
    pub fn new(
        traits: Vec<TraitDescriptor>,
        implementations: Vec<(String, TraitImplementer)>,
    ) -> Self {
        let traits: BTreeMap<_, _> = traits.into_iter().map(|t| (t.id.clone(), t)).collect();
        let mut implementers: BTreeMap<String, Vec<TraitImplementer>> = BTreeMap::new();
        let mut by_asset: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (trait_id, implementer) in implementations {
            if !traits.contains_key(&trait_id) {
                continue;
            }
            let ids = by_asset.entry(implementer.asset.clone()).or_default();
            if !ids.contains(&trait_id) {
                ids.push(trait_id.clone());
            }
            let list = implementers.entry(trait_id).or_default();
            if !list.contains(&implementer) {
                list.push(implementer);
            }
        }
        for list in implementers.values_mut() {
            list.sort_by(|a, b| a.asset.cmp(&b.asset).then(a.via.cmp(&b.via)));
        }
        for ids in by_asset.values_mut() {
            ids.sort();
        }
        Self {
            traits,
            implementers,
            by_asset,
        }
    }

    /// Every trait, by id.
    pub fn traits(&self) -> impl Iterator<Item = &TraitDescriptor> {
        self.traits.values()
    }

    /// The trait `reference` names: an id, else a path, else a unique name.
    pub fn get(&self, reference: &str) -> Option<&TraitDescriptor> {
        if let Some(found) = self.traits.get(reference) {
            return Some(found);
        }
        let normalized = reference.replace('\\', "/");
        if let Some(found) = self
            .traits
            .values()
            .find(|t| t.path.as_deref() == Some(normalized.as_str()))
        {
            return Some(found);
        }
        let mut named = self.traits.values().filter(|t| t.name == reference);
        let first = named.next()?;
        named.next().is_none().then_some(first)
    }

    /// Assets implementing the trait `reference` names, directly or through
    /// a trait that extends it.
    pub fn implementers(&self, reference: &str) -> &[TraitImplementer] {
        self.get(reference)
            .and_then(|t| self.implementers.get(&t.id))
            .map_or(&[], Vec::as_slice)
    }

    /// Traits the asset at `asset` implements, directly or inherited.
    pub fn traits_of(&self, asset: &str) -> Vec<&TraitDescriptor> {
        let asset = asset.replace('\\', "/");
        self.by_asset
            .get(&asset)
            .into_iter()
            .flatten()
            .filter_map(|id| self.traits.get(id))
            .collect()
    }

    /// Whether the asset implements the trait.
    pub fn implements(&self, asset: &str, reference: &str) -> bool {
        let Some(found) = self.get(reference) else {
            return false;
        };
        let asset = asset.replace('\\', "/");
        self.by_asset
            .get(&asset)
            .is_some_and(|ids| ids.contains(&found.id))
    }

    /// This snapshot as a JSON value, for a plugin that is not Rust.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Published trait snapshots, one per project.
#[derive(Default)]
pub struct TraitRegistry {
    scopes: RwLock<BTreeMap<String, Arc<TraitSnapshot>>>,
}

impl TraitRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the project's snapshot. Readers holding the old one keep it.
    pub fn publish(&self, scope: &str, snapshot: TraitSnapshot) {
        let mut scopes = self.scopes.write().unwrap_or_else(|e| e.into_inner());
        scopes.insert(scope.to_owned(), Arc::new(snapshot));
    }

    /// The project's current snapshot, if the host has published one.
    pub fn snapshot(&self, scope: &str) -> Option<Arc<TraitSnapshot>> {
        let scopes = self.scopes.read().unwrap_or_else(|e| e.into_inner());
        scopes.get(scope).cloned()
    }

    /// Forget a project (it closed).
    pub fn remove(&self, scope: &str) -> bool {
        let mut scopes = self.scopes.write().unwrap_or_else(|e| e.into_inner());
        scopes.remove(scope).is_some()
    }

    /// The projects with a published snapshot.
    pub fn scopes(&self) -> Vec<String> {
        let scopes = self.scopes.read().unwrap_or_else(|e| e.into_inner());
        scopes.keys().cloned().collect()
    }
}

/// The key a project's snapshot is published under: its root with forward
/// slashes and no trailing slash. The host and plugins must agree on it, so
/// both spell it with this.
pub fn trait_scope_key(project_root: &std::path::Path) -> String {
    project_root
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned()
}

/// The process-wide trait registry. Shared by every linked copy of this
/// crate (see [`crate::runtime`]).
pub static TRAIT_REGISTRY: crate::runtime::Shared<TraitRegistry> =
    crate::runtime::Shared::new(|runtime| (runtime.traits)());

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(id: &str, name: &str, path: &str) -> TraitDescriptor {
        TraitDescriptor {
            id: id.into(),
            name: name.into(),
            display_name: name.into(),
            description: None,
            path: Some(path.into()),
            builtin: false,
            contract_version: 0,
            supertraits: Vec::new(),
            methods: Vec::new(),
        }
    }

    fn implementer(asset: &str, via: Option<&str>) -> TraitImplementer {
        TraitImplementer {
            asset: asset.into(),
            kind: "blueprint".into(),
            via: via.map(str::to_owned),
        }
    }

    #[test]
    fn references_resolve_by_id_path_and_unique_name() {
        let snapshot = TraitSnapshot::new(
            vec![
                descriptor("a", "Movable", "types/traits/movable.trait.json"),
                descriptor("b", "Dup", "types/traits/dup1.trait.json"),
                descriptor("c", "Dup", "types/traits/dup2.trait.json"),
            ],
            Vec::new(),
        );
        assert_eq!(snapshot.get("a").unwrap().name, "Movable");
        assert_eq!(
            snapshot.get("types\\traits\\movable.trait.json").unwrap().id,
            "a"
        );
        assert_eq!(snapshot.get("Movable").unwrap().id, "a");
        assert!(snapshot.get("Dup").is_none(), "ambiguous names do not resolve");
        assert!(snapshot.get("missing").is_none());
    }

    #[test]
    fn implementers_are_sorted_deduplicated_and_reverse_indexed() {
        let snapshot = TraitSnapshot::new(
            vec![descriptor("a", "Movable", "m.trait.json")],
            vec![
                ("a".into(), implementer("z/graph_save.json", None)),
                ("a".into(), implementer("b/graph_save.json", None)),
                ("a".into(), implementer("b/graph_save.json", None)),
                ("gone".into(), implementer("q/graph_save.json", None)),
            ],
        );
        let assets: Vec<_> = snapshot
            .implementers("Movable")
            .iter()
            .map(|i| i.asset.as_str())
            .collect();
        assert_eq!(assets, ["b/graph_save.json", "z/graph_save.json"]);
        assert_eq!(snapshot.traits_of("b/graph_save.json")[0].id, "a");
        assert!(snapshot.implements("b\\graph_save.json", "a"));
        assert!(snapshot.traits_of("q/graph_save.json").is_empty());
    }

    #[test]
    fn registry_scopes_are_independent() {
        let registry = TraitRegistry::new();
        registry.publish(
            "p1",
            TraitSnapshot::new(vec![descriptor("a", "A", "a.trait.json")], Vec::new()),
        );
        assert!(registry.snapshot("p2").is_none());
        let held = registry.snapshot("p1").unwrap();
        registry.publish("p1", TraitSnapshot::default());
        assert_eq!(held.traits().count(), 1, "readers keep the snapshot they took");
        assert_eq!(registry.snapshot("p1").unwrap().traits().count(), 0);
        assert!(registry.remove("p1"));
        assert!(registry.scopes().is_empty());
    }

    #[test]
    fn snapshots_round_trip_as_json() {
        let snapshot = TraitSnapshot::new(
            vec![descriptor("a", "A", "a.trait.json")],
            vec![("a".into(), implementer("x/graph_save.json", None))],
        );
        let back: TraitSnapshot = serde_json::from_value(snapshot.to_json()).unwrap();
        assert_eq!(back, snapshot);
    }
}

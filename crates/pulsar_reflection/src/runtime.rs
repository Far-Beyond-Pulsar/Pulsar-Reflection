//! Process-wide state shared by every statically linked copy of this crate
//! (Pulsar-Native#1083).
//!
//! The editor links this crate statically, and so does every plugin library
//! it loads, so every registry here would exist once per binary: a plugin's
//! classes, methods, conversions and property editors would land in tables
//! the editor never reads. Instead, one copy (the editor's) owns the
//! registries, and every other copy is [`attach`]ed to the owner's
//! [`Runtime`] when its library loads: from then on the public registries
//! ([`REGISTRY`](crate::REGISTRY), [`METHOD_REGISTRY`](crate::methods::METHOD_REGISTRY),
//! ...) dereference to the owner's, and the owner's registries include
//! what this copy's `inventory` collected.
//!
//! The model is WGPUI's shared runtime for gpui: every copy is built by the
//! same compiler from the same sources, which [`Runtime::abi`] checks. A
//! `TypeId` differs between copies built by different cargo invocations, so
//! a plugin's own types are found under the plugin's `TypeId`s; engine
//! types are best reached by name.

use std::any::TypeId;
use std::fmt;
use std::ops::Deref;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex};

use crate::conversions::{ConversionRegistration, ConversionRegistry};
use crate::dyn_registry::{DynMethodRegistration, DynMethodRegistry};
use crate::dynamic_types::DynamicTypeRegistry;
use crate::methods::{MethodRegistry, TypeMethodRegistration};
use crate::registry::{ComponentMethodRegistration, EngineClassRegistration, EngineClassRegistry};
use crate::runtime_registry::{RuntimeTypeRegistration, RuntimeTypeRegistry};
use crate::type_renderer::{TypeRendererRegistration, TypeRendererRegistry};
use crate::{
    EnumVariantDocs, RuntimeBehaviorRegistration, ScenePropsApplierRegistration,
    UiPropertyEditorHint,
};

/// Version of [`Runtime`]'s ABI. Bump it on any change to `Runtime`'s fields
/// or to a type passed through them.
pub const ABI_VERSION: u64 = 1;

const FINGERPRINT: u64 = {
    let parts = [
        ABI_VERSION as usize,
        size_of::<Runtime>(),
        size_of::<Registrations>(),
        size_of::<EngineClassRegistry>(),
        size_of::<MethodRegistry>(),
        size_of::<ConversionRegistry>(),
        size_of::<RuntimeTypeRegistry>(),
        size_of::<DynMethodRegistry>(),
        size_of::<DynamicTypeRegistry>(),
        size_of::<Mutex<TypeRendererRegistry>>(),
        size_of::<TypeId>(),
    ];
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut index = 0;
    while index < parts.len() {
        hash ^= parts[index] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }
    hash
};

/// The process-wide registries, as the functions of the copy that owns
/// them. See the module doc.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Runtime {
    /// [`ABI_VERSION`] and the layout fingerprint of the copy that built
    /// this runtime; [`attach`] refuses a runtime whose `abi` differs.
    pub abi: u64,
    pub(crate) engine_classes: fn() -> &'static EngineClassRegistry,
    pub(crate) methods: fn() -> &'static MethodRegistry,
    pub(crate) conversions: fn() -> &'static ConversionRegistry,
    pub(crate) runtime_types: fn() -> &'static RuntimeTypeRegistry,
    pub(crate) dyn_methods: fn() -> &'static DynMethodRegistry,
    pub(crate) dynamic_types: fn() -> &'static DynamicTypeRegistry,
    pub(crate) type_renderers: fn() -> &'static Mutex<TypeRendererRegistry>,
    pub(crate) component_method_registrations: fn() -> &'static [&'static ComponentMethodRegistration],
    pub(crate) dyn_method_registrations: fn() -> &'static [&'static DynMethodRegistration],
    pub(crate) editor_hints: fn() -> &'static [&'static UiPropertyEditorHint],
    pub(crate) enum_docs: fn() -> &'static [&'static EnumVariantDocs],
    pub(crate) scene_props: fn() -> &'static [&'static ScenePropsApplierRegistration],
    pub(crate) runtime_behaviors: fn() -> &'static [&'static RuntimeBehaviorRegistration],
    pub(crate) add_registrations: fn(&Registrations),
}

/// The registrations one copy's `inventory` collected, handed to the owner
/// by [`attach`].
pub struct Registrations {
    engine_classes: Vec<&'static EngineClassRegistration>,
    component_methods: Vec<&'static ComponentMethodRegistration>,
    methods: Vec<&'static TypeMethodRegistration>,
    conversions: Vec<&'static ConversionRegistration>,
    runtime_types: Vec<&'static RuntimeTypeRegistration>,
    dyn_methods: Vec<&'static DynMethodRegistration>,
    type_renderers: Vec<&'static TypeRendererRegistration>,
    editor_hints: Vec<&'static UiPropertyEditorHint>,
    enum_docs: Vec<&'static EnumVariantDocs>,
    scene_props: Vec<&'static ScenePropsApplierRegistration>,
    runtime_behaviors: Vec<&'static RuntimeBehaviorRegistration>,
}

fn collected<T: inventory::Collect>() -> Vec<&'static T> {
    inventory::iter::<T>.into_iter().collect()
}

impl Registrations {
    fn collected() -> Self {
        Self {
            engine_classes: collected(),
            component_methods: collected(),
            methods: collected(),
            conversions: collected(),
            runtime_types: collected(),
            dyn_methods: collected(),
            type_renderers: collected(),
            editor_hints: collected(),
            enum_docs: collected(),
            scene_props: collected(),
            runtime_behaviors: collected(),
        }
    }
}

/// A list of `'static` registrations: one copy's `inventory` collection,
/// extended by attached copies'. Readers load one pointer; extending
/// replaces the list and leaks the old one (once per plugin load, so
/// bounded).
pub struct AppendList<T: 'static> {
    list: AtomicPtr<Vec<&'static T>>,
    write: Mutex<()>,
    build: fn() -> Vec<&'static T>,
}

impl<T: 'static> AppendList<T> {
    /// A list that starts as `build()` on first use.
    pub const fn new(build: fn() -> Vec<&'static T>) -> Self {
        Self {
            list: AtomicPtr::new(std::ptr::null_mut()),
            write: Mutex::new(()),
            build,
        }
    }

    fn get_locked(&'static self) -> &'static [&'static T] {
        let list = self.list.load(Ordering::Acquire);
        if !list.is_null() {
            // SAFETY: lists are leaked, never freed.
            return unsafe { &*list };
        }
        let built = Box::leak(Box::new((self.build)()));
        self.list.store(built, Ordering::Release);
        built
    }

    /// Every registration, in link order, then in the order copies attached.
    pub fn get(&'static self) -> &'static [&'static T] {
        let list = self.list.load(Ordering::Acquire);
        if !list.is_null() {
            // SAFETY: lists are leaked, never freed.
            return unsafe { &*list };
        }
        let _write = self.write.lock().unwrap_or_else(|e| e.into_inner());
        self.get_locked()
    }

    /// Append `entries` after the existing ones.
    pub fn extend(&'static self, entries: &[&'static T]) {
        let _write = self.write.lock().unwrap_or_else(|e| e.into_inner());
        let mut next = self.get_locked().to_vec();
        next.extend_from_slice(entries);
        self.list.store(Box::leak(Box::new(next)), Ordering::Release);
    }
}

/// A registry built from an [`AppendList`] of its registrations, and built
/// again when the list is extended (the old registry is leaked).
struct Rebuilt<T: 'static, R: 'static> {
    registrations: AppendList<R>,
    value: AtomicPtr<T>,
    write: Mutex<()>,
    build: fn(&[&'static R]) -> T,
}

impl<T: 'static, R: 'static> Rebuilt<T, R> {
    const fn new(collect: fn() -> Vec<&'static R>, build: fn(&[&'static R]) -> T) -> Self {
        Self {
            registrations: AppendList::new(collect),
            value: AtomicPtr::new(std::ptr::null_mut()),
            write: Mutex::new(()),
            build,
        }
    }

    fn get(&'static self) -> &'static T {
        let value = self.value.load(Ordering::Acquire);
        if !value.is_null() {
            // SAFETY: registries are leaked, never freed.
            return unsafe { &*value };
        }
        let _write = self.write.lock().unwrap_or_else(|e| e.into_inner());
        let value = self.value.load(Ordering::Acquire);
        if !value.is_null() {
            // SAFETY: as above.
            return unsafe { &*value };
        }
        let built = Box::leak(Box::new((self.build)(self.registrations.get())));
        self.value.store(built, Ordering::Release);
        built
    }

    fn extend(&'static self, entries: &[&'static R]) {
        let _write = self.write.lock().unwrap_or_else(|e| e.into_inner());
        self.registrations.extend(entries);
        let built = Box::leak(Box::new((self.build)(self.registrations.get())));
        self.value.store(built, Ordering::Release);
    }
}

/// A public registry: dereferences to the registry of the copy that owns
/// the process's state (this copy's, unless it is attached).
pub struct Shared<T: 'static> {
    select: fn(&'static Runtime) -> &'static T,
}

impl<T: 'static> Shared<T> {
    pub(crate) const fn new(select: fn(&'static Runtime) -> &'static T) -> Self {
        Self { select }
    }
}

impl<T: 'static> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &T {
        (self.select)(runtime())
    }
}

static ENGINE_CLASSES: Rebuilt<EngineClassRegistry, EngineClassRegistration> =
    Rebuilt::new(collected, EngineClassRegistry::from_registrations);
static METHODS: Rebuilt<MethodRegistry, TypeMethodRegistration> =
    Rebuilt::new(collected, MethodRegistry::from_registrations);
static CONVERSIONS: Rebuilt<ConversionRegistry, ConversionRegistration> =
    Rebuilt::new(collected, ConversionRegistry::from_registrations);
static RUNTIME_TYPES: Rebuilt<RuntimeTypeRegistry, RuntimeTypeRegistration> =
    Rebuilt::new(collected, RuntimeTypeRegistry::from_registrations);
static DYN_METHODS: Rebuilt<DynMethodRegistry, DynMethodRegistration> =
    Rebuilt::new(collected, DynMethodRegistry::from_registrations);
static DYNAMIC_TYPES: LazyLock<DynamicTypeRegistry> = LazyLock::new(DynamicTypeRegistry::new);
static TYPE_RENDERERS: LazyLock<Mutex<TypeRendererRegistry>> =
    LazyLock::new(|| Mutex::new(TypeRendererRegistry::from_registrations(&collected())));
static COMPONENT_METHODS: AppendList<ComponentMethodRegistration> = AppendList::new(collected);
static EDITOR_HINTS: AppendList<UiPropertyEditorHint> = AppendList::new(collected);
static ENUM_DOCS: AppendList<EnumVariantDocs> = AppendList::new(collected);
static SCENE_PROPS: AppendList<ScenePropsApplierRegistration> = AppendList::new(collected);
static RUNTIME_BEHAVIORS: AppendList<RuntimeBehaviorRegistration> = AppendList::new(collected);

fn add_registrations(r: &Registrations) {
    ENGINE_CLASSES.extend(&r.engine_classes);
    METHODS.extend(&r.methods);
    CONVERSIONS.extend(&r.conversions);
    RUNTIME_TYPES.extend(&r.runtime_types);
    DYN_METHODS.extend(&r.dyn_methods);
    {
        let mut renderers = TYPE_RENDERERS.lock().unwrap_or_else(|e| e.into_inner());
        for registration in &r.type_renderers {
            renderers.register(registration.type_id, registration.renderer.clone());
        }
    }
    COMPONENT_METHODS.extend(&r.component_methods);
    EDITOR_HINTS.extend(&r.editor_hints);
    ENUM_DOCS.extend(&r.enum_docs);
    SCENE_PROPS.extend(&r.scene_props);
    RUNTIME_BEHAVIORS.extend(&r.runtime_behaviors);
}

static OWN: Runtime = Runtime {
    abi: FINGERPRINT,
    engine_classes: || ENGINE_CLASSES.get(),
    methods: || METHODS.get(),
    conversions: || CONVERSIONS.get(),
    runtime_types: || RUNTIME_TYPES.get(),
    dyn_methods: || DYN_METHODS.get(),
    dynamic_types: || &DYNAMIC_TYPES,
    type_renderers: || &TYPE_RENDERERS,
    component_method_registrations: || COMPONENT_METHODS.get(),
    dyn_method_registrations: || DYN_METHODS.registrations.get(),
    editor_hints: || EDITOR_HINTS.get(),
    enum_docs: || ENUM_DOCS.get(),
    scene_props: || SCENE_PROPS.get(),
    runtime_behaviors: || RUNTIME_BEHAVIORS.get(),
    add_registrations,
};

static ATTACHED: AtomicPtr<Runtime> = AtomicPtr::new(std::ptr::null_mut());

/// The runtime this copy uses: the one it is attached to, or its own.
#[inline]
pub(crate) fn runtime() -> &'static Runtime {
    let attached = ATTACHED.load(Ordering::Acquire);
    if attached.is_null() {
        &OWN
    } else {
        // SAFETY: `attach` only stores a pointer to a `Runtime` that lives
        // for the process, checked for this ABI.
        unsafe { &*attached }
    }
}

/// The runtime to hand a plugin library's copy of this crate: the one this
/// copy uses.
pub fn shared() -> &'static Runtime {
    runtime()
}

/// Every registered `#[property_editor]` hint: this copy's and every
/// attached copy's.
pub fn editor_hints() -> &'static [&'static UiPropertyEditorHint] {
    (runtime().editor_hints)()
}

pub(crate) fn component_method_registrations() -> &'static [&'static ComponentMethodRegistration] {
    (runtime().component_method_registrations)()
}

pub(crate) fn dyn_method_registrations() -> &'static [&'static DynMethodRegistration] {
    (runtime().dyn_method_registrations)()
}

pub(crate) fn enum_docs() -> &'static [&'static EnumVariantDocs] {
    (runtime().enum_docs)()
}

pub(crate) fn scene_props() -> &'static [&'static ScenePropsApplierRegistration] {
    (runtime().scene_props)()
}

pub(crate) fn runtime_behaviors() -> &'static [&'static RuntimeBehaviorRegistration] {
    (runtime().runtime_behaviors)()
}

/// Why [`attach`] refused a runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachError {
    /// The pointer was null.
    Null,
    /// The runtime was built by a copy of another ABI: another version of
    /// this crate or another compiler.
    Abi { expected: u64, found: u64 },
    /// This copy is already attached to another runtime.
    AlreadyAttached,
}

impl fmt::Display for AttachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("no runtime given"),
            Self::Abi { expected, found } => write!(
                f,
                "runtime ABI mismatch (expected {expected:#x}, found {found:#x}): \
                 the library was built from other sources or compiler"
            ),
            Self::AlreadyAttached => f.write_str("this copy is attached to another runtime"),
        }
    }
}

impl std::error::Error for AttachError {}

/// Attach this copy to `owner`, the runtime of the copy that owns the
/// process's registries (see [`shared`]), and hand it the registrations
/// this copy collected. Call it before this copy does anything else.
/// Attaching a copy to its own runtime, or again to the same one, does
/// nothing.
///
/// # Safety
///
/// `owner` is null or points to a [`Runtime`] that lives for the rest of
/// the process, and whose functions stay loaded that long.
pub unsafe fn attach(owner: *const Runtime) -> Result<(), AttachError> {
    // SAFETY: the caller's contract.
    let owner = unsafe { owner.as_ref() }.ok_or(AttachError::Null)?;
    if owner.abi != OWN.abi {
        return Err(AttachError::Abi {
            expected: OWN.abi,
            found: owner.abi,
        });
    }
    if std::ptr::eq(owner, &OWN) {
        return Ok(());
    }
    let owner_ptr = owner as *const Runtime as *mut Runtime;
    match ATTACHED.compare_exchange(
        std::ptr::null_mut(),
        owner_ptr,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) => {}
        Err(current) if current == owner_ptr => return Ok(()),
        Err(_) => return Err(AttachError::AlreadyAttached),
    }
    (owner.add_registrations)(&Registrations::collected());
    Ok(())
}

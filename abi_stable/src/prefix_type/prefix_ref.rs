use crate::{
    abi_stability::{GetStaticEquivalent, GetStaticEquivalent_, PrefixStableAbi, StableAbi},
    pointer_trait::{GetPointerKind, PK_Reference},
    prefix_type::{PrefixMetadata, PrefixRefTrait, WithMetadata_},
    reexports::True,
    reflection::ModReflMode,
    sabi_types::StaticRef,
    std_types::RSlice,
    type_layout::{
        CompTLField, GenericTLData, LifetimeRange, MonoTLData, MonoTypeLayout, ReprAttr, TypeLayout,
    },
    utils::Transmuter,
};

use std::{
    fmt::{self, Debug},
    ptr::NonNull,
};

/// A reference to a prefix type.
///
/// This is the type that all `*_Ref` pointer types generated by `StableAbi` wrap.
///
/// # Example
///
/// ```rust
/// use abi_stable::{
///     prefix_type::{PrefixRef, PrefixTypeTrait, WithMetadata},
///     staticref, StableAbi,
/// };
///
/// fn main() {
///     // `Module_Ref`'s constructor can also be called at compile-time
///     asserts(Module_Ref(PREFIX_A));
///     asserts(Module_Ref(PREFIX_B));
/// }
///
/// fn asserts(module: Module_Ref) {
///     assert_eq!(module.first(), 5);
///     assert_eq!(module.second(), 8);
///
///     // If this Module_Ref had come from a previous version of the library without a
///     // `third` field it would return `None`.
///     assert_eq!(module.third(), Some(13));
/// }
///
/// #[repr(C)]
/// #[derive(StableAbi)]
/// #[sabi(kind(Prefix(prefix_ref = Module_Ref, prefix_fields = Module_Prefix)))]
/// struct Module {
///     first: usize,
///     // The `#[sabi(last_prefix_field)]` attribute here means that this is
///     // the last field in this struct that was defined in the
///     // first compatible version of the library,
///     // requiring new fields to always be added after it.
///     // Moving this attribute is a breaking change, it can only be done in a
///     // major version bump..
///     #[sabi(last_prefix_field)]
///     second: usize,
///     third: usize,
/// }
///
/// const MOD_VAL: Module = Module {
///     first: 5,
///     second: 8,
///     third: 13,
/// };
///
/// /////////////////////////////////////////
/// // First way to construct a PrefixRef
/// // This is a way that PrefixRef can be constructed in statics
///
/// const PREFIX_A: PrefixRef<Module_Prefix> = {
///     const S: &WithMetadata<Module> =
///         &WithMetadata::new(PrefixTypeTrait::METADATA, MOD_VAL);
///
///     S.static_as_prefix()
/// };
///
/// /////////////////////////////////////////
/// // Second way to construct a PrefixRef
/// // This is a way that PrefixRef can be constructed in associated constants,
///
/// struct WithAssoc;
///
/// impl WithAssoc {
///     // This macro declares a `StaticRef` pointing to the assigned `WithMetadata`.
///     staticref!(const MOD_WM: WithMetadata<Module> = {
///         WithMetadata::new(PrefixTypeTrait::METADATA, MOD_VAL)
///     });
/// }
///
/// const PREFIX_B: PrefixRef<Module_Prefix> = WithAssoc::MOD_WM.as_prefix();
///
/// /////////////////////////////////////////
///
/// ```
#[repr(transparent)]
pub struct PrefixRef<P> {
    ptr: NonNull<WithMetadata_<P, P>>,
}

impl<P> Clone for PrefixRef<P> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<P> Copy for PrefixRef<P> {}

unsafe impl<'a, P: 'a> Sync for PrefixRef<P> where &'a WithMetadata_<P, P>: Sync {}

unsafe impl<'a, P: 'a> Send for PrefixRef<P> where &'a WithMetadata_<P, P>: Send {}

impl<P> Debug for PrefixRef<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metadata = self.metadata();
        f.debug_struct("PrefixRef")
            .field("metadata", &metadata)
            .field("value_type", &std::any::type_name::<P>())
            .finish()
    }
}

impl<P> PrefixRef<P> {
    /// Constructs a `PrefixRef` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be a non-dangling pointer to a valid, initialized instance of `T`,
    /// and live for the rest of the program's lifetime
    /// (if called at compile-time it means live for the entire program).
    ///
    /// `T` must implement `PrefixTypeTrait<Fields = P>`,
    /// this is automatically true if this is called with
    /// `&WithMetadata::new(PrefixTypeTrait::METADATA, <value>)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abi_stable::{
    ///     for_examples::{Module, Module_Prefix, Module_Ref},
    ///     prefix_type::{PrefixRef, PrefixTypeTrait, WithMetadata},
    ///     rstr,
    ///     std_types::*,
    /// };
    ///
    /// const MOD_WM: &WithMetadata<Module> = {
    ///     &WithMetadata::new(
    ///         PrefixTypeTrait::METADATA,
    ///         Module {
    ///             first: RSome(3),
    ///             second: rstr!("hello"),
    ///             third: 8,
    ///         },
    ///     )
    /// };
    ///
    /// const PREFIX: PrefixRef<Module_Prefix> = unsafe { PrefixRef::from_raw(MOD_WM) };
    ///
    /// const MODULE: Module_Ref = Module_Ref(PREFIX);
    ///
    /// assert_eq!(MODULE.first(), RSome(3));
    ///
    /// assert_eq!(MODULE.second().as_str(), "hello");
    ///
    /// // The accessor returns an `Option` because the field comes after the prefix,
    /// // and returning an Option is the default for those.
    /// assert_eq!(MODULE.third(), Some(8));
    ///
    /// ```
    #[inline(always)]
    pub const unsafe fn from_raw<T>(ptr: *const WithMetadata_<T, P>) -> Self {
        Self {
            ptr: unsafe {
                NonNull::new_unchecked(
                    ptr as *const WithMetadata_<P, P> as *mut WithMetadata_<P, P>,
                )
            },
        }
    }

    /// Constructs a `PrefixRef` from a [`StaticRef`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use abi_stable::{
    ///     for_examples::{Module, Module_Prefix, Module_Ref},
    ///     prefix_type::{PrefixRef, PrefixTypeTrait, WithMetadata},
    ///     rstr, staticref,
    ///     std_types::*,
    /// };
    ///
    /// struct Foo {}
    ///
    /// impl Foo {
    ///     // This macro declares a `StaticRef` pointing to the assigned `WithMetadata`.
    ///     staticref! {const MOD_WM: WithMetadata<Module> =
    ///         WithMetadata::new(PrefixTypeTrait::METADATA, Module{
    ///             first: RNone,
    ///             second: rstr!("world"),
    ///             third: 13,
    ///         })
    ///     }
    /// }
    ///
    /// const PREFIX: PrefixRef<Module_Prefix> = PrefixRef::from_staticref(Foo::MOD_WM);
    ///
    /// const MODULE: Module_Ref = Module_Ref(PREFIX);
    ///
    /// assert_eq!(MODULE.first(), RNone);
    ///
    /// assert_eq!(MODULE.second().as_str(), "world");
    ///
    /// // The accessor returns an `Option` because the field comes after the prefix,
    /// // and returning an Option is the default for those.
    /// assert_eq!(MODULE.third(), Some(13));
    ///
    /// ```
    ///
    /// [`StaticRef`]: ../sabi_types/struct.StaticRef.html
    #[inline]
    pub const fn from_staticref<T>(ptr: StaticRef<WithMetadata_<T, P>>) -> Self {
        unsafe { Self::from_raw(ptr.as_ptr()) }
    }

    /// Constructs a `PrefixRef` from a static reference.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abi_stable::{
    ///     for_examples::{Module, Module_Prefix, Module_Ref},
    ///     prefix_type::{PrefixRef, PrefixTypeTrait, WithMetadata},
    ///     rstr,
    ///     std_types::*,
    /// };
    ///
    /// const MOD_WM: &WithMetadata<Module> = {
    ///     &WithMetadata::new(
    ///         PrefixTypeTrait::METADATA,
    ///         Module {
    ///             first: RNone,
    ///             second: rstr!("foo"),
    ///             third: 21,
    ///         },
    ///     )
    /// };
    ///
    /// const PREFIX: PrefixRef<Module_Prefix> = PrefixRef::from_ref(MOD_WM);
    ///
    /// const MODULE: Module_Ref = Module_Ref(PREFIX);
    ///
    /// assert_eq!(MODULE.first(), RNone);
    ///
    /// assert_eq!(MODULE.second().as_str(), "foo");
    ///
    /// // The accessor returns an `Option` because the field comes after the prefix,
    /// // and returning an Option is the default for those.
    /// assert_eq!(MODULE.third(), Some(21));
    ///
    /// ```
    ///
    #[inline]
    pub const fn from_ref<T>(ptr: &'static WithMetadata_<T, P>) -> Self {
        unsafe { Self::from_raw(ptr) }
    }

    /// Gets the metadata about the prefix type, including available fields.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abi_stable::{
    ///     for_examples::{Module, Module_Prefix},
    ///     prefix_type::{PrefixRef, PrefixTypeTrait, WithMetadata},
    ///     std_types::*,
    /// };
    ///
    /// const MOD_WM: &WithMetadata<Module> = {
    ///     &WithMetadata::new(
    ///         PrefixTypeTrait::METADATA,
    ///         Module {
    ///             first: RNone,
    ///             second: RStr::empty(),
    ///             third: 0,
    ///         },
    ///     )
    /// };
    ///
    /// const PREFIX: PrefixRef<Module_Prefix> = PrefixRef::from_ref(MOD_WM);
    ///
    /// let accessibility = PREFIX.metadata().field_accessibility();
    ///
    /// assert!(accessibility.is_accessible(0)); // The `first` field
    /// assert!(accessibility.is_accessible(1)); // The `second` field
    /// assert!(accessibility.is_accessible(2)); // The `third` field
    /// assert!(!accessibility.is_accessible(3)); // There's no field after `third`
    ///
    /// ```
    ///
    #[inline]
    pub fn metadata(self) -> PrefixMetadata<P, P> {
        unsafe { (*self.ptr.as_ptr()).metadata }
    }

    /// Gets a reference to the pointed-to prefix.
    ///
    /// # Example
    ///
    /// ```rust
    /// use abi_stable::{
    ///     for_examples::{Module, Module_Prefix},
    ///     prefix_type::{PrefixRef, PrefixTypeTrait, WithMetadata},
    ///     rstr,
    ///     std_types::*,
    /// };
    ///
    /// const MOD_WM: &WithMetadata<Module> = {
    ///     &WithMetadata::new(
    ///         PrefixTypeTrait::METADATA,
    ///         Module {
    ///             first: RNone,
    ///             second: rstr!("foo"),
    ///             third: 21,
    ///         },
    ///     )
    /// };
    ///
    /// const PREFIX_REF: PrefixRef<Module_Prefix> = PrefixRef::from_ref(MOD_WM);
    ///
    /// let prefix: &Module_Prefix = PREFIX_REF.prefix();
    ///
    /// assert_eq!(prefix.first, RNone);
    ///
    /// assert_eq!(prefix.second.as_str(), "foo");
    ///
    /// // The `third` field is not in the prefix, so it can't be accessed here.
    /// // prefix.third;
    ///
    /// ```
    ///
    #[inline]
    pub fn prefix<'a>(self) -> &'a P {
        unsafe { &(*self.ptr.as_ptr()).value.0 }
    }

    /// Converts this PrefixRef into a raw pointer.
    #[inline(always)]
    pub const fn to_raw_ptr(self) -> *const WithMetadata_<P, P> {
        unsafe { Transmuter { from: self }.to }
    }

    /// Casts the pointed-to prefix to another type.
    ///
    /// # Safety
    ///
    /// This function is intended for casting the `PrefixRef<P>` to `PrefixRef<U>`,
    /// and then cast back to `PrefixRef<P>` to use it again.
    ///
    /// The prefix in the returned `PrefixRef<U>` must only be accessed
    /// when this `PrefixRef` was originally cosntructed with a `ẀithMetadata_<_, U>`.
    /// access includes calling `prefix`, and reading the `value` field in the `WithMetadata`
    /// that this points to.
    ///
    pub const unsafe fn cast<U>(self) -> PrefixRef<U> {
        PrefixRef {
            ptr: self.ptr.cast(),
        }
    }
}

unsafe impl<P> GetStaticEquivalent_ for PrefixRef<P>
where
    P: GetStaticEquivalent_,
{
    type StaticEquivalent = PrefixRef<GetStaticEquivalent<P>>;
}

unsafe impl<P> StableAbi for PrefixRef<P>
where
    P: PrefixStableAbi,
{
    type IsNonZeroType = True;

    const LAYOUT: &'static TypeLayout = {
        const MONO_TYPE_LAYOUT: &MonoTypeLayout = &MonoTypeLayout::new(
            *mono_shared_vars,
            rstr!("PrefixRef"),
            make_item_info!(),
            MonoTLData::struct_(rslice![]),
            tl_genparams!('a;0;),
            ReprAttr::Transparent,
            ModReflMode::DelegateDeref { layout_index: 0 },
            {
                const S: &[CompTLField] =
                    &[CompTLField::std_field(field0, LifetimeRange::EMPTY, 0)];
                RSlice::from_slice(S)
            },
        );

        make_shared_vars! {
            impl[P] PrefixRef<P>
            where [P: PrefixStableAbi];

            let (mono_shared_vars,shared_vars)={
                strings={ field0:"0", },
                prefix_type_layouts=[P],
            };
        }

        &TypeLayout::from_std::<Self>(
            shared_vars,
            MONO_TYPE_LAYOUT,
            Self::ABI_CONSTS,
            GenericTLData::Struct,
        )
    };
}

unsafe impl<P> GetPointerKind for PrefixRef<P> {
    type PtrTarget = WithMetadata_<P, P>;
    type Kind = PK_Reference;
}

unsafe impl<P> PrefixRefTrait for PrefixRef<P> {
    type PrefixFields = P;
}

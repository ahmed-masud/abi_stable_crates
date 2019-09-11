use super::*;

use crate::{
    abi_stability::{
        abi_checking::{AbiInstability,push_err},
    },
    const_utils::log2_usize,
    std_types::{RSlice,RVec,RString},
};


///////////////////////////


/// The layout of an enum,that doesn't depend on generic parameters.
#[repr(C)]
#[derive(Copy, Clone, StableAbi)]
#[sabi(unsafe_sabi_opaque_fields)]
pub struct MonoTLEnum{
    /// The ammount of fields of each variant.
    field_count:*const u8,
    field_count_len:u16,

    /// A ';' separated list of all variant names
    variant_names:StartLen,

    /// All the fields of the enums,not separated by variant.
    pub(super) fields: CompTLFields,
}

unsafe impl Sync for MonoTLEnum {}
unsafe impl Send for MonoTLEnum {}


impl MonoTLEnum{
    /// Constructs a `TLEnum`.
    pub const fn new(
        variant_names:StartLen,
        field_count:RSlice<'static,u8>,
        fields: CompTLFields,
    ) -> Self {
        Self {
            field_count:field_count.as_ptr(),
            field_count_len:field_count.len() as u16,
            variant_names,
            fields,
        }
    }

    /// Returns the ammount of variants in the enum.
    pub fn variant_count(&self)->usize{
        self.field_count_len as usize
    }

    pub fn field_count(&self)->RSlice<'static,u8>{
        unsafe{
            RSlice::from_raw_parts( self.field_count, self.field_count_len as usize )
        }
    }

    pub fn expand(self,other:GenericTLEnum,shared_vars:&'static SharedVars)->TLEnum{
        TLEnum{
            field_count:self.field_count(),
            variant_names:(&shared_vars.strings()[self.variant_names.to_range()]).into(),
            fields:self.fields.expand(shared_vars),
            exhaustiveness: other.exhaustiveness,
            discriminants: other.discriminants,
        }
    }
}

///////////////////////////

/// The layout of an enum,that might depend on generic parameters.
#[repr(C)]
#[derive(Debug, Copy, Clone, StableAbi)]
#[sabi(unsafe_sabi_opaque_fields)]
pub struct GenericTLEnum{
    /// The exhaustiveness of this enum.
    exhaustiveness: IsExhaustive,
    /// The discriminants of the variants in the enum.
    discriminants: TLDiscriminants,
}


impl GenericTLEnum{
    /// Constructs a `TLData::Enum`.
    pub const fn new(
        exhaustiveness:IsExhaustive,
        discriminants:TLDiscriminants,
    ) -> Self {
        Self {
            exhaustiveness,
            discriminants,
        }
    }

    pub const fn exhaustive(discriminants:TLDiscriminants)->Self{
        Self::new(IsExhaustive::exhaustive(),discriminants)
    }
}


///////////////////////////

/// Every property about an enum specifically.
#[derive(Debug,Copy,Clone,PartialEq,Eq)]
pub struct TLEnum{
    /// The ammount of fields of each variant.
    pub field_count:RSlice<'static,u8>,

    /// A ';' separated list of all variant names
    pub variant_names:RStr<'static>,

    /// All the fields of the enums,not separated by variant.
    pub fields: TLFields,

    /// The exhaustiveness of this enum.
    pub exhaustiveness: IsExhaustive,

    /// The discriminants of the variants in the enum.
    pub discriminants: TLDiscriminants,
}

impl TLEnum{
    /// Returns the ammount of variants in the enum.
    pub fn variant_count(&self)->usize{
        self.field_count.len()
    }
    /// Returns an iterator over the names of the variants in this enum.
    pub fn variant_names_iter(&self)->GetVariantNames{
        GetVariantNames{
            split:self.variant_names.as_str().split(';'),
            length:self.field_count.len(),
            current:0,
        }
    }

    /// Returns the enum with the (maximum,minimum) ammount of variants.
    pub fn max_min<'a>(&'a self,other:&'a TLEnum)->(&'a TLEnum,&'a TLEnum){
        if self.variant_count() < other.variant_count() {
            (self,other)
        }else{
            (other,self)
        }
    }

}


impl Display for TLEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f,"variants:{:?}",self.variant_names)?;
        writeln!(
            f,
            "fields(all variants combined):\n{}",
            self.fields.to_string().left_padder(4)
        )?;
        writeln!(f,"field counts(per-variant):{:?}",self.field_count)?;
        writeln!(f,"exhaustiveness:{:?}",self.exhaustiveness)?;
        writeln!(f,"discriminants:{:?}",self.discriminants)?;
        Ok(())        
    }
}



///////////////////////////




macro_rules! declare_tl_discriminants {
    (
        $((
            $(#[$variant_attr:meta])*
            $variant:ident ( $ty:ty ),
            $single:ident,
            $(#[$method_attr:meta])*
            $method:ident
        ))*
    ) => (
        /// The discriminant of an enum variant.
        #[repr(u8)]
        #[derive(Copy, Clone, StableAbi)]
        pub enum TLDiscriminants{
            $(
                $(#[$variant_attr])*
                // Storing the length and pointer like this so that the enum 
                // is only 2 usize large.
                $variant{
                    len:u16,
                    discriminants:*const $ty,
                },
            )*
        }

        impl Debug for TLDiscriminants{
            fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
                match *self {
                    $(
                        TLDiscriminants::$variant{discriminants,len}=>unsafe{
                            let slice=std::slice::from_raw_parts(discriminants,len as usize);
                            Debug::fmt(slice,f)
                        }
                    )*
                }
            }
        }

        impl PartialEq for TLDiscriminants {
            fn eq(&self,other:&Self)->bool{
                match (*self,*other) {
                    $(
                        (
                            TLDiscriminants::$variant{discriminants: t_discr_ptr, len:t_len },
                            TLDiscriminants::$variant{discriminants: o_discr_ptr, len:o_len }
                        )=>{
                            let t_discrs=unsafe{
                                RSlice::from_raw_parts(t_discr_ptr,t_len as usize) 
                            };
                            let o_discrs=unsafe{
                                RSlice::from_raw_parts(o_discr_ptr,o_len as usize) 
                            };
                            t_discrs==o_discrs
                        }
                    )*
                    _=>false,
                }
            }
        }

        impl Eq for TLDiscriminants{}

        impl TLDiscriminants{

            $(
                $(#[$method_attr])*
                pub const fn $method(arr:RSlice<'static,$ty>)->Self{
                    TLDiscriminants::$variant{
                        len:arr.len() as u16,
                        discriminants:arr.as_ptr(),
                    }
                }
            )*

            /// Gets the type of a discriminant in this TLDiscriminants.
            pub fn discriminant_repr(&self)->DiscriminantRepr{
                match self {
                    $(
                        TLDiscriminants::$variant{..}=>DiscriminantRepr::$variant,
                    )*
                }
            }

            pub fn compare(&self,other:&Self)->Result<(),RVec<AbiInstability>>{
                let mut errs=RVec::new();
                match (*self,*other) {
                    $(
                        (
                            TLDiscriminants::$variant{discriminants: t_discr_ptr, len:t_len },
                            TLDiscriminants::$variant{discriminants: o_discr_ptr, len:o_len }
                        )=>{
                            let t_discrs=unsafe{
                                RSlice::from_raw_parts(t_discr_ptr,t_len as usize) 
                            };
                            let o_discrs=unsafe{
                                RSlice::from_raw_parts(o_discr_ptr,o_len as usize) 
                            };

                            for (&t_discr,&o_discr) in 
                                t_discrs.as_slice().iter().zip(o_discrs.as_slice())
                            {
                                if t_discr!=o_discr {
                                    push_err(
                                        &mut errs,
                                        t_discr,
                                        o_discr,
                                        |x| TLDiscriminant::$single(x as _),
                                        AbiInstability::EnumDiscriminant,
                                    );
                                }
                            }
                        }
                    )*
                    _=>{
                        push_err(
                            &mut errs,
                            self,
                            other,
                            |x| ReprAttr::Int(x.discriminant_repr()),
                            AbiInstability::ReprAttr
                        );
                    }
                }
                if errs.is_empty(){
                    Ok(())
                }else{
                    Err(errs)
                }
            }
        }
    )
}


declare_tl_discriminants!{
    ( U8(u8) ,Signed  , from_u8_slice )
    ( I8(i8) ,Unsigned, from_i8_slice )
    ( U16(u16) ,Signed  , from_u16_slice )
    ( I16(i16) ,Unsigned, from_i16_slice )
    ( U32(u32) ,Signed  , from_u32_slice )
    ( I32(i32) ,Unsigned, from_i32_slice )
    ( U64(u64) ,Signed  , from_u64_slice )
    ( I64(i64) ,Unsigned, from_i64_slice )
    ( Usize(usize) ,Usize, from_usize_slice )
    ( Isize(isize) ,Isize, from_isize_slice )
}



/// The discriminant of an enum variant.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, StableAbi)]
#[sabi(unsafe_sabi_opaque_fields)]
pub enum TLDiscriminant{
    /// The assigned value of a discriminant in a `#[repr(isize)]` enum.
    Isize(isize),
    /// The assigned value of a discriminant in a `#[repr(usize)]` enum.
    Usize(usize),
    /// The assigned value of a discriminant in a `#[repr(i8/i16/i32/i64)]` enum.
    Signed(i64),
    /// The assigned value of a discriminant in a `#[repr(u8/u16/u32/u64)]` enum.
    Unsigned(u64),
}




/// How the discriminant of an enum is represented.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
#[sabi(unsafe_sabi_opaque_fields)]
pub enum DiscriminantRepr {
    /// The type of the discriminant for a `#[repr(u8)]`enum
    U8,
    /// The type of the discriminant for a `#[repr(i8)]`enum
    I8,
    /// The type of the discriminant for a `#[repr(u16)]`enum
    U16,
    /// The type of the discriminant for a `#[repr(i16)]`enum
    I16,
    /// The type of the discriminant for a `#[repr(u32)]`enum
    U32,
    /// The type of the discriminant for a `#[repr(i32)]`enum
    I32,
    /// The type of the discriminant for a `#[repr(u64)]`enum
    U64,
    /// The type of the discriminant for a `#[repr(i64)]`enum
    I64,
    /// Reserved,just in case that u128 gets a c-compatible layout
    U128,
    /// Reserved,just in case that i128 gets a c-compatible layout
    I128,
    /// The type of the discriminant for a `#[repr(usize)]`enum
    Usize,
    /// The type of the discriminant for a `#[repr(isize)]`enum
    ///
    /// This is the default discriminant type for `repr(C)`.
    Isize,
}


/// Whether this enum is exhaustive,if it is,it can add variants in minor versions.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
#[sabi(unsafe_sabi_opaque_fields)]
pub struct IsExhaustive{
    value:Option<&'static TLNonExhaustive>,
}


impl IsExhaustive{
    /// Constructs this `IsExhaustive` as being exhaustive.
    pub const fn exhaustive()->IsExhaustive{
        IsExhaustive{value:None}
    }
    /// Constructs this `IsExhaustive` as being nonexhaustive.
    pub const fn nonexhaustive(nonexhaustive:&'static TLNonExhaustive)->IsExhaustive{
        IsExhaustive{value:Some(nonexhaustive)}
    }
    /// Whether this is an exhaustive enum.
    pub fn is_exhaustive(&self)->bool{
        self.value.is_none()
    }
    /// Whether this is an nonexhaustive enum.
    pub fn is_nonexhaustive(&self)->bool{
        self.value.is_some()
    }
    /// Converts this to a TLNonExhaustive.Returning None if it is exhaustive.
    pub fn as_nonexhaustive(&self)->Option<&'static TLNonExhaustive>{
        self.value
    }
}


/// Properties exclusive to nonexhaustive enums.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
#[sabi(unsafe_sabi_opaque_fields)]
pub struct TLNonExhaustive{
    original_size:usize,
    original_alignment_pow2:u8,
}


impl TLNonExhaustive{
    /// Constructs the TLNonExhaustive from the size and alignment of `T`
    pub const fn new<T>()->Self{
        Self{
            original_size:std::mem::size_of::<T>(),
            original_alignment_pow2:log2_usize(mem::align_of::<T>()),
        }
    }

    #[inline]
    fn original_size(&self)->usize{
        self.original_size
    }
    #[inline]
    fn original_alignment(&self)->usize{
        1_usize << (self.original_alignment_pow2 as u32)
    }

    /// Checks that `layout` is compatible with `self.size` and `self.alignment`,
    /// returning an error if it's not.
    pub fn check_compatible(&self,layout:&TypeLayout)->Result<(),IncompatibleWithNonExhaustive>{
        let err=
            layout.size() < self.original_size() || 
            layout.alignment() < self.original_alignment();

        if err {
            Err(IncompatibleWithNonExhaustive{
                full_type:layout.full_type().to_string().into(),
                module_path:layout.mod_path(),
                type_size:self.original_size(),
                type_alignment:self.original_alignment(),
                storage_size:layout.size(),
                storage_alignment:layout.alignment(),
            })
        }else{
            Ok(())
        }
    }
}

////////////////////////////



/**
An error produced when checking that the Storage of a nonexhaustive enum is 
compatible with the enum.
*/
#[repr(C)]
#[derive(Debug,Clone,PartialEq,Eq,StableAbi)]
#[sabi(unsafe_sabi_opaque_fields)]
pub struct IncompatibleWithNonExhaustive{
    full_type:RString,
    module_path:ModPath,
    type_size:usize,
    type_alignment:usize,
    storage_size:usize,
    storage_alignment:usize,
}


impl Display for IncompatibleWithNonExhaustive{
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        write!(
            f,
            "Type '{ty}' has an incompatible layout for the storage.\n\
             Type    size:{t_size} alignment:{t_align}
             Storage size:{s_size} alignment:{s_align}
             module_path:{mod_}
            ",
            ty=self.full_type,
            t_size=self.type_size,
            t_align=self.type_alignment,
            s_size=self.storage_size,
            s_align=self.storage_alignment,
            mod_=self.module_path,
        )
    }
}

impl std::error::Error for IncompatibleWithNonExhaustive{}


/////////////////////////////////////////////////////////////////////////////


/**
An iterator that yields the names of an enum's variants.
*/
#[derive(Debug,Clone)]
pub struct GetVariantNames{
    split:std::str::Split<'static,char>,
    length:usize,
    current:usize,
}

impl Iterator for GetVariantNames{
    type Item=&'static str;
    fn next(&mut self) -> Option<Self::Item>{
        if self.length==self.current {
            return None;
        }
        let current=self.current;
        self.current+=1;
        match self.split.next().filter(|&x| !x.is_empty()||x=="_" ) {
            Some(x)=>Some(x),
            None=>Some(VARIANT_INDEX[current]),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len=self.length-self.current;
        (len,Some(len))
    }
    fn count(self) -> usize {
        let len=self.length-self.current;
        len
    }
}


impl std::iter::ExactSizeIterator for GetVariantNames{}


static VARIANT_INDEX: [&'static str; 68] = [
    "Variant0", "Variant1", "Variant2", "Variant3", 
    "Variant4", "Variant5", "Variant6", "Variant7", 
    "Variant8", "Variant9", "Variant10", "Variant11", 
    "Variant12", "Variant13", "Variant14", "Variant15",
    "Variant16", "Variant17", "Variant18", "Variant19",
    "Variant20", "Variant21", "Variant22", "Variant23",
    "Variant24", "Variant25", "Variant26", "Variant27",
    "Variant28", "Variant29", "Variant30", "Variant31",
    "Variant32", "Variant33", "Variant34", "Variant35",
    "Variant36", "Variant37", "Variant38", "Variant39",
    "Variant40", "Variant41", "Variant42", "Variant43",
    "Variant44", "Variant45", "Variant46", "Variant47",
    "Variant48", "Variant49", "Variant50", "Variant51",
    "Variant52", "Variant53", "Variant54", "Variant55",
    "Variant56", "Variant57", "Variant58", "Variant59",
    "Variant60", "Variant61", "Variant62", "Variant63",
    "Variant64", "Variant65", "Variant66", "Variant67",
];



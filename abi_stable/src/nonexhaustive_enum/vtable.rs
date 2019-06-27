use std::{
    cmp::{PartialEq,Ord,PartialOrd},
    fmt::{Debug,Display},
    hash::Hash,
    marker::PhantomData,
};

use crate::{
    abi_stability::Tag,
    erased_types::{c_functions,trait_objects,InterfaceType,FormattingMode},
    marker_type::ErasedObject,
    nonexhaustive_enum::{
        alt_c_functions,NonExhaustive,EnumInfo,GetEnumInfo,SerializeEnum,
    },
    prefix_type::{PrefixTypeTrait,WithMetadata,panic_on_missing_fieldname},
    type_level::{
        bools::{True,False,Boolean},
    },
    sabi_types::{StaticRef},
    std_types::{ROption,RResult,RString,RCow,RCmpOrdering,RBoxError},
    traits::InlineStorage,
};


/// Gets the vtable of `NonExhaustive<Self,S,I>`.
pub unsafe trait GetVTable<S,I>:GetEnumInfo{
    const VTABLE_VAL:NonExhaustiveVtableVal<Self,S,I>;
    
    const VTABLE_PTR: *const WithMetadata<NonExhaustiveVtableVal<Self,S,I>> = 
        &WithMetadata::new(PrefixTypeTrait::METADATA,Self::VTABLE_VAL);

    const VTABLE_REF:StaticRef<NonExhaustiveVtable<Self,S,I>>=unsafe{
        let full=WithMetadata::raw_as_prefix(Self::VTABLE_PTR);
        StaticRef::from_raw(full)
    };
}



/// The vtable for NonExhaustive<>.
#[repr(C)]
#[derive(StableAbi)]
#[sabi(
    unconstrained(E,I),
    missing_field(default),
    kind(Prefix(prefix_struct="NonExhaustiveVtable"))
)]
pub struct NonExhaustiveVtableVal<E,S,I>{
    pub(crate) _sabi_tys:PhantomData<extern "C" fn(E,S,I)>,
    
    _sabi_enum_info:*const EnumInfo<u8>,

    pub(crate) _sabi_drop :unsafe extern "C" fn(this:&mut ErasedObject),
    pub(crate) _sabi_clone:Option<
        extern "C" fn(
            &ErasedObject,
            StaticRef<NonExhaustiveVtable<E,S,I>>
        )->NonExhaustive<E,S,I>
    >,
    pub(crate) _sabi_debug:Option<
        extern "C" fn(&ErasedObject,FormattingMode,&mut RString)->RResult<(),()>
    >,
    pub(crate) _sabi_display:Option<
        extern "C" fn(&ErasedObject,FormattingMode,&mut RString)->RResult<(),()>
    >,
    pub(crate) _sabi_serialize: Option<
        extern "C" fn(&ErasedObject)->RResult<RCow<'_,str>,RBoxError>
    >,
    pub(crate) _sabi_partial_eq: Option<
        extern "C" fn(&ErasedObject,&NonExhaustive<E,S,I>)->bool
    >,
    pub(crate) _sabi_cmp: Option<
        extern "C" fn(&ErasedObject,&NonExhaustive<E,S,I>)->RCmpOrdering,
    >,
    pub(crate) _sabi_partial_cmp: Option<
        extern "C" fn(&ErasedObject,&NonExhaustive<E,S,I>)->ROption<RCmpOrdering>,
    >,
    #[sabi(last_prefix_field)]
    pub(crate) _sabi_hash:Option<
        extern "C" fn(&ErasedObject,trait_objects::HasherObject<'_>)
    >,
}


impl<E,S,I> NonExhaustiveVtable<E,S,I>{
    pub fn enum_info(&self)->&'static EnumInfo<<E as GetEnumInfo>::Discriminant>
    where
        E:GetEnumInfo
    {
        unsafe{
            &*(
                self._sabi_enum_info()
                    as *const EnumInfo<u8>
                    as *const EnumInfo<<E as GetEnumInfo>::Discriminant>
            )
        }
    }
}


unsafe impl<E,S,I> GetVTable<S,I> for E
where 
    S:InlineStorage,
    I:InterfaceType,
    E:GetEnumInfo,
    I::Sync:RequiresSync<E,S,I>,
    I::Send:RequiresSend<E,S,I>,
    I::Clone:InitCloneField<E,S,I>,
    I::Debug:InitDebugField<E,S,I>,
    I::Display:InitDisplayField<E,S,I>,
    I::Serialize:InitSerializeField<E,S,I>,
    I::PartialEq:InitPartialEqField<E,S,I>,
    I::PartialOrd:InitPartialOrdField<E,S,I>,
    I::Ord:InitOrdField<E,S,I>,
    I::Hash:InitHashField<E,S,I>,
{
    const VTABLE_VAL:NonExhaustiveVtableVal<E,S,I>=
        NonExhaustiveVtableVal{
            _sabi_tys:PhantomData,
            _sabi_enum_info:E::ENUM_INFO
                as *const EnumInfo<<E as GetEnumInfo>::Discriminant>
                as *const EnumInfo<u8>,
            _sabi_drop:alt_c_functions::drop_impl::<E>,
            _sabi_clone:<I::Clone as InitCloneField<E,S,I>>::VALUE,
            _sabi_debug:<I::Debug as InitDebugField<E,S,I>>::VALUE,
            _sabi_display:<I::Display as InitDisplayField<E,S,I>>::VALUE,
            _sabi_serialize:<I::Serialize as InitSerializeField<E,S,I>>::VALUE,
            _sabi_partial_eq:<I::PartialEq as InitPartialEqField<E,S,I>>::VALUE,
            _sabi_partial_cmp:<I::PartialOrd as InitPartialOrdField<E,S,I>>::VALUE,
            _sabi_cmp:<I::Ord as InitOrdField<E,S,I>>::VALUE,
            _sabi_hash:<I::Hash as InitHashField<E,S,I>>::VALUE,
        };
}





use self::trait_bounds::*;
pub mod trait_bounds{
    use super::*;

    macro_rules! declare_conditional_marker {
        (
            trait $trait_name:ident[$self_:ident,$Filler:ident,$OrigPtr:ident]
            where [ $($where_preds:tt)* ]
        ) => (
            pub trait $trait_name<$self_,$Filler,$OrigPtr>:Boolean{}

            impl<$self_,$Filler,$OrigPtr> $trait_name<$self_,$Filler,$OrigPtr> for False{}
            
            impl<$self_,$Filler,$OrigPtr> $trait_name<$self_,$Filler,$OrigPtr> for True
            where
                $($where_preds)*
            {}
        )
    }

    macro_rules! declare_field_initalizer {
        (
            type $selector:ident;
            trait $trait_name:ident[$enum_:ident,$filler:ident,$interf:ident]
            $( where_for_both[ $($where_preds_both:tt)* ] )?
            where [ $($where_preds:tt)* ]
            $priv_field:ident,$field:ident : $field_ty:ty;
            field_index=$field_index:ident;
            value=$field_value:expr,
        ) => (
            pub trait $trait_name<$enum_,$filler,$interf>:Boolean
            where
                $($($where_preds_both)*)?
            {
                const VALUE:Option<$field_ty>;
            }

            impl<$enum_,$filler,$interf> $trait_name<$enum_,$filler,$interf> for False
            where
                $($($where_preds_both)*)?
            {
                const VALUE:Option<$field_ty>=None;
            }

            impl<$enum_,$filler,$interf> $trait_name<$enum_,$filler,$interf> for True
            where
                $($($where_preds_both)*)?
                $($where_preds)*
            {
                const VALUE:Option<$field_ty>=Some($field_value);
            }

            impl<E,S,$interf> NonExhaustiveVtable<E,S,$interf>{
                pub fn $field(&self)->$field_ty
                where
                    $interf:InterfaceType<$selector=True>,
                {
                    match self.$priv_field().into() {
                        Some(v)=>v,
                        None=>panic_on_missing_fieldname::<
                            NonExhaustiveVtableVal<E,S,$interf>,
                        >(
                            Self::$field_index,
                            self._prefix_type_layout(),
                        )
                    }
                }
            }
        )
    }


    declare_conditional_marker!{
        trait RequiresSend[E,S,I]
        where [ E:Send ]
    }

    declare_conditional_marker!{
        trait RequiresSync[E,S,I]
        where [ E:Sync ]
    }

    declare_field_initalizer!{
        type Clone;
        trait InitCloneField[E,S,I]
        where_for_both[ E:GetEnumInfo, ]
        where [ E:Clone ]
        _sabi_clone,clone_: 
            extern "C" fn(
                &ErasedObject,
                StaticRef<NonExhaustiveVtable<E,S,I>>
            )->NonExhaustive<E,S,I>;
        field_index=field_index_for__sabi_clone;
        value=alt_c_functions::clone_impl::<E,S,I>,
    }
    declare_field_initalizer!{
        type Debug;
        trait InitDebugField[E,S,I]
        where [ E:Debug ]
        _sabi_debug,debug: 
            extern "C" fn(&ErasedObject,FormattingMode,&mut RString)->RResult<(),()>;
        field_index=field_index_for__sabi_debug;
        value=c_functions::debug_impl::<E>,
    }
    declare_field_initalizer!{
        type Display;
        trait InitDisplayField[E,S,I]
        where [ E:Display ]
        _sabi_display,display: 
            extern "C" fn(&ErasedObject,FormattingMode,&mut RString)->RResult<(),()>;
        field_index=field_index_for__sabi_display;
        value=c_functions::display_impl::<E>,
    }
    declare_field_initalizer!{
        type Serialize;
        trait InitSerializeField[E,S,I]
        where [ I:SerializeEnum<E> ]
        _sabi_serialize,serialize: 
            extern "C" fn(&ErasedObject)->RResult<RCow<'_,str>,RBoxError>;
        field_index=field_index_for__sabi_serialize;
        value=alt_c_functions::serialize_impl::<E,I>,
    }
    declare_field_initalizer!{
        type PartialEq;
        trait InitPartialEqField[E,S,I]
        where_for_both[ E:GetEnumInfo, ]
        where [ E:PartialEq ]
        _sabi_partial_eq,partial_eq: extern "C" fn(&ErasedObject,&NonExhaustive<E,S,I>)->bool;
        field_index=field_index_for__sabi_partial_eq;
        value=alt_c_functions::partial_eq_impl::<E,S,I>,
    }
    declare_field_initalizer!{
        type PartialOrd;
        trait InitPartialOrdField[E,S,I]
        where_for_both[ E:GetEnumInfo, ]
        where [ E:PartialOrd ]
        _sabi_partial_cmp,partial_cmp:
            extern "C" fn(&ErasedObject,&NonExhaustive<E,S,I>)->ROption<RCmpOrdering>;
        field_index=field_index_for__sabi_partial_cmp;
        value=alt_c_functions::partial_cmp_ord::<E,S,I>,
    }
    declare_field_initalizer!{
        type Ord;
        trait InitOrdField[E,S,I]
        where_for_both[ E:GetEnumInfo, ]
        where [ E:Ord ]
        _sabi_cmp,cmp: extern "C" fn(&ErasedObject,&NonExhaustive<E,S,I>)->RCmpOrdering;
        field_index=field_index_for__sabi_cmp;
        value=alt_c_functions::cmp_ord::<E,S,I>,
    }
    declare_field_initalizer!{
        type Hash;
        trait InitHashField[E,S,I]
        where [ E:Hash ]
        _sabi_hash,hash: extern "C" fn(&ErasedObject,trait_objects::HasherObject<'_>);
        field_index=field_index_for__sabi_hash;
        value=c_functions::hash_Hash::<E>,
    }
}

macro_rules! declare_InterfaceBound {
    (
        auto_traits=[ $( $auto_trait:ident ),* $(,)* ]
        required_traits=[ $( $required_traits:ident ),* $(,)* ]
    ) => (

        #[allow(non_upper_case_globals)]
        pub trait InterfaceBound:InterfaceType{
            const TAG:Tag;
            $(const $auto_trait:bool;)*
            $(const $required_traits:bool;)*
        }

        #[allow(non_upper_case_globals)]
        impl<I> InterfaceBound for I
        where 
            I:InterfaceType,
            $(I::$auto_trait:Boolean,)*
            $(I::$required_traits:Boolean,)*
        {
            const TAG:Tag={
                const fn str_if(cond:bool,s:&'static str)->Tag{
                    [ Tag::null(), Tag::str(s) ][cond as usize]
                }

                tag!{{
                    "auto traits"=>tag![[
                        $(  
                            str_if(
                                <I::$auto_trait as Boolean>::VALUE,
                                stringify!($auto_trait)
                            ),
                        )*
                    ]],
                    "required traits"=>tag!{{
                        $(  
                            str_if(
                                <I::$required_traits as Boolean>::VALUE,
                                stringify!($required_traits)
                            ),
                        )*
                    }}
                }}
            };

            $(const $auto_trait:bool=<I::$auto_trait as Boolean>::VALUE;)*
            $(const $required_traits:bool=<I::$required_traits as Boolean>::VALUE;)*
        }
    )
}

declare_InterfaceBound!{
    auto_traits=[ Sync,Send ]
    required_traits=[ 
        Clone,
        Debug,Display,
        Serialize,Deserialize,
        Eq,PartialEq,Ord,PartialOrd,
        Hash,Error,
    ]
}
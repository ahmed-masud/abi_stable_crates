use crate::{
    composite_collections::{SmallStartLen as StartLen},
    lifetimes::{LifetimeRange,LifetimeIndex},
};

use super::{
    reflection::{CompFieldAccessor,FieldAccessor},
    shared_vars::SharedVars,
};

use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;


abi_stable_shared::declare_comp_tl_field!{
    attrs=[]
}


impl CompTLField{
    pub(crate) fn from_expanded<'a,I>(
        name:&str,
        lifetime_indices:I,
        field_accessor:FieldAccessor<'a>,
        layout: u16,
        is_function:bool,
        shared_vars:&mut SharedVars<'a>,
    )->Self
    where
        I:IntoIterator<Item=LifetimeIndex>,
    {
        let (name_range,comp_field_accessor)=
            Self::push_name_field_accessor(&name,field_accessor,shared_vars);

        Self::new(
            name_range,
            shared_vars.extend_with_lifetime_indices( lifetime_indices ),
            comp_field_accessor,
            layout,
            is_function,
        )
    }

    pub(crate) fn from_expanded_std_field<'a,I>(
        name:&str,
        lifetime_indices:I,
        layout: u16,
        shared_vars:&mut SharedVars<'a>,
    )->Self
    where
        I:IntoIterator<Item=LifetimeIndex>,
    {
        Self::from_expanded(
            name,
            lifetime_indices,
            FieldAccessor::Direct,
            layout,
            false,
            shared_vars,
        )
    }

    /// Pushes the name and field accessor payload with the 
    /// `<name><field_accessor_payload>;` format.
    fn push_name_field_accessor<'a>(
        name:&str,
        field_accessor:FieldAccessor<'a>,
        shared_vars:&mut SharedVars<'a>,
    )->(StartLen,CompFieldAccessor){
        let name_range=shared_vars.push_str(name);
        let comp_field_accessor=field_accessor.compress(shared_vars);
        shared_vars.push_str(";");
        (name_range,comp_field_accessor)
    }
}


impl CompTLField {
    pub(crate) fn type_<'a>(&self,shared_vars:&SharedVars<'a>)-> &'a syn::Type {
        shared_vars.get_type(self.type_layout_index()).unwrap()
    }
}

impl ToTokens for CompTLField {
    fn to_tokens(&self, ts: &mut TokenStream2) {
        self.bits0.to_tokens(ts);
    }
}


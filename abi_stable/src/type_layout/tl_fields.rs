use super::*;

use std::{
    iter,
    slice,
};

/// The layout of all field in a type definition.
#[repr(C)]
#[derive(Copy, Clone, StableAbi)]
#[sabi(unsafe_sabi_opaque_fields)]
pub struct CompTLFields {
    /// All TLField fields which map 1:1.
    comp_fields:*const CompTLField,

    /// All the function pointer types in the field.
    functions:Option<&'static TLFunctions >,

    comp_fields_len:u16,
}


unsafe impl Sync for CompTLFields {}
unsafe impl Send for CompTLFields {}


impl CompTLFields{
    pub const EMPTY:Self=Self::from_fields(rslice![]);

    /// Constructs a `TLFields`.
    pub const fn new(
        comp_fields:RSlice<'static,CompTLFieldRepr>,
        functions:Option<&'static TLFunctions >,
    )->Self{
        Self{
            comp_fields:comp_fields.as_ptr()
                as *const CompTLFieldRepr
                as *const CompTLField,
            comp_fields_len:comp_fields.len() as u16,

            functions,
        }
    }

    /// Constructs a `TLFields` with only fields.
    pub const fn from_fields(
        comp_fields:RSlice<'static,CompTLField>,
    )->Self{
        Self{
            comp_fields:comp_fields.as_ptr(),
            comp_fields_len:comp_fields.len() as u16,

            functions:None,
        }
    }

    pub fn comp_fields(&self)->&'static [CompTLField] {
        unsafe{
            slice::from_raw_parts(self.comp_fields,self.comp_fields_len as usize)
        }
    }

    pub fn comp_fields_rslice(&self)->RSlice<'static,CompTLField> {
        unsafe{
            RSlice::from_raw_parts(self.comp_fields,self.comp_fields_len as usize)
        }
    }

    /// The ammount of fields this represents
    pub fn len(&self)->usize{
        self.comp_fields_len as usize
    }
    
    pub fn expand(self,shared_vars:&'static SharedVars)->TLFields{
        TLFields{
            shared_vars,
            comp_fields:self.comp_fields_rslice(),
            functions:self.functions,        
        }
    }
}


///////////////////////////////////////////////////////////////////////////////

/// The layout of all field in a type definition.
#[repr(C)]
#[derive(Copy, Clone, StableAbi)]
pub struct TLFields {
    shared_vars:&'static SharedVars,

    comp_fields:RSlice<'static,CompTLField>,

    /// All the function pointer types in the field.
    functions:Option<&'static TLFunctions >,

}



impl TLFields{
    pub fn from_fields(
        comp_fields:&'static [CompTLField],
        shared_vars:&'static SharedVars,
    )->Self{
        Self{
            comp_fields:comp_fields.into(),
            shared_vars,
            functions:None,
        }
    }
	
    /// The ammount of fields this represents
    pub fn len(&self)->usize{
        self.comp_fields.len()
    }

    /// Whether this contains any fields
    pub fn is_empty(&self)->bool{
        self.comp_fields.is_empty()
    }

    pub fn get(&self,i:usize)->Option<TLField>{
        self.comp_fields.get(i)
            .map(|field| field.expand(i,self.functions,self.shared_vars) )
        
    }

    /// Gets an iterator over the fields.
    pub fn iter(&self)->TLFieldsIterator{
        TLFieldsIterator{
            shared_vars:self.shared_vars,
            comp_fields:self.comp_fields.as_slice().iter().enumerate(),
            functions:self.functions,
        }
    }
    
    /// Collects the fields into a `Vec<TLField>`.
    pub fn to_vec(&self)->Vec<TLField>{
        self.iter().collect()
    }
}


impl IntoIterator for TLFields {
    type IntoIter=TLFieldsIterator;
    type Item=TLField;

    #[inline]
    fn into_iter(self)->Self::IntoIter{
        self.iter()
    }
}

impl Debug for TLFields{
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        f.debug_list()
         .entries(self.iter())
         .finish()
    }
}


impl Display for TLFields {
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        for field in self.iter() {
            Display::fmt(&field,f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}


impl Eq for TLFields{}
impl PartialEq for TLFields{
    fn eq(&self,other:&Self)->bool{
        self.iter().eq(other.iter())
    }
}


///////////////////////////////////////////////////////////////////////////////


/**
An iterator over all the fields in a type definition.
*/
#[derive(Clone,Debug)]
pub struct TLFieldsIterator {
    shared_vars:&'static SharedVars,

    pub comp_fields:iter::Enumerate<slice::Iter<'static,CompTLField>>,

    /// All the function pointer types in the field.
    pub functions:Option<&'static TLFunctions >,

}

impl Iterator for TLFieldsIterator{
    type Item=TLField;

    fn next(&mut self)->Option<TLField>{
        self.comp_fields.next()
            .map(|(i,field)|{
                field.expand(i,self.functions,self.shared_vars)
            })
    }

    fn size_hint(&self)->(usize,Option<usize>){
        let len=self.comp_fields.len();
        (len,Some(len))
    }
    fn count(self) -> usize {
        self.comp_fields.len()
    }
}


impl std::iter::ExactSizeIterator for TLFieldsIterator{}


use super::*;


////////////////////////////////////////////////////////////////////////////////



/// The definition of
/// vtables and modules that can be extended in minor versions.
#[repr(C)]
#[derive(Copy, Clone, StableAbi)]
pub struct MonoTLPrefixType {
    /// The first field in the suffix,
    /// the index to the field after 
    /// the one to which `#[sabi(last_prefix_field)]` was applied to
    pub first_suffix_field:u8,
    /// Which fields in the prefix 
    /// (what comes at and before `#[sabi(last_prefix_field)]`)
    /// are conditionally accessible 
    /// (with the `#[sabi(accessible_if=" expression ")]` attribute).
    pub conditional_prefix_fields:FieldConditionality,
    /// All the fields of the prefix-type,even if they are inaccessible.
    pub fields: CompTLFields,
}


impl MonoTLPrefixType{
    pub fn expand(self,other:GenericTLPrefixType,shared_vars:&'static SharedVars)->TLPrefixType{
        TLPrefixType{
            first_suffix_field:self.first_suffix_field,
            conditional_prefix_fields:self.conditional_prefix_fields,
            fields:self.fields.expand(shared_vars),
            accessible_fields:other.accessible_fields,
        }
    }
}


/////////////////////////////////////////////////////


/// The part of TLPrefixType that may depend on generic parameters.
#[repr(C)]
#[derive(Copy, Clone, StableAbi)]
pub struct GenericTLPrefixType {
    /// Which fields are accessible when the prefix type is instantiated in 
    /// the same dynlib/binary.
    pub accessible_fields:FieldAccessibility,
}

/////////////////////////////////////////////////////


#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
pub struct TLPrefixType {
    /// The first field in the suffix,
    /// the index to the field after 
    /// the one to which `#[sabi(last_prefix_field)]` was applied to
    pub first_suffix_field:u8,
    /// Which fields in the prefix 
    /// (what comes at and before `#[sabi(last_prefix_field)]`)
    /// are conditionally accessible 
    /// (with the `#[sabi(accessible_if=" expression ")]` attribute).
    pub conditional_prefix_fields:FieldConditionality,
    /// All the fields of the prefix-type,even if they are inaccessible.
    pub fields: TLFields,

    /// Which fields are accessible when the prefix type is instantiated in 
    /// the same dynlib/binary.
    pub accessible_fields:FieldAccessibility,

}


impl Display for TLPrefixType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f,"first_suffix_field:{}",self.first_suffix_field)?;
        writeln!(
            f,
            "conditional_prefix_fields:\n    {:b}",
            self.conditional_prefix_fields.bits(),
        )?;
        writeln!(f,"fields:\n{}",self.fields.to_string().left_padder(4))?;
        write!(f,"accessible_fields:\n    ")?;
        f.debug_list()
         .entries(self.accessible_fields.iter_field_count(self.fields.len()))
         .finish()?;
        Ok(())
    }
}


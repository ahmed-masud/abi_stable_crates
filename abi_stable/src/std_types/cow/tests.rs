use super::*;

#[test]
fn from_into_cow(){
    macro_rules! from_tests {
        (
            $from:ident,
            Cow<$cow_param:ty>
        ) => ({
            {
                let borrowed_rcow=$from.into_c().piped(RCow::<$cow_param>::Borrowed);
                assert_eq!(
                    $from.piped(Cow::<$cow_param>::Borrowed).piped(RCow::from).as_borrowed(),
                    borrowed_rcow.as_borrowed(),
                );
            }
            {
                let owned_rcow=$from.to_owned().into_c().piped(RCow::<$cow_param>::Owned);
                assert_eq!(
                    $from.to_owned().piped(Cow::<$cow_param>::Owned).piped(RCow::from).as_owned(),
                    owned_rcow.as_owned(),
                );
            }
        })
    }
    
    {
        let line="what the heck";
        from_tests!{ line, Cow< str > }
    }
    {
        let list=[0,1,2,3];
        let list=&list[..];
        from_tests!{ list, Cow< [u8] > }
    }
    {
        let value=&1000u32;
        {
            let borrowed_rcow=value.piped(RCow::<u32>::Borrowed);
            assert_eq!(
                value.piped(Cow::Borrowed).piped(RCow::from).as_borrowed(),
                borrowed_rcow.as_borrowed(),
            );
        }
        {
            let owned_rcow=value.to_owned().piped(RCow::<u32>::Owned);
            assert_eq!(
                value.to_owned().piped(Cow::<u32>::Owned).piped(RCow::from).as_owned(),
                owned_rcow.as_owned(),
            );
        }
    }
}

#[test]
fn to_mut(){

    {
        let mut value=RCow::<u32>::Borrowed(&100);
        assert_eq!(*value, 100);
        *value.to_mut()=137;
        assert_eq!(*value, 137);
    }
    {
        let mut value=RCow::<str>::Borrowed("what".into_c());
        assert_eq!(&*value, "what");
        
        *value.to_mut()="the".into_(RString::T);
        assert_eq!(&*value, "the");
    }
    {
        let arr=[0,1,2,3];

        let mut value=RCow::<[u32]>::Borrowed( (&arr[..]).into() );
        assert_eq!(&*value, &arr[..] );
        *value.to_mut()=vec![99,100,101].into_c();
        assert_eq!(&*value, &[99,100,101][..]);
    }


}


#[test]
fn into_owned(){

    {
        let value=RCow::<u32>::Borrowed(&100);
        let value:u32=value.into_owned();
        assert_eq!(value,100);
    }
    {
        let value=RCow::<str>::Borrowed("what".into_c());
        let value:RString=value.into_owned();
        assert_eq!(&*value,"what");
    }
    {
        let arr=[0,1,2,3];
        let value=RCow::<[u32]>::Borrowed( (&arr[..]).into() );
        let value:RVec<u32>=value.into_owned();
        assert_eq!(&*value,&arr[..]);
    }


}



#[derive(Deserialize)]
#[serde(transparent)]
pub struct BorrowingRCowU8<'a>{
    #[serde(borrow,deserialize_with="deserialize_bytes")]
    cow:RCow<'a,[u8]>
}



#[test]
fn deserialize(){

    {// Borrowed string
        let json=r##" "what the hell" "##;
        let str_borr="what the hell".into_(RStr::T);

        let what:RCow<'_,str>=serde_json::from_str(json).unwrap();

        assert_eq!(
            what.as_borrowed(),
            Some(str_borr),
        );
    }
    {// Owned string
        let json=r##" "what \nthe hell" "##;
        let str_owned="what \nthe hell".into_(RString::T);

        let what:RCow<'_,str>=serde_json::from_str(json).unwrap();

        assert_eq!(
            what.as_owned(),
            Some(&str_owned),
        );
    }
    {// Owned list
        let json=r##" [0,1,2,3] "##;
        
        let what:RCow<'_,[u8]>=serde_json::from_str(json).unwrap();

        assert_eq!(
            what.as_owned(),
            Some(&vec![0,1,2,3].into_c()),
        );
    }
    {// Borrowed list,using bincode.
        let list=[0u8,1,2,3];
        let serialized=bincode::serialize(&list[..]).unwrap();
        
        let what:BorrowingRCowU8<'_>=bincode::deserialize(&serialized[..]).unwrap();

        assert_eq!(
            what.cow.as_borrowed(),
            Some((&list[..]).into_c()),
        );
    }
    {// Owned value
        let json=r##" 1000 "##;

        let what:RCow<'_,u16>=serde_json::from_str(json).unwrap();

        assert_eq!(
            what.as_owned(),
            Some(&1000),
        );

    }


}
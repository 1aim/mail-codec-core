pub use soft_ascii_string::{ SoftAsciiStr as _SoftAsciiStr };


/// Defines a new header types with given type name, filed name and component
/// Note that the name is not checked/validated, it has to be ascii, a valid
/// header field name AND has to comply with the naming schema (each word
/// seperated by `'-'` starts with a capital letter and no cappital letter
/// follow, e.g. "Message-Id" is ok but "Message-ID" isn't).
///
/// This macro will create a test which will check if the used field names
/// are actually valid and appears only once (_per def_header macro call_)
/// so as long as test's are run any invalid name will be found.
///
/// Note that even if a invalid name was used and test where ignored/not run
/// this will _not_ cause an rust safety issue, but can still cause bugs under
/// some circumstances (e.g. if you have multiple differing definitions of the
/// same header with different spelling (at last one failed the test) like e.g.
/// when you override default implementations of fields).
///
/// The macros expects following items:
///
/// 1. `test_name`, which is the name the auto-generated test will have
/// 2. `scope`, the scope all components are used with, this helps with some
///    name collisions. Use `self` to use the current scope.
/// 3. a list of header definitions consisting of:
///
///    1. `1` or `+`, stating wether the header can appear at most one time (1) or more times (+)
///       (not that only `Date`+`From` are required headers, no other can be made into such)
///    2. `<typename>` the name the type of the header will have, i.e. the name of a zero-sized
///       struct which will be generated
///    3. `unchecked` a hint to make people read the documentation and not forget the the
///       folowing data is `unchecked` / only vaidated in the auto-generated test
///    4. `"<header_name>"` the header name in a syntax using `'-'` to serperate words,
///       also each word has to start with a capital letter and be followed by lowercase
///       letters additionaly to being a valid header field name. E.g. "Message-Id" is
///       ok, but "Message-ID" is not. (Note that header field name are on itself ignore
///       case, but by enforcing a specific case in the encoder equality checks can be
///       done on byte level, which is especially usefull for e.g. placing them as keys
///       into a HashMap or for performance reasons.
///    5. `<component>` the name of the type to use ing `scope` a the component type of
///       the header. E.g. `Unstructured` for an unstructured header field (which still
///       support Utf8 through encoded words)
///    6. `None`/`<ident>`, None or the name of a validator function (if there is one).
///       This function is called before encoding with the header map as argument, and
///       can cause a error. Use this to enfore contextual limitations like having a
///       `From` with multiple mailboxes makes `Sender` an required field.
///
/// # Example
///
/// ```norun
/// def_headers! {
///     // the name of the auto-generated test
///     test_name: validate_header_names,
///     // the scope from which all components should be imported
///     // E.g. `DateTime` refers to `components::DateTime`.
///     scope: components,
///     // definitions of the headers
///     1 Date,     unchecked { "Date"          },  DateTime,       None,
///     1 From,     unchecked { "From"          },  MailboxList,    validator_from,
///     1 Subject,  unchecked { "Subject"       },  Unstructured,   None,
///     + Comments, unchecked { "Comments"      },  Unstructured,   None,
/// }
/// ```
#[macro_export]
macro_rules! def_headers {
    (
        test_name: $tn:ident,
        scope: $scope:ident,
        $($multi:tt $name:ident, unchecked { $hname:tt }, $component:ident, $validator:ident),+
    ) => (
        $(
            pub struct $name;

            impl $crate::headers::Header for  $name {
                const MAX_COUNT_EQ_1: bool = def_headers!(_PRIV_boolify $multi);
                type Component = $scope::$component;

                fn name() -> $crate::headers::HeaderName {
                    let as_str: &'static str = $hname;
                    $crate::headers::HeaderName::from_ascii_unchecked( as_str )
                }

                const CONTEXTUAL_VALIDATOR:
                    Option<fn(&$crate::headers::HeaderMap) -> $crate::error::Result<()>> =
                        def_headers!{ _PRIV_mk_validator $validator };
            }
        )+

        $(
            def_headers!{ _PRIV_impl_marker $multi $name }
        )+

        //TODO warn if header type name and header name diverges
        // (by stringifying the type name and then ziping the
        //  array of type names with header names removing
        //  "-" from the header names and comparing them to
        //  type names)


        #[cfg(test)]
        const HEADER_NAMES: &[ &str ] = &[ $(
            $hname
        ),+ ];

        #[test]
        fn $tn() {
            use std::collections::HashSet;
            use $crate::codec::EncodableInHeader;

            let mut name_set = HashSet::new();
            for name in HEADER_NAMES {
                if !name_set.insert(name) {
                    panic!("name appears more than one time in same def_headers macro: {:?}", name);
                }
            }
            fn can_be_trait_object<EN: EncodableInHeader>( v: Option<&EN> ) {
                let _ = v.map( |en| en as &EncodableInHeader );
            }
            $(
                can_be_trait_object::<$scope::$component>( None );
            )+
            for name in HEADER_NAMES {
                let res = $crate::headers::HeaderName::validate_name(
                    $crate::headers::_SoftAsciiStr::from_str(name).unwrap()
                );
                if res.is_err() {
                    panic!( "invalid header name: {:?} ({:?})", name, res.unwrap_err() );
                }
            }
        }
    );
    (_PRIV_mk_validator None) => ({ None });
    (_PRIV_mk_validator $validator:ident) => ({ Some($validator) });
    (_PRIV_boolify +) => ({ false });
    (_PRIV_boolify 1) => ({ true });
    (_PRIV_boolify $other:tt) => (
        compiler_error!( "only `1` (for singular) or `+` (for multiple) are valid" )
    );
    ( _PRIV_impl_marker + $name:ident ) => (
        //do nothing here
    );
    ( _PRIV_impl_marker 1 $name:ident ) => (
        impl $crate::headers::SingularHeaderMarker for $name {}
    );
}
//
//
//use components;
//use self::validators::{
//    from as validator_from,
//    resent_any as validator_resent_any
//};
//def_headers! {
//    test_name: validate_header_names,
//    scope: components,
//    //RFC 5322:
//    1 Date,                    unchecked { "Date"          },  DateTime,       None,
//    1 From,                    unchecked { "From"          },  MailboxList,    validator_from,
//    1 Sender,                  unchecked { "Sender"        },  Mailbox,        None,
//    1 ReplyTo,                 unchecked { "Reply-To"      },  MailboxList,    None,
//    1 To,                      unchecked { "To"            },  MailboxList,    None,
//    1 Cc,                      unchecked { "Cc"            },  MailboxList,    None,
//    1 Bcc,                     unchecked { "Bcc"           },  MailboxList,    None,
//    1 MessageId,               unchecked { "Message-Id"    },  MessageID,      None,
//    1 InReplyTo,               unchecked { "In-Reply-To"   },  MessageIDList,  None,
//    1 References,              unchecked { "References"    },  MessageIDList,  None,
//    1 Subject,                 unchecked { "Subject"       },  Unstructured,   None,
//    + Comments,                unchecked { "Comments"      },  Unstructured,   None,
//    + Keywords,                unchecked { "Keywords"      },  PhraseList,     None,
//    + ResentDate,              unchecked { "Resent-Date"   },  DateTime,       validator_resent_any,
//    + ResentFrom,              unchecked { "Resent-From"   },  MailboxList,    validator_resent_any,
//    + ResentSender,            unchecked { "Resent-Sender" },  Mailbox,        validator_resent_any,
//    + ResentTo,                unchecked { "Resent-To"     },  MailboxList,    validator_resent_any,
//    + ResentCc,                unchecked { "Resent-Cc"     },  MailboxList,    validator_resent_any,
//    + ResentBcc,               unchecked { "Resent-Bcc"    },  OptMailboxList, validator_resent_any,
//    + ResentMsgId,             unchecked { "Resent-Msg-Id" },  MessageID,      validator_resent_any,
//    + ReturnPath,              unchecked { "Return-Path"   },  Path,           None,
//    + Received,                unchecked { "Received"      },  ReceivedToken,  None,
//    //RFC 2045:
//    1 ContentType,             unchecked { "Content-Type"              }, Mime,             None,
//    1 ContentId,               unchecked { "Content-Id"                }, ContentID,        None,
//    1 ContentTransferEncoding, unchecked { "Content-Transfer-Encoding" }, TransferEncoding, None,
//    1 ContentDescription,      unchecked { "Content-Description"       }, Unstructured,     None,
//    //RFC 2183:
//    1 ContentDisposition,      unchecked { "Content-Disposition"       }, Disposition, None
//}
//
//mod validators {
//    use std::collections::HashMap;
//
//    use error::*;
//    use codec::EncodableInHeader;
//    use headers::{ HeaderMap, Header, HeaderName };
//    use super::{ From, ResentFrom, Sender, ResentSender, ResentDate };
//
//    pub fn from(map: &HeaderMap) -> Result<()> {
//        // Note: we do not care about the quantity of From bodies,
//        // nor "other" From bodies
//        // (which do not use a MailboxList and we could
//        //  therefore not cast to it,
//        // whatever header put them in has also put in
//        // this bit of validation )
//        let needs_sender =
//            map.get(From).map(|bodies|
//                bodies.filter_map(|res| res.ok()).any(|list| list.len() > 1 )
//            ).unwrap_or(false);
//
//        if needs_sender && !map.contains(Sender) {
//            bail!("if a multi-mailbox From is used Sender has to be specified");
//        }
//        Ok(())
//    }
//
//    fn validate_resent_block<'a>(
//            block: &HashMap<HeaderName, &'a EncodableInHeader>
//    ) -> Result<()> {
//        if !block.contains_key(&ResentDate::name()) {
//            bail!("each reasond block must have a Resent-Date field");
//        }
//        let needs_sender =
//            //no Resend-From? => no problem
//            block.get(&ResentFrom::name())
//                //can't cast? => not my problem/responsibility
//                .and_then(|tobj| tobj.downcast_ref::<<ResentFrom as Header>::Component>())
//                .map(|list| list.len() > 1)
//                .unwrap_or(false);
//
//        if needs_sender && !block.contains_key(&ResentSender::name()) {
//            bail!("each resent block containing a multi-mailbox Resent-From needs to have a Resent-Sender field too")
//        }
//        Ok(())
//    }
//
//    pub fn resent_any(map: &HeaderMap) -> Result<()> {
//        let resents = map
//            .iter()
//            .filter(|&(name, _)| name.as_str().starts_with("Resent-"));
//
//        let mut block = HashMap::new();
//        for (name, content) in resents {
//            if block.contains_key(&name) {
//                validate_resent_block(&block)?;
//                //create new block
//                block = HashMap::new();
//            }
//            block.insert(name, content);
//        }
//        validate_resent_block(&block)
//    }
//}
//
//#[cfg(test)]
//mod test {
//    use components::DateTime;
//    use headers::{
//        HeaderMap,
//        From, ResentFrom, ResentTo, ResentDate,
//        Sender, ResentSender, Subject
//    };
//
//    #[test]
//    fn from_validation_normal() {
//        let mut map = HeaderMap::new();
//        map.insert(From, [("Mr. Peté", "pete@nixmail.nixdomain")]).unwrap();
//        map.insert(Subject, "Ok").unwrap();
//
//        assert_ok!(map.use_contextual_validators());
//    }
//    #[test]
//    fn from_validation_multi_err() {
//        let mut map = HeaderMap::new();
//        map.insert(From, (
//            ("Mr. Peté", "nixperson@nixmail.nixdomain"),
//            "a@b.c"
//        )).unwrap();
//        map.insert(Subject, "Ok").unwrap();
//
//        assert_err!(map.use_contextual_validators());
//    }
//
//    #[test]
//    fn from_validation_multi_ok() {
//        let mut map = HeaderMap::new();
//        map.insert(From, (
//            ("Mr. Peté", "nixperson@nixmail.nixdomain"),
//            "a@b.c"
//        )).unwrap();
//        map.insert(Sender, "abx@d.e").unwrap();
//        map.insert(Subject, "Ok").unwrap();
//
//        assert_ok!(map.use_contextual_validators());
//    }
//
//    #[test]
//    fn resent_no_date_err() {
//        let mut map = HeaderMap::new();
//        map.insert(ResentFrom,["a@b.c"]).unwrap();
//        assert_err!(map.use_contextual_validators());
//    }
//
//    #[test]
//    fn resent_with_date() {
//        let mut map = HeaderMap::new();
//        map.insert(ResentFrom,["a@b.c"]).unwrap();
//        map.insert(ResentDate, DateTime::now()).unwrap();
//        assert_ok!(map.use_contextual_validators());
//    }
//
//    #[test]
//    fn resent_no_date_err_second_block() {
//        let mut map = HeaderMap::new();
//        map.insert(ResentDate, DateTime::now()).unwrap();
//        map.insert(ResentFrom,["a@b.c"]).unwrap();
//        map.insert(ResentTo, ["e@f.d"]).unwrap();
//        map.insert(ResentFrom, ["ee@ee.e"]).unwrap();
//
//        assert_err!(map.use_contextual_validators());
//    }
//
//    #[test]
//    fn resent_with_date_second_block() {
//        let mut map = HeaderMap::new();
//        map.insert(ResentDate, DateTime::now()).unwrap();
//        map.insert(ResentFrom,["a@b.c"]).unwrap();
//        map.insert(ResentTo, ["e@f.d"]).unwrap();
//        map.insert(ResentFrom, ["ee@ee.e"]).unwrap();
//        map.insert(ResentDate, DateTime::now()).unwrap();
//
//        assert_ok!(map.use_contextual_validators());
//    }
//
//    #[test]
//    fn resent_multi_mailbox_from_no_sender() {
//        let mut map = HeaderMap::new();
//        map.insert(ResentDate, DateTime::now()).unwrap();
//        map.insert(ResentFrom, ["a@b.c","e@c.d"]).unwrap();
//
//        assert_err!(map.use_contextual_validators());
//    }
//
//    #[test]
//    fn resent_multi_mailbox_from_with_sender() {
//        let mut map = HeaderMap::new();
//        map.insert(ResentDate, DateTime::now()).unwrap();
//        map.insert(ResentFrom, ["a@b.c","e@c.d"]).unwrap();
//        map.insert(ResentSender, "a@b.c").unwrap();
//
//        assert_ok!(map.use_contextual_validators());
//    }
//}
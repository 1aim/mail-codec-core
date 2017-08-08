use std::mem::replace;
use std::result::{ Result as StdResult };

use serde;
use serde::ser::Error;

use components::{ Mailbox, MessageID };
use mail::resource::Resource;

use super::resource::{
    Embeddings, Attachments,
    EmbeddingInMail,
};
use super::context::{
    Context
};



pub trait DataInterface: serde::Serialize {

    fn find_externals<F1,F2>( &mut self, emb: F1, att: F2 )
        where F1: FnMut( &mut EmbeddingInData),
              F2: FnMut( &mut AttachmentInData);

    fn see_from_mailbox(&mut self, mbox: &Mailbox );
    fn see_to_mailbox(&mut self, mbox: &Mailbox );
}



#[derive(Debug, Serialize)]
pub struct EmbeddingInData(InnerEmbedding);
#[derive(Debug)]
enum InnerEmbedding {
    AsValue( Resource ),
    AsContentId( MessageID )
}

impl EmbeddingInData {
    pub fn new( resource: Resource ) -> Self {
        EmbeddingInData( InnerEmbedding::AsValue( resource ) )
    }

    //TODO access methods for the AsValue variant

    fn swap_with_content_id( &mut self, cid: MessageID ) -> Option<Resource> {
        use self::InnerEmbedding::*;
        match replace( &mut self.0, AsContentId( cid ) ) {
            //TODO warn this is definitily a bug
            AsContentId( _cid ) => None,
            AsValue( value ) => Some( value )
        }
    }
}

impl serde::Serialize for InnerEmbedding {
    fn serialize<S>( &self, serializer: S ) -> StdResult<S::Ok, S::Error>
        where S: serde::Serializer
    {
        use serde::ser::Error;
        use self::InnerEmbedding::*;
        match *self {
            AsValue( .. ) => Err( S::Error::custom( concat!(
                "embeddings can be serialized as content id, not as value, ",
                "preprocess_data should have ben called before" ) ) ),
            //FIXME potentialy use cid encode as string!
            AsContentId( ref cid ) => cid.serialize( serializer )
        }
    }
}

//FIXME PathBuf => FileSource
#[derive(Debug, Serialize)]
pub struct AttachmentInData(InnerAttachment );
#[derive(Debug)]
enum InnerAttachment {
    AsValue( Resource ),
    /// the resource was moved out of data, to be added to the
    /// mail attachments
    Moved
}

impl AttachmentInData {
    pub fn new( resource: Resource ) -> Self {
        AttachmentInData( InnerAttachment::AsValue( resource ) )
    }

    //TODO access methods for the AsValue variant

    fn move_out( &mut self ) -> Option<Resource> {
        use self::InnerAttachment::*;
        match replace( &mut self.0, InnerAttachment::Moved ) {
            AsValue( value ) => Some( value ),
            //TODO warn as this is likely a bug
            Moved => None
        }
    }
}

impl serde::Serialize for InnerAttachment {
    fn serialize<S>( &self, serializer: S ) -> StdResult<S::Ok, S::Error>
        where S: serde::Serializer
    {
        use self::InnerAttachment::*;
        match *self {
            AsValue( .. ) => Err( S::Error::custom( concat!(
                "only moved attachments can be serialized, ",
                "preprocess_data should have ben called before" ) ) ),
            Moved => serializer.serialize_none()
        }
    }
}

pub fn preprocess_data<C: Context, D: DataInterface>( ctx: &C, data: &mut D )
    -> (Embeddings, Attachments)
{
    let mut embeddings = Vec::new();
    let mut attachments = Vec::new();
    data.find_externals(
        |embedding| {
            let new_cid = ctx.new_content_id();
            if let Some( embedding ) = embedding.swap_with_content_id( new_cid.clone() ) {
                embeddings.push( EmbeddingInMail {
                    content_id: new_cid,
                    resource: embedding
                } )
            }
        },
        |attachment| {
            if let Some( attachment ) = attachment.move_out() {
                attachments.push( attachment )
            }
        }
    );

    (embeddings, attachments)
}
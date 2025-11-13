use storage_system::Storage;

pub struct MessageService<S: Storage> {
    storage: S,
}

impl<S: Storage> MessageService<S> {
    // async fn save(&self, storage_message: StorageMessage) -> Result<Uuid, StorageError> {
    //     let db_message: storage_system::Message = storage_message.into_storage_format();
    //     self.storage.save_message(&db_message).await
    // }
    
    // async fn get_conversation_messages(&self, convo_id: Uuid) -> Result<Vec<StorageMessage>, StorageError> {
    //     let storage_message = self.storage.get_messages(convo_id).await?;
    //     Ok(storage_message.iter().map(|x| x.to_owned().into()).collect())
    // }
}
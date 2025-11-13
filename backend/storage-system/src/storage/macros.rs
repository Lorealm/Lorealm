#[macro_export]
macro_rules! impl_storage_defaults {
    ($type:ty) => {
        impl $crate::storage::ConversationStorage for $type {}
        impl $crate::storage::MessageStorage for $type {}
        impl $crate::storage::UserStorage for $type {}
        impl $crate::storage::CharacterStorage for $type {}  // Add this
    };
}

/// Declare which features a storage backend supports
#[macro_export]
macro_rules! impl_storage_capabilities {
    ($type:ty, { $($feature:ident: $value:expr),* $(,)? }) => {
        impl $crate::storage::StorageCapabilities for $type {
            $(
                fn $feature(&self) -> bool {
                    $value
                }
            )*
        }
    };
}
use lorealm_storage::{
    TokenizerStorage,
    models::{Snowflake, Tokenizer},
};

use crate::{errors::TokenizerError, tests::Harness};

async fn byte_tokenizer(h: &Harness, is_public: bool) -> Snowflake {
    use base64::Engine;
    let ranks: String = (0..=255u8)
        .map(|b| {
            let token = base64::engine::general_purpose::STANDARD.encode([b]);
            format!("{token} {b}\n")
        })
        .collect();
    let blob = h.blobs.put(ranks.as_bytes()).await.unwrap();
    let id = h.snow.generate();
    h.store
        .save_tokenizer(&Tokenizer {
            id,
            name: "bytes".into(),
            blob_uri: blob.to_uri(),
            owner_id: h.user,
            is_public,
        })
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn owner_can_load_their_private_tokenizer() {
    let h = Harness::new().await;
    let tok = byte_tokenizer(&h, false).await;

    h.tokenizers
        .load_kitoken_tokenizer(tok, h.user)
        .await
        .expect("owner loads");
}

#[tokio::test]
async fn anyone_can_load_a_public_tokenizer() {
    let h = Harness::new().await;
    let tok = byte_tokenizer(&h, true).await;

    let tokens = h
        .tokenizers
        .tokenize("hi".into(), tok, h.stranger)
        .await
        .expect("public tokenizer loads for a non-owner");
    assert_eq!(tokens.len(), 2);
}

#[tokio::test]
async fn shared_users_can_load_a_private_tokenizer() {
    let h = Harness::new().await;
    let tok = byte_tokenizer(&h, false).await;
    h.store.share_tokenizer(tok, h.stranger).await.unwrap();

    h.tokenizers
        .load_kitoken_tokenizer(tok, h.stranger)
        .await
        .expect("shared user loads");
}

#[tokio::test]
async fn others_cannot_load_a_private_tokenizer() {
    let h = Harness::new().await;
    let tok = byte_tokenizer(&h, false).await;
    h.tokenizers
        .load_kitoken_tokenizer(tok, h.user)
        .await
        .unwrap();

    let Err(err) = h.tokenizers.load_kitoken_tokenizer(tok, h.stranger).await else {
        panic!("stranger is refused");
    };
    assert!(
        matches!(err, TokenizerError::NotFound(id) if id == tok),
        "{err:?}"
    );
}

use uuid::Uuid;

#[derive(Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub id: Uuid,
    pub timestamp: i64,
    pub source: String
}
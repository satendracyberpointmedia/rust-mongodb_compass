use mongodb::{Cursor, bson::Document};
use futures::StreamExt;

pub struct CursorSession {
    pub cursor: Cursor<Document>,
    pub batch_size: usize,
}

impl CursorSession {
    pub async fn next_batch(&mut self) -> Vec<Document> {
        let mut batch = Vec::new();
        for _ in 0..self.batch_size {
            if let Some(doc) = self.cursor.next().await {
                if let Ok(d) = doc {
                    batch.push(d);
                }
            } else {
                break;
            }
        }
        batch
    }
}

use mongodb::{Cursor, bson::Document};
use futures::StreamExt;

pub struct CursorSession {
    pub cursor: Cursor<Document>,
    pub batch_size: usize,
}

impl CursorSession {
    pub async fn next_batch(&mut self) -> Vec<Document> {
        let mut batch = Vec::with_capacity(self.batch_size);
        for _ in 0..self.batch_size {
            match self.cursor.next().await {
                Some(Ok(doc)) => batch.push(doc),
                Some(Err(_)) => {
                    // Log error but continue with what we have
                    break;
                }
                None => break,
            }
        }
        batch
    }
    
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size.max(1).min(1000); // Clamp between 1 and 1000
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub rating: i64,
    pub comment: String,
}

#[derive(Debug, Serialize)]
pub struct ReviewResponse {
    pub id: String,
    pub product_id: String,
    pub user_id: String,
    pub rating: i64,
    pub comment: String,
}

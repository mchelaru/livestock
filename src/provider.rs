use chrono::NaiveDate;
use std::fmt::Debug;

pub(crate) trait Provider {
    type ErrorType: Debug;

    fn get_provider_name(&self) -> String;
    async fn download_price(
        &self,
        name: String,
        date: NaiveDate,
    ) -> Result<(String, NaiveDate, f64), Self::ErrorType>;
}

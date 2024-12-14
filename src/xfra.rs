use std::{collections::HashMap, sync::Mutex};

use chrono::NaiveDate;
use reqwest;

use crate::provider::Provider;

/// Get the data from XFRA API
/// E.g. https://api.boerse-frankfurt.de/v1/data/price_information/single?isin=SOME_ISIN_HERE&mic=XFRA
#[derive(Debug)]
pub struct XFRA {
    /// because the XFRA API doesn't allow yet to query a specific date, we use
    /// this cache in order to avoid redundant queries
    cache: Mutex<HashMap<String, f64>>,
}

impl XFRA {
    pub(crate) fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::default()),
        }
    }
}

impl Provider for XFRA {
    type ErrorType = std::io::Error;

    fn get_provider_name(&self) -> String {
        "XFRA".to_owned()
    }

    /// Downloads the price for a given ISIN
    async fn download_price(
        &self,
        isin: String,
        date: NaiveDate,
    ) -> Result<(String, NaiveDate, f64), Self::ErrorType> {
        if let Some(cache_result) = self.cache.lock().unwrap().get(&isin) {
            return Ok((isin, date, *cache_result));
        }

        // TODO: use a keepalive http connection instead of doing 3-way handshake for each request
        let url = format!(
            "https://api.boerse-frankfurt.de/v1/data/price_information/single?isin={isin}&mic=XFRA"
        );
        let response = reqwest::get(url)
            .await
            .map_err(|_| {
                std::io::Error::other(format!("XFRA: Invalid response while querying for {isin}"))
            })?
            .text()
            .await
            .map_err(|_| {
                std::io::Error::other(format!(
                    "XFRA: Invalid text in response while querying for {isin}"
                ))
            })?;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        let price = match json.get("lastPrice") {
            Some(value) => value,
            None => {
                return Err(std::io::Error::other(format!(
                    "XFRA: error retrieving the lastPrice key for {isin}"
                )));
            }
        };

        let mut float_price =
            serde_json::from_value(price.clone()).expect("XFRA: error transforming price to float");

        // divide the price by 100 in the case the price is traded in percent
        match json.get("tradedInPercent") {
            Some(value) => match serde_json::from_value(value.clone()) {
                Ok(boolean_value) => {
                    if boolean_value {
                        float_price /= 100.0;
                    }
                }
                Err(_) => {}
            },
            None => {}
        }

        self.cache.lock().unwrap().insert(isin.clone(), float_price);
        Ok((isin, date, float_price))
    }
}

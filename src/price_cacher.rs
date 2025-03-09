use std::sync::{Arc, Mutex};

use chrono::NaiveDate;
use dirs::home_dir;
use rusqlite::{self, Connection};

use crate::provider::Provider;

#[derive(Debug)]
pub struct PriceCacher {
    connection: Mutex<Connection>,
}

impl PriceCacher {
    pub(crate) fn new() -> Self {
        let home = home_dir().unwrap().to_str().unwrap().to_owned();
        let connection = rusqlite::Connection::open(home + "/.livestock.sql").unwrap();
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS cache (
                provider TEXT NOT NULL,
                symbol TEXT NOT NULL,
                date TEXT NOT NULL,
                price REAL NOT NULL
                )",
                (),
            )
            .unwrap();
        Self {
            connection: Mutex::new(connection),
        }
    }

    fn get_provider_name(provider: &Provider) -> String {
        match provider {
            Provider::YFinance(yfinance) => yfinance.get_provider_name(),
            Provider::Xfra(xfra) => xfra.get_provider_name(),
        }
    }

    pub async fn download_price(
        &self,
        provider: Arc<Provider>,
        ticker: String,
        date: NaiveDate,
    ) -> Result<(String, NaiveDate, f64), std::io::Error> {
        const DATE_FORMATTER: &str = "%Y-%m-%d";
        // try matching it in the cache
        let provider_name = Self::get_provider_name(&provider);
        let cached_price: rusqlite::Result<f64> =
            self.connection.lock().unwrap().query_row_and_then(
                "SELECT price FROM cache WHERE provider=?1 and symbol=?2 and date=?3",
                (
                    provider_name.clone(),
                    ticker.clone(),
                    date.format(DATE_FORMATTER).to_string(),
                ),
                |row| row.get(0),
            );
        match cached_price {
            Ok(price) => Ok((ticker, date, price)),
            Err(_) => {
                // not found in the cache, try resolving it
                let result = provider.download_price(&ticker, date).await?;
                // cache the result
                let _ = self.connection.lock().unwrap().execute(
                    "INSERT INTO cache (provider, symbol, date, price) VALUES(?1, ?2, ?3, ?4)",
                    (
                        provider_name,
                        result.0.clone(),
                        result.1.format(DATE_FORMATTER).to_string(),
                        result.2,
                    ),
                );
                Ok(result)
            }
        }
    }
}

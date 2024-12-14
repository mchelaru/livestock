use chrono::NaiveDate;

use crate::provider::Provider;
use crate::{xfra::XFRA, yfinance::YFinance};

#[derive(Debug)]
pub(crate) enum Providers {
    YFinance(YFinance),
    XFRA(XFRA),
}

impl Providers {
    pub(crate) async fn download_price(
        &self,
        name: &str,
        date: NaiveDate,
    ) -> Result<(String, NaiveDate, f64), std::io::Error> {
        match self {
            Providers::YFinance(yfinance) => yfinance
                .download_price(name.to_owned(), date)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Providers::XFRA(xfra) => xfra
                .download_price(name.to_owned(), date)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        }
    }

    pub(crate) fn get_provider_name(&self) -> String {
        match self {
            Providers::YFinance(yfinance) => yfinance.get_provider_name(),
            Providers::XFRA(xfra) => xfra.get_provider_name(),
        }
    }

    pub(crate) fn build(typestr: &str) -> Option<Self> {
        match typestr {
            "Yahoo" => return Some(Providers::YFinance(YFinance::new(false))),
            "XFRA" => return Some(Providers::XFRA(XFRA::new())),
            _ => None,
        }
    }
}

use chrono::NaiveDate;

use crate::provider::Provider;
use crate::{xfra::Xfra, yfinance::YFinance};

#[derive(Debug)]
pub(crate) enum Providers {
    YFinance(YFinance),
    Xfra(Xfra),
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
            Providers::Xfra(xfra) => xfra
                .download_price(name.to_owned(), date)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        }
    }

    pub(crate) fn get_provider_name(&self) -> String {
        match self {
            Providers::YFinance(yfinance) => yfinance.get_provider_name(),
            Providers::Xfra(xfra) => xfra.get_provider_name(),
        }
    }

    pub(crate) fn build(typestr: &str) -> Option<Self> {
        match typestr {
            "Yahoo" => Some(Providers::YFinance(YFinance::new(false))),
            "XFRA" => Some(Providers::Xfra(Xfra::new())),
            _ => None,
        }
    }
}

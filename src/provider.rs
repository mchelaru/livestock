use chrono::NaiveDate;

use crate::{xfra::Xfra, yfinance::YFinance};

#[derive(Debug)]
pub(crate) enum Provider {
    YFinance(YFinance),
    Xfra(Xfra),
}

impl Provider {
    pub(crate) async fn download_price(
        &self,
        name: &str,
        date: NaiveDate,
    ) -> Result<(String, NaiveDate, f64), std::io::Error> {
        match self {
            Provider::YFinance(yfinance) => yfinance
                .download_price(name.to_owned(), date)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Provider::Xfra(xfra) => xfra
                .download_price(name.to_owned(), date)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        }
    }

    pub(crate) fn get_provider_name(&self) -> String {
        match self {
            Provider::YFinance(yfinance) => yfinance.get_provider_name(),
            Provider::Xfra(xfra) => xfra.get_provider_name(),
        }
    }

    pub(crate) fn build(typestr: &str) -> Option<Self> {
        match typestr {
            "Yahoo" => Some(Provider::YFinance(YFinance::new(false))),
            "XFRA" => Some(Provider::Xfra(Xfra::new())),
            _ => None,
        }
    }
}

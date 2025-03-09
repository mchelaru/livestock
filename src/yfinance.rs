use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
    sync::Mutex,
};

use chrono::NaiveDate;
use yahoo_finance_api::{
    self as yf,
    time::{Duration, OffsetDateTime},
    YahooConnector,
};

#[repr(transparent)]
struct DebugHolder<T> {
    pub(crate) inner: T,
}

impl<T> Debug for DebugHolder<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugHolder")
            .field("inner", &std::any::type_name::<T>())
            .finish()
    }
}

#[derive(Debug)]
pub struct YFinanceError {
    pub reason: String,
    pub inner: yf::YahooError,
}

impl YFinanceError {
    pub fn new(ticker: &str, date: &NaiveDate, original_error: yf::YahooError) -> Self {
        Self {
            reason: format!("YFinance: error downloading data for {ticker} on date {date}"),
            inner: original_error,
        }
    }
}

impl Display for YFinanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "YFinanceError: {0};\nInner error: {1:#?}",
            self.reason, self.inner
        )
    }
}

impl Error for YFinanceError {}

#[derive(Debug)]
pub struct YFinance {
    provider: tokio::sync::Mutex<DebugHolder<YahooConnector>>,
    ticker_resolver_cache: Mutex<HashMap<String, String>>,
    debug: bool,
}

impl YFinance {
    pub(crate) fn new(debug: bool) -> Self {
        Self {
            provider: tokio::sync::Mutex::new(DebugHolder {
                inner: yf::YahooConnector::new().unwrap(),
            }),
            ticker_resolver_cache: Mutex::new(HashMap::default()),
            debug,
        }
    }

    async fn resolve_symbol(&self, ticker: &str) -> Result<String, YFinanceError> {
        if let Some(cache_result) = self.ticker_resolver_cache.lock().unwrap().get(ticker) {
            return Ok(cache_result.clone());
        }

        let search_result = self
            .provider
            .lock()
            .await
            .inner
            .search_ticker(ticker)
            .await
            .map_err(|err| {
                YFinanceError::new(ticker, &chrono::Utc::now().naive_utc().into(), err)
            })?;

        if search_result.quotes.is_empty() {
            eprintln!("Error matching symbol {ticker}");
            return Err(YFinanceError::new(
                ticker,
                &chrono::Utc::now().naive_utc().into(),
                yahoo_finance_api::YahooError::DataInconsistency,
            ));
        } else if search_result.quotes.len() > 1 && self.debug {
            eprintln!("Multiple matches for {ticker} - using the first match");
            eprintln!(
                "{}",
                search_result
                    .quotes
                    .iter()
                    .map(|q| q.symbol.clone())
                    .reduce(|mut acc, s| {
                        acc.push(' ');
                        acc.push_str(&s);
                        acc
                    })
                    .unwrap()
            );
        }
        self.ticker_resolver_cache
            .lock()
            .unwrap()
            .insert(ticker.to_owned(), search_result.quotes[0].symbol.clone());
        Ok(search_result.quotes[0].symbol.clone())
    }

    pub fn get_provider_name(&self) -> String {
        "Yahoo! Finance".to_owned()
    }

    pub async fn download_price(
        &self,
        ticker: String,
        date: NaiveDate,
    ) -> Result<(String, NaiveDate, f64), YFinanceError> {
        let yahoo_symbol = self.resolve_symbol(&ticker).await?;
        let date_time = date.and_hms_opt(0, 0, 0).unwrap();

        let start = OffsetDateTime::from_unix_timestamp(date_time.and_utc().timestamp()).unwrap();
        let end = start.checked_add(Duration::days(1)).unwrap();

        let quote = self
            .provider
            .lock()
            .await
            .inner
            .get_quote_history_interval(&yahoo_symbol, start, end, "1d")
            .await
            .map_err(|err| YFinanceError::new(&ticker, &date, err))?;
        Ok((ticker, date, quote.last_quote().unwrap().close))
    }
}

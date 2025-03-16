use std::collections::hash_map::Entry;
use std::hash::Hash;
use std::io::Error;
use std::{collections::HashMap, sync::Arc};

use chrono::{Days, NaiveDate};
use tokio::task::JoinSet;

use crate::{price_cacher::PriceCacher, Provider};

#[derive(Debug, Clone)]
pub struct Instrument {
    name: String,
    quantity: u32,
    provider: Arc<Provider>,
}

impl PartialEq for Instrument {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.quantity == other.quantity
            && self.provider.get_provider_name() == other.provider.get_provider_name()
    }
}

impl Eq for Instrument {}

impl Hash for Instrument {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.quantity.hash(state);
        self.provider.get_provider_name().hash(state);
    }
}

impl Instrument {
    pub fn get_provider(&self) -> Arc<Provider> {
        Arc::clone(&self.provider)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

pub struct Portfolio {
    portfolio: HashMap<Instrument, HashMap<NaiveDate, f64>>,
    price_cacher: Arc<PriceCacher>,
    request_join_handles: JoinSet<Result<(Instrument, NaiveDate, f64), Error>>,
    debug: bool,
}

impl Portfolio {
    /// Creates a new Portfolio from a JSON configuration
    pub fn from_json(json: serde_json::Value) -> Self {
        let maps = json.as_object().unwrap();

        #[allow(clippy::mutable_key_type)]
        let mut portfolio = HashMap::default();

        for provider_key in maps.keys() {
            let Some(provider) = Provider::build(provider_key) else {
                eprintln!("Invalid provider: {}", provider_key);
                continue;
            };
            let provider = Arc::new(provider);

            let stocks_str = json.get(provider_key).expect("error parsing config key");
            let provider_stocks_dict: HashMap<String, u32> =
                serde_json::from_value(stocks_str.clone()).unwrap();
            for (instrument_name, quantity) in provider_stocks_dict {
                let instrument = Instrument {
                    name: instrument_name,
                    quantity,
                    provider: Arc::clone(&provider),
                };
                portfolio.insert(instrument, HashMap::default());
            }
        }
        Self {
            portfolio,
            price_cacher: Arc::new(PriceCacher::new()),
            request_join_handles: JoinSet::new(),
            debug: false,
        }
    }

    pub fn set_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Starts async jobs to fetch the prices for the portfolio on a certain date
    pub fn get_prices(&mut self, date: NaiveDate) {
        for instrument in self.portfolio.keys() {
            let m_price_cacher = Arc::clone(&self.price_cacher);
            let m_instrument = instrument.clone();
            self.request_join_handles
                .spawn(async move { m_price_cacher.download_price(m_instrument, date).await });
        }
    }

    /// Waits for the [get_prices](Self::get_prices) jobs to finish and updates the portfolio with the prices
    pub async fn wait_for_prices(&mut self) {
        while let Some(res) = self.request_join_handles.join_next().await {
            match res {
                Ok(Ok((instrument, date, price))) => {
                    let day_quotes = self.portfolio.get_mut(&instrument).unwrap();
                    day_quotes.insert(date, price);
                }
                Ok(Err(e)) if self.debug => {
                    println!("Error fetching price: {:?}", e)
                }
                Err(e) if self.debug => {
                    println!("Error fetching price: {:?}", e)
                }
                _ => {}
            }
        }
    }

    /// In case we are missing some prices, we can extend the known prices to dates that we don't have
    pub fn extend_dates(&mut self, start_date: NaiveDate, end_date: NaiveDate) {
        for value in self.portfolio.values_mut() {
            if value.is_empty() {
                // if it's empty, we can't extend it
                continue;
            }
            let min_date = *value.keys().min().unwrap();
            let min_date_price = value[&min_date];
            let mut last_price = min_date_price;

            // extend it to the left
            let mut current_date = start_date;
            while current_date < min_date {
                value.insert(current_date, min_date_price);
                current_date = current_date.checked_add_days(Days::new(1)).unwrap();
            }

            // extend it everywhere else with the last known price
            let mut current_date = start_date;
            while current_date <= end_date {
                let entry = value.entry(current_date);
                match entry {
                    Entry::Vacant(e) => {
                        e.insert(last_price);
                    }
                    Entry::Occupied(o) => last_price = *o.get(),
                }
                current_date = current_date.checked_add_days(Days::new(1)).unwrap();
            }
        }
    }

    /// Returns an iterator over the instruments and their values on a certain date
    pub fn instruments_and_values(
        &self,
        date: NaiveDate,
    ) -> impl Iterator<Item = (String, f64)> + '_ {
        self.portfolio
            .iter()
            .filter(move |(_, value)| value.contains_key(&date))
            .map(move |(instrument, value)| {
                (
                    instrument.name.clone(),
                    instrument.quantity as f64 * value[&date],
                )
            })
    }

    /// Returns the total value of the portfolio on a certain date
    pub fn portfolio_value(&self, date: NaiveDate) -> f64 {
        self.instruments_and_values(date)
            .map(|(_, value)| value)
            .reduce(|acc, p| acc + p)
            .unwrap_or_default()
    }
}

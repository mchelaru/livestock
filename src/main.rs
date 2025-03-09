use chrono::{Datelike, Days, Utc, Weekday};
use clap::Parser;
use providers::Providers;
use std::{
    collections::{HashMap, HashSet},
    fs,
    sync::Arc,
};
use textplots::{Chart, LabelBuilder, Plot, Shape};

mod price_cacher;
use price_cacher::PriceCacher;
mod provider;
mod providers;
mod xfra;
mod yfinance;

use chrono::NaiveDate;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The JSON configuration file
    #[arg(short, long)]
    file: String,

    /// The number of days to look back
    #[arg(long, default_value_t = 10)]
    days: usize,

    /// Displays additional debug information
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Extends the last known price in case no data exists
    #[arg(long, default_value_t = true)]
    extend_price: bool,

    /// display the daily portfolio value
    #[arg(long, default_value_t = false)]
    display_daily_value: bool,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args = Args::parse();

    // get the list of dates
    let today = Utc::now().naive_utc();
    let start_day = today.checked_sub_days(Days::new(args.days as u64)).unwrap();

    // read the symbol file
    let file = match fs::File::open(args.file.clone()) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Unable to open {}", args.file);
            return;
        }
    };
    let json: serde_json::Value = match serde_json::from_reader(file) {
        Ok(jv) => jv,
        Err(e) => {
            eprintln!("Unable to parse json in file {}. Error: {}", args.file, e);
            return;
        }
    };

    let maps = json.as_object().unwrap();

    let mut portfolio: HashMap<NaiveDate, HashMap<String, f64>> = HashMap::default();
    let mut quotes_join_handles = vec![];
    let mut stocks_dict: HashMap<String, u32> = HashMap::default();

    for provider_key in maps.keys() {
        let stocks_str = json.get(provider_key).expect("error parsing config key");
        let provider_stocks_dict: HashMap<String, u32> =
            serde_json::from_value(stocks_str.clone()).unwrap();
        stocks_dict.extend(provider_stocks_dict.iter().map(|(k, v)| (k.clone(), *v)));

        let price_cacher = Arc::new(PriceCacher::new());

        let provider = Providers::build(provider_key);
        if provider.is_none() {
            eprintln!("Invalid provider: {}", provider_key);
            continue;
        }
        let provider = Arc::new(provider.unwrap());
        println!("Querying {}...", provider.get_provider_name());

        let mut current_date = start_day;
        while current_date < today {
            if current_date.weekday() != Weekday::Sat && current_date.weekday() != Weekday::Sun {
                for ticker in provider_stocks_dict.keys() {
                    let mticker = ticker.clone(); // moved ticker
                    let price_cacher_ref = Arc::clone(&price_cacher);
                    let provider_ref = Arc::clone(&provider);
                    let date = current_date.into();
                    let jh = tokio::spawn(async move {
                        price_cacher_ref
                            .download_price(provider_ref, mticker, date)
                            .await
                    });
                    quotes_join_handles.push(jh);
                }
            }
            current_date = current_date.checked_add_days(Days::new(1)).unwrap();
        }
    }

    for j in quotes_join_handles {
        match j.await.unwrap() {
            Ok((ticker, date, price)) => {
                let quantity = stocks_dict.get(&ticker).unwrap();
                if args.debug {
                    println!(
                        "Quote at close for {ticker} on {date}: {price} * {} = {}",
                        *quantity,
                        price * (*quantity) as f64
                    );
                }
                let day_quotes = portfolio.entry(date).or_default();
                day_quotes.insert(ticker, price * (*quantity) as f64);
            }
            Err(e) => {
                if args.debug {
                    eprintln!("Error {e:#?}")
                }
            }
        }
    }

    let mut sorted_dates = portfolio.keys().copied().collect::<Vec<_>>();
    sorted_dates.sort();
    if args.extend_price && sorted_dates.len() > 1 {
        // right extend the prices in case they are not present for the latest day{s}
        // YF is well known for this "feature"
        let mut tickers = HashSet::new();
        for portfolio_date in portfolio.values() {
            portfolio_date.keys().for_each(|k| {
                tickers.insert(k.clone());
            });
        }
        for ticker in tickers {
            let mut last_price = 0.;
            for date in &sorted_dates[0..sorted_dates.len()] {
                let portfolio_date = portfolio.get_mut(date).unwrap();
                last_price = *portfolio_date.entry(ticker.clone()).or_insert(last_price);
            }
        }
    }

    if args.debug {
        println!("{:#?}", portfolio);
    }

    //
    // graph and print the total value
    //
    if args.days > 1 {
        let empty_day_dict = HashMap::default(); // in case we have missing days (e.g. weekends)
        println!("Portfolio evolution for the past {} days", args.days);
        Chart::new(150, 40, 0., portfolio.len() as f32 + 1.0)
            .x_label_format(textplots::LabelFormat::None)
            .lineplot(&Shape::Continuous(Box::new(|x| {
                portfolio
                    .get(
                        &start_day
                            .checked_add_days(Days::new(x.round() as u64))
                            .unwrap()
                            .date(),
                    )
                    .unwrap_or(&empty_day_dict)
                    .iter()
                    .map(|(_ticker, price)| *price)
                    .reduce(|acc, p| acc + p)
                    .unwrap_or_default() as f32
            })))
            .display();
    }

    // and finally prints the total portfolio value
    if args.display_daily_value {
        for date in &sorted_dates {
            println!(
                "Portfolio total value on {date}: {:.2}",
                portfolio[date]
                    .values()
                    .copied()
                    .reduce(|acc, p| acc + p)
                    .unwrap()
            );
        }
    } else {
        println!(
            "Portfolio total value: {:.2}",
            match sorted_dates.last() {
                Some(last_day) => portfolio[last_day]
                    .values()
                    .copied()
                    .reduce(|acc, p| acc + p)
                    .unwrap(),
                None => 0.,
            }
        );
    }
}

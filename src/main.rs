use chrono::{Datelike, Days, Utc, Weekday};
use clap::Parser;
use provider::Provider;
use std::fs;
use textplots::{Chart, LabelBuilder, Plot, Shape};

mod portfolio;
use portfolio::Portfolio;
mod price_cacher;
mod provider;

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
    let start_date = today.checked_sub_days(Days::new(args.days as u64)).unwrap();

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

    let mut portfolio = Portfolio::from_json(json).set_debug(args.debug);
    let mut current_date = start_date;
    let mut sorted_dates = vec![];
    while current_date < today {
        if current_date.weekday() != Weekday::Sat && current_date.weekday() != Weekday::Sun {
            portfolio.get_prices(current_date.into());
            sorted_dates.push(current_date);
        }
        current_date = current_date.checked_add_days(Days::new(1)).unwrap();
    }
    portfolio.wait_for_prices().await;

    // extend the portfolio to the last known price
    if args.extend_price {
        portfolio.extend_dates(start_date.into(), today.into());
    }

    if args.debug {
        for date in &sorted_dates {
            let mut portfolio_instruments = portfolio
                .instruments_and_values((*date).into())
                .collect::<Vec<_>>();
            portfolio_instruments.sort_by(|(a, _), (b, _)| a.cmp(b));
            println!("Portfolio on {date}");
            for (instrument_name, value) in portfolio_instruments {
                println!("  {instrument_name}: {value}");
            }
        }
    }

    //
    // graph and print the total value
    //
    if args.days > 1 {
        println!("Portfolio evolution for the past {} days", args.days);
        Chart::new(150, 40, 0., sorted_dates.len() as f32 + 1.0)
            .x_label_format(textplots::LabelFormat::None)
            .lineplot(&Shape::Continuous(Box::new(|x| {
                portfolio.portfolio_value(
                    start_date
                        .checked_add_days(Days::new(x.round() as u64))
                        .unwrap()
                        .into(),
                ) as f32
            })))
            .display();
    }

    // and finally prints the total portfolio value
    if args.display_daily_value {
        for date in &sorted_dates {
            println!(
                "Portfolio total value on {date}: {:.2}",
                portfolio.portfolio_value((*date).into())
            );
        }
    } else {
        println!(
            "Portfolio total value: {:.2}",
            match sorted_dates.last() {
                Some(last_day) => portfolio.portfolio_value((*last_day).into()),
                None => 0.,
            }
        );
    }
}

use clap::Parser;
use std::{collections::HashMap, fs};
use textplots::{Chart, LabelBuilder, Plot, Shape};
use tokio_test;
use yahoo_finance_api::{self as yf, Quote};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The JSON configuration file
    #[arg(short, long)]
    file: String,

    /// The number of days to look back
    #[arg(long, default_value_t = 10)]
    days: usize,

    /// Prints additional debug information
    #[arg(long, default_value_t = false)]
    debug: bool,
}

fn main() {
    let args = Args::parse();

    // read the symbols
    let file = fs::File::open(args.file).expect("file should open read only");
    let json: serde_json::Value =
        serde_json::from_reader(file).expect("file should be proper JSON");
    let stocks_str = json
        .get("Stocks")
        .expect("config file should have Stocks key");
    let stocks_dict: HashMap<String, u32> = serde_json::from_value(stocks_str.clone()).unwrap();

    let mut quotes: HashMap<String, Vec<Quote>> = HashMap::new();
    let provider = yf::YahooConnector::new().unwrap();

    println!("Querying Yahoo! Finance...");
    for (ticker, quantity) in &stocks_dict {
        // resolve the symbols
        let yahoo_symbol;
        match tokio_test::block_on(provider.search_ticker(&ticker)) {
            Ok(r) => {
                if r.quotes.len() < 1 {
                    eprintln!("Error matching symbol {ticker}");
                    continue;
                } else if r.quotes.len() > 1 && args.debug {
                    eprintln!("Multiple matches for {ticker} - using the first match");
                    println!(
                        "{}",
                        r.quotes
                            .iter()
                            .map(|q| q.symbol.clone())
                            .reduce(|mut acc, s| {
                                acc.push_str(" ");
                                acc.push_str(&s);
                                acc
                            })
                            .unwrap()
                    );
                }
                yahoo_symbol = r.quotes[0].symbol.clone();
            }
            Err(_) => {
                eprintln!("Error searching for symbol {ticker}");
                continue;
            }
        }

        let response = tokio_test::block_on(provider.get_quote_range(
            &yahoo_symbol,
            "1d",
            &format!("{}d", args.days),
        ));
        match response {
            Ok(response) => {
                let q = response.quotes().unwrap();
                quotes.insert(ticker.clone(), q);

                if args.debug {
                    let quote_at_close = response.last_quote().unwrap().close;
                    println!(
                        "Last quote at close for {yahoo_symbol}: {} * {} = {}",
                        quote_at_close,
                        *quantity,
                        quote_at_close * (*quantity) as f64
                    );
                }
            }
            Err(_) => {
                eprintln!("Error loading quotes for symbol {ticker}");
                continue;
            }
        }
    }

    // graph and print the total value
    let mut values = Vec::with_capacity(args.days);
    for day in 0..args.days {
        let mut value = 0.;
        for (ticker, quantity) in &stocks_dict {
            let ticker_quote = quotes.get(ticker);

            // Get the value at index from a vector or just return the last
            // possible value if the index is greater than the len
            // Returns None for empty vectors
            // implemented as a macro in order to quickly access ticker and args
            macro_rules! get_or_last {
                ($v: ident, $index: ident) => {
                    if $v.len() == 0 {
                        None
                    } else if $index < $v.len() {
                        Some($v[$index].clone())
                    } else {
                        if args.debug {
                            println!("Error accessing day {} for ticker {ticker}", $index);
                        }
                        Some($v[$v.len() - 1].clone())
                    }
                };
            }

            match ticker_quote {
                Some(q) => match get_or_last!(q, day) {
                    Some(quote) => value += quote.close * (*quantity) as f64,
                    None => println!("No quotes found for ticker {ticker}"),
                },
                None => {
                    eprintln!("No value found for {ticker}");
                }
            }
        }
        values.push(value);
    }
    if args.days > 1 {
        println!("Portfolio evolution on the last {} days", args.days);
        Chart::new(75, 20, 0., values.len() as f32 - 0.1)
            .x_label_format(textplots::LabelFormat::None)
            .lineplot(&Shape::Continuous(Box::new(|i| values[i as usize] as f32)))
            .display();
    }
    println!("Portfolio total value: {:.2}", values[values.len() - 1]);
}

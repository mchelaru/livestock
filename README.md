# LIVESTOCK

Livestock is a portfolio value tracker. Usage:

```
livestock [OPTIONS] --file <FILE>

Options:
  -f, --file <FILE>  The JSON configuration file
      --days <DAYS>  The number of days to look back [default: 10]
      --debug        Prints additional debug information
  -h, --help         Print help
  -V, --version      Print version
```

## Configuration file example

```
{
    "Stocks": {
        "AAPL": 50,
        "CSCO": 20,
        "SPY": 25,
        "TSLA": 10
    }
}
```

## Examples

Running it on the file above returns:

```
./livestock -f stocks.json --days 5
Querying Yahoo! Finance...
Portfolio evolution on the last 5 days
⡁                             ⢰⠉⠉⠉⠉⠉⠉⠁ 29853.2
⠍⠉⠉⠉⠉⠉⠉⠉⠒⠒⠒⠒⠒⠒⠒⡆      ⢰⠉⠉⠉⠉⠉          
⠂                ⡇      ⢸               
⡁                ⡇      ⢸               
⠄                ⡇      ⢸               
                 ⠈⠉⠉⠉⠉⠉⠉⠉              29249.8


Portfolio total value: 29853.20
```

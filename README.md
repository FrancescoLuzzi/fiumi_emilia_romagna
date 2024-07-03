# Allerta Meteo

## API calls

Get all stations and levels:

`https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-sensor-values?variabile=254,0,0/1,-,-,-/B13215&time=1719649800000`

```json
...
{
    "idstazione": "-/987258,4485120/simnpr",
    "ordinamento": 9999,
    "nomestaz": "Castell'Arquato Canale",
    "lon": "987258",
    "soglia1": 0,
    "value": 0.06,
    "soglia2": 0,
    "lat": "4485120",
    "soglia3": 0
},
...
```
for the DB we could drop value and ordinamento

Get station time series:

`https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-time-series/?stazione=-/1129579,4472121/simnbo&variabile=254,0,0/1,-,-,-/B13215`

```json
[{"t":1719439200000,"v":3.58},...]
```

timestamps are unix timestamp in milliseconds, by dropping the last 3 digits we can read it as an u32
value is a f32

~27.5GB of data per year (he thick)


# TODO

Use [QuestDB](https://questdb.io/download/) timeseries DB with a background worker that every 60 minutes scraps all the stations for their infos
Then the TUI can have a 2 mode logic:

- load data from the internet in real time
- load data from stored data in DB


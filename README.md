# fit2json command line fit converter

This command line tool provides a high performance tool to read Garmin FIT files and convert them to JSON.
The tool provides the following features:

- Total extraction of all data split up into multiple json files for easier digestion
- Extraction of basic data and statistics such as number of entries of each record type, user data, workout type, length, overall performance metrics
- Extraction of specific info
  - Generic queries and overall info - useful for creating summary SQL tables
    - overall summary info:
      - user
      - start and end timestamps, 
      - GPS box
      - sport type
      - overall performance statistics
  - Generic queries for any type of FitDataRecord allow filtering by certain fields or timestamp ranges (if applicable)
    - Individual Record types query
    - Overall data per activity/ session/ lap, session, or activity

  - Queries for large volume data records (allow filtering by timestamp, lap, )
    - Record-related queries
    - GPS-Metadta related queries
    - Events
  
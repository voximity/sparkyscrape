# sparkyscrape

sparkyscrape will learn about Geometry Dash levels posted by the Sparky bot (in
the Geometry Dash Discord server), build a database, and recognize those levels
in the future.

<!-- prettier-ignore -->
> [!warning]
> Using this program is against the rules of the Geometry Dash
> Discord. The code is posted for educational purposes.

## Components

- A Rust program that will connect to a Discord account, watch for Sparky games
  being played, and simultaneously build a database and make guesses
- A React web app that connects to the Rust program with real-time feedback on
  guesses made, levels learned, and guess performance
- There is confetti when sparkyscrape gets a level right

## How it works

The database is a small binary format that stores the names of each level and
three arrays (one for each color channel) of DCT coefficients for a known level
image. The guessing algorithm computes the DCT coefficient of the source image
and compares it against the database in parallel through weighted Euclidean
distance. When someone correctly guesses the level, it is able to validate
whether or not its guess was correct; if incorrect, then the new level is added
to the database.

## Usage

Probably don't use this project. The code is available for you to poke around,
though. If you can figure out how to use it, have fun and good luck, I guess?

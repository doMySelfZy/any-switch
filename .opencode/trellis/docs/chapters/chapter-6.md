# Chapter 6: Every Outcome Has a Contract

Every outcome produces something and consumes something. Mapping these contracts before building eliminates the "two architects, one door" problem — two outcomes making conflicting assumptions about shared data.

Think of a contract as a handshake between outcomes. One outcome creates a booking record. Another outcome displays booking details. If the first outcome does not produce the information the second outcome needs, the system breaks — not because of a code error, but because the specification had a gap.

## Key Principles

- **Every outcome has a contract.** It consumes data from somewhere and produces data for something else. Making this explicit before the build starts means dependencies are visible, not hidden.

- **The contract map is the system's skeleton.** When you draw out which outcomes produce which contracts and which outcomes consume them, you see the structure of your system. You see which outcomes must be built first. You see where two outcomes might conflict. You see what is missing.

- **Contract conflicts are specification problems, not technical problems.** If two outcomes assume different shapes for the same data — one expects a booking to include a seat number, the other does not — that is a specification gap. Resolve it before building, not during debugging.

- **Contracts are described in domain language.** A contract is not a database schema. It is "a booking record containing the customer name, event date, number of tickets, and total price." Describe what information flows between outcomes in the language you use to talk about your business.

## Red Flags

- An outcome that consumes nothing. Where does the data come from? If an outcome displays booking details but no other outcome produces booking details, something is missing from your specification.

- An outcome that produces nothing. Where does the data go? If an outcome collects customer feedback but nothing else in the system uses that feedback, either the feedback outcome is unnecessary or you are missing a downstream outcome.

- Two outcomes producing the same contract with different assumptions. If "create booking" and "import bookings" both produce booking records but assume different fields, the consuming outcomes cannot rely on either. Align the contracts.

- Contracts described in technical terms. If your contract mentions "JSON payload," "foreign key," or "API response," rewrite it. Describe the information in domain language: what does this data represent to the people who use your system?

## What This Means for You

For each outcome in your current specification, write down: what data does this outcome need to exist before it runs? What data does this outcome create or change? Then draw the connections. Where one outcome's produced contract matches another outcome's consumed contract, you have a dependency. Where there is a mismatch, you have a specification gap to resolve.

Next: Chapter 7 explains why writing an outcome once is thinking, and writing it twice is specifying.

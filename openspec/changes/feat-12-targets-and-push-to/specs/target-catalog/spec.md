## Purpose

The deep-sky objects an observer can choose as a target, bundled in the console so it works
offline, and the search that finds one by Messier number, NGC or IC id, or common name.

## ADDED Requirements

### Requirement: All of OpenNGC, offline

The system SHALL bundle every object in OpenNGC's NGC, IC and addendum lists except those typed
`NonEx`, each with its id, Messier number, common names, type, J2000 position, axes, position
angle, magnitude and constellation. A row typed `Dup` SHALL NOT be an object of its own: its id
SHALL find the object it duplicates.
**Refs**: OpenNGC (Mattia Verga, CC BY-SA 4.0) at a pinned commit; GLOSSARY.md (target)

#### Scenario: The object count
- **WHEN** the bundled catalog is counted
- **THEN** it holds exactly the rows of OpenNGC's `NGC.csv` and `addendum.csv` at the pinned
  commit less the `NonEx` and `Dup` rows, the count recorded with the extract

#### Scenario: A duplicate's id
- **WHEN** each id that OpenNGC marks `Dup` is looked up
- **THEN** each finds the object OpenNGC names as the one it duplicates, and none is listed as
  an object of its own

#### Scenario: Offline
- **WHEN** a search runs on a machine with no network connection
- **THEN** it answers from the bundled catalog, exactly as it does online

### Requirement: The Messier objects agree

The catalog's Messier objects SHALL be the finder console's 110 Messier objects: each Messier
number SHALL find one object, at the Messier catalog's J2000 position, with M102 as NGC 5866,
M76 as NGC 650 and M45 as Mel 22.
**Refs**: finder-console "The Messier catalog"; the Messier extract's notes (M102, M76, M45)

#### Scenario: All 110
- **WHEN** M1 to M110 are each looked up in the catalog
- **THEN** each is found, and each lies within 1″ of the same object in the Messier catalog

#### Scenario: The disputed and doubled ones
- **WHEN** M102, M76 and M45 are looked up
- **THEN** they are NGC 5866, NGC 650 and Mel 22, and `M111` is not found

### Requirement: Exact ids

The system SHALL find an object by an exact id: a Messier number or a catalogue designation
(NGC, IC or an addendum catalogue), in any case, with or without a space before the number, and
with or without leading zeros. Each object SHALL have one canonical id, its designation with a
space and no leading zeros (`NGC 224`), or its Messier number when it has no other.
**Refs**: OpenNGC naming (`NGC0224`, `IC0434`, `Mel022`); finder-console "The Messier catalog"

#### Scenario: Ways to write the same object
- **WHEN** `m31`, `M 31`, `ngc224`, `NGC 0224` and `ngc0224` are looked up
- **THEN** each finds the one object whose canonical id is `NGC 224`

#### Scenario: An id that is not there
- **WHEN** `NGC 99999` or `M111` is looked up
- **THEN** nothing is found

### Requirement: Search

The system SHALL answer `GET /api/targets?q=` with the objects the query names: exact ids first,
then objects with a common-name word the query begins (any case), brightest first, objects with
no magnitude last. `limit` SHALL default to 20 and be clamped to 1–50; a missing or empty `q`
SHALL answer 400 with a JSON `error`.
**Refs**: GLOSSARY.md (target); OpenNGC common names

#### Scenario: Three ways to Andromeda
- **WHEN** `m31`, `ngc0224` and `andromeda` are each searched
- **THEN** each list starts with NGC 224 (M31)

#### Scenario: Nothing by that name
- **WHEN** `qqq` is searched
- **THEN** the answer is 200 with an empty list

#### Scenario: No query
- **WHEN** `/api/targets` is requested with no `q`, or with `q=` empty
- **THEN** the answer is 400 with a JSON `error`

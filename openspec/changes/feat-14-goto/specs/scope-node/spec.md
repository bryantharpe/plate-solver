## ADDED Requirements

### Requirement: The mount port

The scope node SHALL drive a mount only through a mount port of two axes, each of which reports
its axis position and capabilities (counts per turn, top rate, whether it can go to a position),
goes to a position, moves at a signed rate and stops. Every mount SHALL be reached through an
adapter behind this port.
**Refs**: ADR-0001; GLOSSARY.md (mount port, mount adapter, axis position)

#### Scenario: The simulator's mount
- **WHEN** the simulated mount's axes are asked for their capabilities, sent to positions, moved
  at rates and stopped through the port
- **THEN** each reports its counts per turn and top rate, reaches each position within one
  count, holds each rate within 0.1%, and stops

### Requirement: The Sky-Watcher adapter

The Sky-Watcher adapter SHALL speak the motor-controller protocol: `:`, command, axis, data, CR;
24-bit values low byte first, positions offset by 0x800000. It SHALL probe 9600 then 115200 baud
with `:e1`, initialise with `:F`, read `:e`, `:a`, `:b` and `:g`, switch the auxiliary encoders
off with `:W`, and stop an axis before reversing it.
**Refs**: Sky-Watcher motor controller command set; docs/research/2026-10-21-synscan-mount-control.md §3, §5

#### Scenario: Positions on the wire
- **WHEN** positions 0, +1 and −1 are encoded
- **THEN** they are `000080`, `010080` and `FFFF7F`, and each decodes back to itself

#### Scenario: A goto on the wire
- **WHEN** axis 1 is sent to position +0x123 against the byte-level emulator
- **THEN** the emulator receives `:S1230180\r` among the goto's commands, and the axis reaches
  +0x123

#### Scenario: Motor not stopped
- **WHEN** the emulator answers a command with `!2`
- **THEN** the adapter stops that axis, waits until `:f` reports it stopped, and sends the
  command again once

#### Scenario: Reversing
- **WHEN** an axis moving at +1°/s is asked to move at −1°/s
- **THEN** the emulator sees it stopped and reported stopped before the new direction is set

#### Scenario: The baud rate
- **WHEN** the emulator answers only at 115200 baud
- **THEN** the adapter finds it there after no answer at 9600, and reports the board's version

### Requirement: Stop wins, and a watchdog

A stop SHALL be carried out before any command that arrived with or after it, and SHALL cancel
any move in progress. The scope node SHALL stop both axes when 1 s has passed without a heartbeat
from the console.
**Refs**: scope-link "Clock mapping", "Link health"; GLOSSARY.md (scope node)

#### Scenario: Silence
- **WHEN** the console's heartbeats stop while both axes move, in virtual time
- **THEN** both axes are commanded to stop 1.0 ± 0.1 s after the last heartbeat

#### Scenario: Stop among moves
- **WHEN** a stop and a goto reach the node together
- **THEN** the axes stop and the goto is not carried out

### Requirement: Axis positions on star frames

While a mount is connected, each star frame SHALL carry both axes' positions read at the start
and at the end of its exposure, on the node's clock.
**Refs**: GLOSSARY.md (sync, axis position); pointing-model "Every solve is a sync"

#### Scenario: Positions bracket the exposure
- **WHEN** the simulated azimuth axis turns at a steady rate during a 200 ms exposure
- **THEN** the frame's start and end positions differ by that rate times the exposure, within
  two counts

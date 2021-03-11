# Code Structure

The entire application is running on several async threads in a tokio runtime:

- [Mail](#mail)
- [UI](#ui)
- [VM](#vm)

Each of these communicates with the others via pairs of unbounded async
channels.

## Mail

The mail thread is in charge of communicating with mail servers. It keeps a
single connection alive to each server even if the UI thread has multiple mail
views open.

## UI

The UI thread manages everything user-facing. It runs a terminal UI using the
tui crate. There's a tiny windowing system built in that allows for tiling
windows, split horizontally or vertically.

## VM

The VM runs the scripting language that can be used inside the application.
